use std::collections::{HashMap, HashSet};
use std::net::UdpSocket;
use std::thread;

use crossbeam::channel::{self, Receiver, Sender};
use log::{info, warn};
use serde_json::to_vec;

use quote_common::{QuoteError, StockQuote};

use crate::tcp_handler::StreamRequest;

/// Commands sent to the UDP dispatcher.
#[derive(Debug)]
pub enum UdpCommand {
    AddClient(StreamRequest),
    Shutdown,
}

struct ClientChannels {
    tickers: HashSet<String>,
    sender: Sender<StockQuote>,
    handle: thread::JoinHandle<()>,
}

/// Start a UDP dispatcher that distributes quotes to client threads.
pub fn start_udp_streamer(
    quote_rx: Receiver<StockQuote>,
) -> Result<(Sender<UdpCommand>, thread::JoinHandle<()>), QuoteError> {
    let (command_tx, command_rx) = channel::unbounded::<UdpCommand>();

    let handle = thread::Builder::new()
        .name("udp-dispatcher".to_string())
        .spawn(move || dispatcher_loop(quote_rx, command_rx))
        .map_err(QuoteError::from)?;

    Ok((command_tx, handle))
}

fn dispatcher_loop(quote_rx: Receiver<StockQuote>, command_rx: Receiver<UdpCommand>) {
    let mut clients: HashMap<usize, ClientChannels> = HashMap::new();
    let mut next_id: usize = 0;

    loop {
        crossbeam::channel::select! {
            recv(command_rx) -> command => match command {
                Ok(UdpCommand::AddClient(request)) => {
                    if let Err(err) = register_client(&mut clients, &mut next_id, request) {
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
    }

    shutdown_clients(clients);
    info!("UDP dispatcher shutting down");
}

fn register_client(
    clients: &mut HashMap<usize, ClientChannels>,
    next_id: &mut usize,
    request: StreamRequest,
) -> Result<(), QuoteError> {
    let tickers = request.tickers.iter().cloned().collect::<HashSet<_>>();

    let (quote_tx, quote_rx) = channel::unbounded::<StockQuote>();
    let client_id = *next_id;

    let request_for_thread = request.clone();

    let handle = thread::Builder::new()
        .name(format!("udp-client-{client_id}"))
        .spawn(move || client_loop(request_for_thread, quote_rx))
        .map_err(QuoteError::from)?;

    clients.insert(
        client_id,
        ClientChannels {
            tickers,
            sender: quote_tx,
            handle,
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

fn client_loop(request: StreamRequest, quote_rx: Receiver<StockQuote>) {
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

    for quote in quote_rx.iter() {
        match to_vec(&quote) {
            Ok(payload) => {
                if let Err(err) = socket.send_to(&payload, request.udp_addr) {
                    warn!("Failed to send UDP packet to {}: {}", request.udp_addr, err);
                }
            }
            Err(err) => {
                warn!("Failed to serialize quote for {}: {}", quote.ticker, err);
            }
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::UdpSocket;
    use std::time::Duration;

    use quote_common::StockQuote;

    #[test]
    fn test_client_receives_filtered_quotes() {
        let (quote_tx, quote_rx) = channel::unbounded::<StockQuote>();
        let (manager_tx, manager_handle) = start_udp_streamer(quote_rx).expect("start manager");

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
}
