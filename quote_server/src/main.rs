mod config;
mod generator;
mod tcp_handler;
mod udp_streamer;

use std::path::Path;
use std::time::Duration;

use crossbeam::channel;
use log::{error, info};

use quote_common::QuoteError;

use config::{load_config, load_tickers};
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
    let config = load_config(Path::new("server_config.toml"))?;

    info!("Loaded configuration:");
    info!("  TCP address: {}", config.tcp_addr);
    info!("  Tickers file: {}", config.tickers_file);
    info!("  Quote rate: {}ms", config.quote_rate_ms);
    info!("  Keepalive timeout: {}s", config.keepalive_timeout_secs);
    info!("  Initial prices: {} tickers", config.initial_prices.len());

    let tickers = load_tickers(Path::new(&config.tickers_file))?;
    info!("Loaded {} tickers from file", tickers.len());

    let (quote_rx, generator_handle) = start_generator(
        tickers,
        config.initial_prices.clone(),
        Some(config.quote_rate_ms),
    )?;

    let keepalive_timeout = Duration::from_secs(config.keepalive_timeout_secs);
    let server_tcp_addr: std::net::SocketAddr = config.tcp_addr.parse().map_err(|err| {
        QuoteError::ConfigError(format!("invalid TCP address '{}': {}", config.tcp_addr, err))
    })?;
    let (dispatcher_tx, dispatcher_handle) = start_udp_streamer(quote_rx, keepalive_timeout, server_tcp_addr)?;

    let (request_tx, request_rx) = channel::unbounded::<StreamRequest>();
    let (_shutdown_tx, tcp_handle) = start_tcp_server(&config.tcp_addr, request_tx.clone())?;

    // Process requests continuously until TCP server thread exits
    // The TCP server thread runs until it receives a shutdown signal
    while let Ok(request) = request_rx.recv() {
        log_stream_request(&request);
        dispatcher_tx
            .send(UdpCommand::AddClient(request))
            .map_err(|err| {
                QuoteError::NetworkError(format!("failed to register UDP client: {err}"))
            })?;
    }

    // Wait for TCP server thread to finish
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
