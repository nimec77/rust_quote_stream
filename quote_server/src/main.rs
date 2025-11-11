use std::collections::HashMap;

use log::{error, info};

use quote_common::{DEFAULT_QUOTE_RATE_MS, POPULAR_TICKERS, QuoteError, StockQuote};

mod generator;

use generator::start_generator;

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

    log_sample_quotes(receiver.iter().take(5));

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
