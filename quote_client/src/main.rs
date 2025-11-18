use std::net::{SocketAddr, UdpSocket};
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::Duration;

use log::info;

use quote_common::QuoteError;

mod cli;
mod tcp_client;
mod udp_receiver;

use cli::{load_tickers, parse};
use tcp_client::send_stream_command;
use udp_receiver::{spawn_listener, spawn_ping_thread};

const CLIENT_SHUTDOWN_GRACE_MS: u64 = 200;

fn main() {
    env_logger::init();

    if let Err(err) = run() {
        quote_common::log_error!(err, "Client exited with error");
        std::process::exit(1);
    }
}

fn run() -> Result<(), QuoteError> {
    let args = parse();

    let tickers = load_tickers(&args.tickers_file)?;
    let server_addr: SocketAddr = args.server_addr.parse().map_err(|err| {
        quote_common::quote_error!(
            ConfigError,
            "invalid server address '{}': {}",
            args.server_addr,
            err
        )
    })?;

    let socket = UdpSocket::bind(("0.0.0.0", args.udp_port)).map_err(|err| {
        quote_common::quote_error!(NetworkError, "failed to bind UDP socket: {}", err)
    })?;
    let local_addr = socket.local_addr().map_err(|err| {
        quote_common::quote_error!(NetworkError, "failed to read UDP socket address: {}", err)
    })?;

    // Send STREAM command and get the client's IP address from the TCP connection.
    // The function constructs the UDP address using the client's IP (from TCP connection)
    // and the UDP port, ensuring the server can send UDP packets back to this client.
    let client_ip = send_stream_command(&args.server_addr, local_addr.port(), &tickers)?;
    let advertised_udp_addr = format!("{}:{}", client_ip, local_addr.port());

    info!(
        "Bound UDP listener on {} (advertising to server as {})",
        local_addr, advertised_udp_addr
    );

    // Set up shutdown flag for thread coordination
    let shutdown = quote_common::setup_shutdown_flag()?;

    // Clone socket for ping thread before moving original to listener
    let ping_socket = socket.try_clone().map_err(|err| {
        quote_common::quote_error!(NetworkError, "failed to clone UDP socket: {}", err)
    })?;

    let listener_handle = spawn_listener(socket, Arc::clone(&shutdown))?;
    let ping_handle = spawn_ping_thread(ping_socket, server_addr, Arc::clone(&shutdown))?;

    info!("STREAM established; press Ctrl+C to stop.");

    // Wait for shutdown signal (set by Ctrl+C handler)
    while !shutdown.load(Ordering::SeqCst) {
        std::thread::sleep(Duration::from_millis(100));
    }

    // Allow threads to notice shutdown signal.
    std::thread::sleep(Duration::from_millis(CLIENT_SHUTDOWN_GRACE_MS));

    listener_handle
        .join()
        .map_err(|_| quote_common::quote_error!(NetworkError, "UDP listener thread panicked"))?;
    ping_handle
        .join()
        .map_err(|_| quote_common::quote_error!(NetworkError, "ping thread panicked"))?;

    info!("Client shut down cleanly.");

    Ok(())
}
