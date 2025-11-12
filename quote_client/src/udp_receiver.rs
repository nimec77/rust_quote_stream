use std::net::UdpSocket;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::thread;
use std::time::Duration;

use log::{debug, info, warn};

use quote_common::{QuoteError, StockQuote};

/// Spawn a thread that listens for UDP quotes until shutdown is signalled.
pub fn spawn_listener(
    socket: UdpSocket,
    shutdown: Arc<AtomicBool>,
) -> Result<thread::JoinHandle<()>, QuoteError> {
    socket.set_nonblocking(true).map_err(|err| {
        QuoteError::NetworkError(format!("failed to set UDP socket nonblocking: {err}"))
    })?;
    socket
        .set_read_timeout(Some(Duration::from_millis(200)))
        .map_err(|err| {
            QuoteError::NetworkError(format!("failed to set UDP read timeout: {err}"))
        })?;

    let handle = thread::Builder::new()
        .name("udp-listener".to_string())
        .spawn(move || listen_loop(socket, shutdown))
        .map_err(|err| QuoteError::NetworkError(format!("failed to spawn UDP listener: {err}")))?;

    Ok(handle)
}

fn listen_loop(socket: UdpSocket, shutdown: Arc<AtomicBool>) {
    let mut buffer = [0u8; 2048];
    info!(
        "Listening for UDP quotes on {}",
        socket
            .local_addr()
            .map(|addr| addr.to_string())
            .unwrap_or_else(|_| "<unknown>".into())
    );

    while !shutdown.load(Ordering::SeqCst) {
        match socket.recv(&mut buffer) {
            Ok(size) => {
                if let Err(err) = handle_payload(&buffer[..size]) {
                    warn!("{err}");
                }
            }
            Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(50));
            }
            Err(err) if err.kind() == std::io::ErrorKind::TimedOut => {}
            Err(err) => {
                warn!("UDP receive error: {}", err);
                thread::sleep(Duration::from_millis(100));
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
}
