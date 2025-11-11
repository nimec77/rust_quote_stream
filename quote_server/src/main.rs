mod generator;
mod tcp_handler;

use std::collections::HashMap;

use crossbeam::channel;
use log::{error, info};

use quote_common::{DEFAULT_QUOTE_RATE_MS, POPULAR_TICKERS, QuoteError, StockQuote};

use generator::start_generator;
use tcp_handler::{StreamRequest, start_tcp_server};

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

    let (receiver, handle) = start_generator(tickers, initial_prices, Some(DEFAULT_QUOTE_RATE_MS))?;
    let (request_tx, request_rx) = channel::unbounded();
    let (shutdown_tx, tcp_handle) = start_tcp_server("127.0.0.1:8080", request_tx.clone())?;

    log_sample_quotes(receiver.iter().take(5));

    for request in request_rx.try_iter() {
        log_stream_request(&request);
    }

    shutdown_tx
        .send(())
        .map_err(|err| QuoteError::NetworkError(format!("failed to stop TCP server: {err}")))?;
    tcp_handle
        .join()
        .map_err(|_| QuoteError::NetworkError("tcp server thread panicked".to_string()))?;

    drop(request_tx);
    drop(receiver);
    handle
        .join()
        .map_err(|_| QuoteError::NetworkError("generator thread panicked".to_string()))?;

    Ok(())
}

fn log_sample_quotes<I>(quotes: I)
where
    I: IntoIterator<Item = StockQuote>,
{
    for quote in quotes {
        info!(
            "Generated quote [{}]: price=${:.2}, volume={}, ts={}",
            quote.ticker, quote.price, quote.volume, quote.timestamp
        );
    }
    info!("Sample logging complete; shutting down generator thread.");
}

fn log_stream_request(request: &StreamRequest) {
    info!(
        "Client requested STREAM to {} for [{}]",
        request.udp_addr,
        request.tickers.join(",")
    );
}
