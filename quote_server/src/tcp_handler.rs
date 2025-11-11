use std::io::{BufRead, BufReader, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::str::FromStr;
use std::thread;
use std::time::Duration;

use crossbeam::channel::Sender;
use log::{info, warn};

use quote_common::QuoteError;

/// Parsed representation of a valid STREAM command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StreamRequest {
    pub udp_addr: SocketAddr,
    pub tickers: Vec<String>,
}

/// Parse an incoming STREAM command into a `StreamRequest`.
pub fn parse_stream_command(command: &str) -> Result<StreamRequest, QuoteError> {
    let trimmed = command.trim();
    let rest = trimmed
        .strip_prefix("STREAM ")
        .ok_or_else(|| QuoteError::InvalidCommand("missing STREAM prefix".to_string()))?;

    let (addr_part, tickers_part) = rest.split_once(' ').ok_or_else(|| {
        QuoteError::InvalidCommand("STREAM command missing ticker list".to_string())
    })?;

    let udp_addr = addr_part.strip_prefix("udp://").ok_or_else(|| {
        QuoteError::InvalidCommand("STREAM command missing udp:// prefix".to_string())
    })?;

    let socket_addr = SocketAddr::from_str(udp_addr)
        .map_err(|_| QuoteError::InvalidCommand(format!("invalid UDP address: {udp_addr}")))?;

    let tickers = tickers_part
        .split(',')
        .map(|ticker| ticker.trim().to_uppercase())
        .filter(|ticker| !ticker.is_empty())
        .collect::<Vec<_>>();

    if tickers.is_empty() {
        return Err(QuoteError::InvalidCommand(
            "ticker list cannot be empty".to_string(),
        ));
    }

    for ticker in &tickers {
        if !ticker
            .chars()
            .all(|ch| ch.is_ascii_uppercase() || ch.is_ascii_digit())
        {
            return Err(QuoteError::InvalidCommand(format!(
                "invalid ticker symbol: {ticker}"
            )));
        }
    }

    Ok(StreamRequest {
        udp_addr: socket_addr,
        tickers,
    })
}

fn handle_connection(
    mut stream: TcpStream,
    request_tx: &Sender<StreamRequest>,
) -> Result<(), QuoteError> {
    let peer_addr = stream
        .peer_addr()
        .map(|addr| addr.to_string())
        .unwrap_or_else(|_| "<unknown>".to_string());

    let mut reader = BufReader::new(stream.try_clone()?);
    let mut line = String::new();
    let bytes_read = reader.read_line(&mut line).map_err(QuoteError::from)?;

    if bytes_read == 0 {
        return Ok(());
    }

    match parse_stream_command(&line) {
        Ok(request) => {
            if let Err(err) = request_tx.send(request.clone()) {
                let message = "server unavailable";
                stream
                    .write_all(format!("ERR {message}\n").as_bytes())
                    .and_then(|_| stream.flush())
                    .map_err(QuoteError::from)?;
                warn!("Failed to forward stream request from {peer_addr}: {err}");
            } else {
                stream
                    .write_all(b"OK\n")
                    .and_then(|_| stream.flush())
                    .map_err(QuoteError::from)?;
                info!(
                    "Accepted STREAM request from {peer_addr} for {}",
                    request.tickers.join(",")
                );
            }
        }
        Err(err) => {
            stream
                .write_all(format!("ERR {}\n", err).as_bytes())
                .and_then(|_| stream.flush())
                .map_err(QuoteError::from)?;
            warn!("Invalid STREAM command from {peer_addr}: {err}");
        }
    }

    Ok(())
}

/// Start TCP server listening for STREAM commands, returning a shutdown sender and join handle.
pub fn start_tcp_server(
    addr: &str,
    request_tx: Sender<StreamRequest>,
) -> Result<(Sender<()>, thread::JoinHandle<()>), QuoteError> {
    let listener = TcpListener::bind(addr)?;
    listener.set_nonblocking(true)?;
    info!("TCP server listening on {addr}");

    let (shutdown_tx, shutdown_rx) = crossbeam::channel::bounded(1);

    let handle = thread::Builder::new()
        .name("tcp-listener".to_string())
        .spawn(move || {
            let poll_interval = Duration::from_millis(100);
            loop {
                if shutdown_rx.try_recv().is_ok() {
                    break;
                }

                match listener.accept() {
                    Ok((stream, _)) => {
                        if let Err(err) = handle_connection(stream, &request_tx) {
                            warn!("Failed to handle connection: {err}");
                        }
                    }
                    Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(poll_interval);
                    }
                    Err(err) => {
                        warn!("TCP accept error: {err}");
                        thread::sleep(poll_interval);
                    }
                }
            }
            info!("TCP server shutting down");
        })
        .map_err(QuoteError::from)?;

    Ok((shutdown_tx, handle))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_stream_command_valid() {
        let command = "STREAM udp://127.0.0.1:9000 aapl, tsla \n";
        let result = parse_stream_command(command).expect("valid command");
        assert_eq!(result.udp_addr, "127.0.0.1:9000".parse().unwrap());
        assert_eq!(result.tickers, vec!["AAPL".to_string(), "TSLA".to_string()]);
    }

    #[test]
    fn test_parse_stream_command_missing_prefix() {
        let err = parse_stream_command("START udp://127.0.0.1:9000 AAPL").expect_err("should fail");
        assert!(matches!(err, QuoteError::InvalidCommand(_)));
    }

    #[test]
    fn test_parse_stream_command_invalid_address() {
        let err = parse_stream_command("STREAM udp://bad-address AAPL").expect_err("should fail");
        assert!(matches!(err, QuoteError::InvalidCommand(_)));
    }

    #[test]
    fn test_parse_stream_command_empty_tickers() {
        let err = parse_stream_command("STREAM udp://127.0.0.1:9000   ").expect_err("should fail");
        assert!(matches!(err, QuoteError::InvalidCommand(_)));
    }

    #[test]
    fn test_parse_stream_command_invalid_ticker() {
        let err =
            parse_stream_command("STREAM udp://127.0.0.1:9000 a$pl").expect_err("should fail");
        assert!(matches!(err, QuoteError::InvalidCommand(_)));
    }
}
