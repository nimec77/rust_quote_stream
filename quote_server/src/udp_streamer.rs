use std::collections::{HashMap, HashSet};
use std::net::UdpSocket;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use crossbeam::channel::{self, Receiver, RecvTimeoutError, Sender};
use log::{debug, info, warn};
use serde_json::to_vec;

use quote_common::{DEFAULT_KEEPALIVE_TIMEOUT_SECS, QuoteError, StockQuote};

use crate::tcp_handler::StreamRequest;

/// Commands sent to the UDP dispatcher.
#[derive(Debug)]
pub enum UdpCommand {
    /// Add a new client to receive filtered quotes.
    AddClient(StreamRequest),
    /// Shutdown the dispatcher and terminate all client threads.
    Shutdown,
}

struct ClientChannels {
    tickers: HashSet<String>,
    sender: Sender<StockQuote>,
    handle: thread::JoinHandle<()>,
    last_ping: Arc<Mutex<Instant>>,
    timeout: Duration,
    udp_addr: std::net::SocketAddr,
}

/// Start a UDP dispatcher that distributes quotes to client threads.
pub fn start_udp_streamer(
    quote_rx: Receiver<StockQuote>,
    keepalive_timeout: Duration,
    server_udp_addr: std::net::SocketAddr,
) -> Result<(Sender<UdpCommand>, thread::JoinHandle<()>), QuoteError> {
    let (command_tx, command_rx) = channel::unbounded::<UdpCommand>();

    let handle = thread::Builder::new()
        .name("udp-dispatcher".to_string())
        .spawn(move || dispatcher_loop(quote_rx, command_rx, keepalive_timeout, server_udp_addr))
        .map_err(|err| {
            quote_common::quote_error!(IoError, err, "failed to spawn UDP dispatcher thread")
        })?;

    Ok((command_tx, handle))
}

fn dispatcher_loop(
    quote_rx: Receiver<StockQuote>,
    command_rx: Receiver<UdpCommand>,
    keepalive_timeout: Duration,
    server_udp_addr: std::net::SocketAddr,
) {
    let mut clients: HashMap<usize, ClientChannels> = HashMap::new();
    let mut next_id: usize = 0;

    // Create shared PING socket bound to server's UDP port
    let ping_socket = match UdpSocket::bind(server_udp_addr) {
        Ok(socket) => {
            if let Err(err) = socket.set_nonblocking(true) {
                warn!("Failed to set PING socket non-blocking: {}", err);
            }
            socket
        }
        Err(err) => {
            warn!("Failed to bind PING socket on {}: {}", server_udp_addr, err);
            return;
        }
    };

    let mut ping_buffer = [0u8; 16];

    loop {
        crossbeam::channel::select! {
            recv(command_rx) -> command => match command {
                Ok(UdpCommand::AddClient(request)) => {
                    if let Err(err) = register_client(&mut clients, &mut next_id, request, keepalive_timeout, server_udp_addr) {
                        warn!("Failed to register UDP client: {err}");
                    }
                }
                Ok(UdpCommand::Shutdown) | Err(_) => break,
            },
            recv(quote_rx) -> message => match message {
                Ok(quote) => {
                    deliver_quote(&mut clients, &quote);
                }
                Err(_) => break,
            }
        }

        // Check for PINGs on shared socket
        match ping_socket.recv_from(&mut ping_buffer) {
            Ok((size, from_addr)) => {
                if &ping_buffer[..size] == b"PING" {
                    // Find client by UDP address and update last_ping
                    for client in clients.values() {
                        if client.udp_addr == from_addr {
                            let now = Instant::now();
                            if let Ok(mut guard) = client.last_ping.lock() {
                                *guard = now;
                            }
                            debug!("PING received from {}", from_addr);
                            break;
                        }
                    }
                }
            }
            Err(err)
                if err.kind() == std::io::ErrorKind::WouldBlock
                    || err.kind() == std::io::ErrorKind::TimedOut => {}
            Err(err) => {
                warn!("PING socket recv error: {}", err);
            }
        }

        purge_expired_clients(&mut clients);
    }

    shutdown_clients(clients);
    info!("UDP dispatcher shutting down");
}

fn register_client(
    clients: &mut HashMap<usize, ClientChannels>,
    next_id: &mut usize,
    request: StreamRequest,
    keepalive_timeout: Duration,
    server_udp_addr: std::net::SocketAddr,
) -> Result<(), QuoteError> {
    let tickers = request.tickers.iter().cloned().collect::<HashSet<_>>();

    let (quote_tx, quote_rx) = channel::unbounded::<StockQuote>();
    let client_id = *next_id;

    let request_for_thread = request.clone();
    let last_ping = Arc::new(Mutex::new(Instant::now()));
    let last_ping_for_thread = Arc::clone(&last_ping);

    let handle = thread::Builder::new()
        .name(format!("udp-client-{client_id}"))
        .spawn(move || {
            client_loop(
                request_for_thread,
                quote_rx,
                keepalive_timeout,
                last_ping_for_thread,
                server_udp_addr,
            )
        })
        .map_err(|err| {
            quote_common::quote_error!(
                IoError,
                err,
                "failed to spawn UDP client thread {}",
                client_id
            )
        })?;

    clients.insert(
        client_id,
        ClientChannels {
            tickers,
            sender: quote_tx,
            handle,
            last_ping,
            timeout: keepalive_timeout,
            udp_addr: request.udp_addr,
        },
    );

    *next_id += 1;

    info!(
        "Registered UDP client {} for [{}] at {}",
        client_id,
        request.tickers.join(","),
        request.udp_addr
    );

    Ok(())
}

fn deliver_quote(clients: &mut HashMap<usize, ClientChannels>, quote: &StockQuote) {
    let mut stale_clients = Vec::new();
    for (client_id, client) in clients.iter() {
        if client.tickers.contains(&quote.ticker) && client.sender.send(quote.clone()).is_err() {
            stale_clients.push(*client_id);
        }
    }

    for client_id in stale_clients {
        if let Some(client) = clients.remove(&client_id) {
            match client.handle.join() {
                Ok(_) => {}
                Err(err) => warn!("Client thread {client_id} panicked: {err:?}"),
            }
        }
    }
}

fn client_loop(
    request: StreamRequest,
    quote_rx: Receiver<StockQuote>,
    keepalive_timeout: Duration,
    last_ping: Arc<Mutex<Instant>>,
    _server_udp_addr: std::net::SocketAddr,
) {
    // Bind to ephemeral port for sending quotes
    let socket = match UdpSocket::bind("0.0.0.0:0") {
        Ok(socket) => socket,
        Err(err) => {
            warn!(
                "Failed to bind UDP socket for {}: {}",
                request.udp_addr, err
            );
            return;
        }
    };

    if let Err(err) = socket.connect(request.udp_addr) {
        warn!(
            "Failed to connect UDP socket to {}: {}",
            request.udp_addr, err
        );
        return;
    }

    loop {
        // Check timeout based on last_ping (updated by dispatcher)
        let elapsed = last_ping
            .lock()
            .map(|instant| instant.elapsed())
            .unwrap_or_else(|_| Duration::from_secs(keepalive_timeout.as_secs() + 1));

        if elapsed > keepalive_timeout {
            warn!(
                "Client {} exceeded keepalive timeout of {:?}",
                request.udp_addr, keepalive_timeout
            );
            break;
        }

        match quote_rx.recv_timeout(Duration::from_millis(100)) {
            Ok(quote) => match to_vec(&quote) {
                Ok(payload) => {
                    if let Err(err) = socket.send(&payload) {
                        warn!("Failed to send UDP packet to {}: {}", request.udp_addr, err);
                    }
                }
                Err(err) => {
                    warn!("Failed to serialize quote for {}: {}", quote.ticker, err);
                }
            },
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => break,
        }
    }
}

fn shutdown_clients(mut clients: HashMap<usize, ClientChannels>) {
    for (client_id, client) in clients.drain() {
        drop(client.sender);
        match client.handle.join() {
            Ok(_) => {}
            Err(err) => warn!("Client thread {client_id} panicked during shutdown: {err:?}"),
        }
    }
}

fn purge_expired_clients(clients: &mut HashMap<usize, ClientChannels>) {
    let mut expired = Vec::new();
    for (client_id, client) in clients.iter() {
        let elapsed = client
            .last_ping
            .lock()
            .map(|instant| instant.elapsed())
            .unwrap_or_else(|_| Duration::from_secs(DEFAULT_KEEPALIVE_TIMEOUT_SECS * 2));
        if elapsed > client.timeout {
            expired.push(*client_id);
        }
    }

    for client_id in expired {
        if let Some(client) = clients.remove(&client_id) {
            warn!("Client {} timed out after {:?}", client_id, client.timeout);
            drop(client.sender);
            if let Err(err) = client.handle.join() {
                warn!("Client thread {client_id} panicked during timeout cleanup: {err:?}");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::UdpSocket;
    use std::time::Duration;

    use quote_common::StockQuote;

    #[test]
    fn test_client_receives_filtered_quotes() {
        let (quote_tx, quote_rx) = channel::unbounded::<StockQuote>();
        let server_addr: std::net::SocketAddr = "127.0.0.1:0".parse().expect("parse addr");
        let (manager_tx, manager_handle) = start_udp_streamer(
            quote_rx,
            Duration::from_secs(DEFAULT_KEEPALIVE_TIMEOUT_SECS),
            server_addr,
        )
        .expect("start manager");

        let listener = UdpSocket::bind("127.0.0.1:0").expect("bind listener");
        listener
            .set_read_timeout(Some(Duration::from_millis(500)))
            .expect("set timeout");
        let addr = listener.local_addr().expect("local addr");

        let request = StreamRequest {
            udp_addr: addr,
            tickers: vec!["AAPL".to_string()],
        };

        manager_tx
            .send(UdpCommand::AddClient(request))
            .expect("add client");

        // Allow client thread to bind before sending quotes.
        std::thread::sleep(Duration::from_millis(50));

        let quote_in = StockQuote::new("AAPL", 150.0, 1_000);
        let quote_filtered = StockQuote::new("MSFT", 200.0, 2_000);

        quote_tx.send(quote_filtered).expect("send filtered");
        quote_tx
            .send(quote_in.clone())
            .expect("send matching quote");

        let mut buffer = [0u8; 1024];
        let (size, _) = listener.recv_from(&mut buffer).expect("receive quote");

        let received: StockQuote =
            serde_json::from_slice(&buffer[..size]).expect("deserialize quote");

        assert_eq!(received.ticker, quote_in.ticker);

        manager_tx
            .send(UdpCommand::Shutdown)
            .expect("shutdown manager");
        drop(quote_tx);

        manager_handle.join().expect("join manager");
    }

    #[test]
    fn test_client_times_out_without_ping() {
        let (quote_tx, quote_rx) = channel::unbounded::<StockQuote>();
        let timeout = Duration::from_millis(50);
        let server_addr: std::net::SocketAddr = "127.0.0.1:0".parse().expect("parse addr");
        let (manager_tx, manager_handle) =
            start_udp_streamer(quote_rx, timeout, server_addr).expect("start manager");

        let listener = UdpSocket::bind("127.0.0.1:0").expect("bind listener");
        listener
            .set_read_timeout(Some(Duration::from_millis(200)))
            .expect("set timeout");
        let addr = listener.local_addr().expect("local addr");

        let request = StreamRequest {
            udp_addr: addr,
            tickers: vec!["AAPL".to_string()],
        };

        manager_tx
            .send(UdpCommand::AddClient(request))
            .expect("add client");

        std::thread::sleep(Duration::from_millis(120));

        let quote = StockQuote::new("AAPL", 123.0, 1_000);
        quote_tx.send(quote).expect("send quote");

        let mut buffer = [0u8; 1024];
        let recv_result = listener.recv_from(&mut buffer);
        assert!(
            recv_result.is_err(),
            "Client should have timed out before quote arrived"
        );

        manager_tx
            .send(UdpCommand::Shutdown)
            .expect("shutdown manager");
        drop(quote_tx);

        manager_handle.join().expect("join manager");
    }
}
