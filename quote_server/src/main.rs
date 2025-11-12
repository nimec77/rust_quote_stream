mod generator;
mod tcp_handler;
mod udp_streamer;

use std::collections::HashMap;
use std::time::Duration;

use crossbeam::channel;
use log::{error, info};

use quote_common::{DEFAULT_QUOTE_RATE_MS, POPULAR_TICKERS, QuoteError};

use generator::start_generator;
use tcp_handler::{StreamRequest, start_tcp_server};
use udp_streamer::{UdpCommand, start_udp_streamer};

fn main() {
    env_logger::init();

    if let Err(err) = run() {
        error!("Server initialization failed: {}", err);
    }
}

fn run() -> Result<(), QuoteError> {
    let tickers = POPULAR_TICKERS
        .iter()
        .map(|ticker| ticker.to_string())
        .collect::<Vec<_>>();
    let initial_prices = HashMap::new();

    let (quote_rx, generator_handle) =
        start_generator(tickers, initial_prices, Some(DEFAULT_QUOTE_RATE_MS))?;

    let (dispatcher_tx, dispatcher_handle) = start_udp_streamer(quote_rx)?;

    let (request_tx, request_rx) = channel::unbounded::<StreamRequest>();
    let (shutdown_tx, tcp_handle) = start_tcp_server("127.0.0.1:8080", request_tx.clone())?;

    loop {
        match request_rx.recv_timeout(Duration::from_secs(1)) {
            Ok(request) => {
                log_stream_request(&request);
                dispatcher_tx
                    .send(UdpCommand::AddClient(request))
                    .map_err(|err| {
                        QuoteError::NetworkError(format!("failed to register UDP client: {err}"))
                    })?;
            }
            Err(channel::RecvTimeoutError::Timeout) => {
                break;
            }
            Err(channel::RecvTimeoutError::Disconnected) => break,
        }
    }

    shutdown_tx
        .send(())
        .map_err(|err| QuoteError::NetworkError(format!("failed to stop TCP server: {err}")))?;
    tcp_handle
        .join()
        .map_err(|_| QuoteError::NetworkError("tcp server thread panicked".to_string()))?;

    drop(request_tx);

    dispatcher_tx
        .send(UdpCommand::Shutdown)
        .map_err(|err| QuoteError::NetworkError(format!("failed to stop UDP dispatcher: {err}")))?;
    dispatcher_handle
        .join()
        .map_err(|_| QuoteError::NetworkError("udp dispatcher thread panicked".to_string()))?;

    generator_handle
        .join()
        .map_err(|_| QuoteError::NetworkError("generator thread panicked".to_string()))?;

    Ok(())
}

fn log_stream_request(request: &StreamRequest) {
    info!(
        "Client requested STREAM to {} for [{}]",
        request.udp_addr,
        request.tickers.join(",")
    );
}
