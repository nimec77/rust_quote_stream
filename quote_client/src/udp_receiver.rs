use std::net::{SocketAddr, UdpSocket};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::thread;
use std::time::Duration;

use log::{debug, info, warn};

use quote_common::{
    BUFFER_SIZE, PING_INTERVAL_SECS, PING_PAYLOAD, QuoteError, StockQuote, UNKNOWN_ADDR_PLACEHOLDER,
};

// Constants replacing magic numbers/words in this module
const UDP_READ_TIMEOUT_MS: u64 = 200;
const PING_LOOP_SLEEP_MS: u64 = 100;
const WOULD_BLOCK_BACKOFF_MS: u64 = 50;
const UDP_RECV_ERROR_BACKOFF_MS: u64 = 100;
const UDP_LISTENER_THREAD_NAME: &str = "udp-listener";
const UDP_PING_THREAD_NAME: &str = "udp-ping";

/// Spawn a thread that listens for UDP quotes until shutdown is signalled.
pub fn spawn_listener(
    socket: UdpSocket,
    shutdown: Arc<AtomicBool>,
) -> Result<thread::JoinHandle<()>, QuoteError> {
    socket.set_nonblocking(true).map_err(|err| {
        quote_common::quote_error!(
            NetworkError,
            "failed to set UDP socket nonblocking: {}",
            err
        )
    })?;
    socket
        .set_read_timeout(Some(Duration::from_millis(UDP_READ_TIMEOUT_MS)))
        .map_err(|err| {
            quote_common::quote_error!(NetworkError, "failed to set UDP read timeout: {}", err)
        })?;

    let handle = thread::Builder::new()
        .name(UDP_LISTENER_THREAD_NAME.to_string())
        .spawn(move || listen_loop(socket, shutdown))
        .map_err(|err| {
            quote_common::quote_error!(NetworkError, "failed to spawn UDP listener: {}", err)
        })?;

    Ok(handle)
}

/// Spawn a thread that sends PING messages to the server at regular intervals.
pub fn spawn_ping_thread(
    socket: UdpSocket,
    server_addr: SocketAddr,
    shutdown: Arc<AtomicBool>,
) -> Result<thread::JoinHandle<()>, QuoteError> {
    let handle = thread::Builder::new()
        .name(UDP_PING_THREAD_NAME.to_string())
        .spawn(move || ping_loop(socket, server_addr, shutdown))
        .map_err(|err| {
            quote_common::quote_error!(NetworkError, "failed to spawn ping thread: {}", err)
        })?;

    Ok(handle)
}

fn ping_loop(socket: UdpSocket, server_addr: SocketAddr, shutdown: Arc<AtomicBool>) {
    let ping_interval = Duration::from_secs(PING_INTERVAL_SECS);
    debug!(
        "Starting ping thread, sending PING every {:?} to {}",
        ping_interval, server_addr
    );

    while !shutdown.load(Ordering::SeqCst) {
        if let Err(err) = socket.send_to(PING_PAYLOAD, server_addr) {
            warn!("Failed to send PING to {}: {}", server_addr, err);
        } else {
            debug!("Sent PING to {}", server_addr);
        }

        // Sleep for ping interval, but check shutdown flag periodically
        let sleep_duration = Duration::from_millis(PING_LOOP_SLEEP_MS);
        let mut elapsed = Duration::ZERO;
        while elapsed < ping_interval && !shutdown.load(Ordering::SeqCst) {
            thread::sleep(sleep_duration);
            elapsed += sleep_duration;
        }
    }

    debug!("Ping thread shutting down");
}

fn listen_loop(socket: UdpSocket, shutdown: Arc<AtomicBool>) {
    let mut buffer = [0u8; BUFFER_SIZE];
    info!(
        "Listening for UDP quotes on {}",
        socket
            .local_addr()
            .map(|addr| addr.to_string())
            .unwrap_or_else(|_| UNKNOWN_ADDR_PLACEHOLDER.into())
    );

    while !shutdown.load(Ordering::SeqCst) {
        match socket.recv(&mut buffer) {
            Ok(size) => {
                if let Err(err) = handle_payload(&buffer[..size]) {
                    warn!("{err}");
                }
            }
            Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(WOULD_BLOCK_BACKOFF_MS));
            }
            Err(err) if err.kind() == std::io::ErrorKind::TimedOut => {}
            Err(err) => {
                warn!("UDP receive error: {}", err);
                thread::sleep(Duration::from_millis(UDP_RECV_ERROR_BACKOFF_MS));
            }
        }
    }

    debug!("UDP listener shutting down");
}

fn handle_payload(payload: &[u8]) -> Result<(), String> {
    let quote: StockQuote = serde_json::from_slice(payload)
        .map_err(|err| format!("Failed to parse quote JSON: {err}"))?;
    info!(
        "Quote [{}] price=${:.2} volume={} ts={}",
        quote.ticker, quote.price, quote.volume, quote.timestamp
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_handle_payload_logs_valid_quote() {
        let quote = StockQuote::new("AAPL", 150.12, 1_000);
        let payload = serde_json::to_vec(&quote).expect("serialize");
        assert!(handle_payload(&payload).is_ok());
    }

    #[test]
    fn test_handle_payload_rejects_invalid_json() {
        let err = handle_payload(br#"{"ticker": 123}"#).expect_err("should fail");
        assert!(err.contains("Failed to parse quote JSON"));
    }

    #[test]
    fn test_ping_thread_sends_ping() {
        let listener = UdpSocket::bind("127.0.0.1:0").expect("bind listener");
        listener
            .set_read_timeout(Some(Duration::from_secs(3)))
            .expect("set timeout");
        let server_addr = listener.local_addr().expect("local addr");

        let ping_socket = UdpSocket::bind("127.0.0.1:0").expect("bind ping socket");
        let shutdown = Arc::new(AtomicBool::new(false));
        let ping_handle = spawn_ping_thread(ping_socket, server_addr, Arc::clone(&shutdown))
            .expect("spawn ping thread");

        // Wait for at least one ping
        let mut buffer = [0u8; 16];
        let (size, _) = listener.recv_from(&mut buffer).expect("receive ping");
        assert_eq!(&buffer[..size], b"PING");

        shutdown.store(true, Ordering::SeqCst);
        ping_handle.join().expect("join ping thread");
    }

    #[test]
    fn test_ping_thread_respects_shutdown() {
        let ping_socket = UdpSocket::bind("127.0.0.1:0").expect("bind ping socket");
        let server_addr: SocketAddr = "127.0.0.1:9999".parse().expect("parse addr");
        let shutdown = Arc::new(AtomicBool::new(true)); // Set shutdown immediately
        let ping_handle =
            spawn_ping_thread(ping_socket, server_addr, shutdown).expect("spawn ping thread");

        // Thread should exit quickly since shutdown is already set
        ping_handle.join().expect("join ping thread");
    }
}
