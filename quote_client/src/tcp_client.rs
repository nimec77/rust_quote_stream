use std::io::{BufRead, BufReader, Write};
use std::net::{IpAddr, SocketAddr, TcpStream};
use std::time::Duration;

use log::{debug, info};

use quote_common::{QuoteError, RESPONSE_ERR_PREFIX, RESPONSE_OK, UDP_SCHEME_PREFIX};

const STREAM_PREFIX: &str = "STREAM";
const TCP_READ_TIMEOUT_SECS: u64 = 5;

/// Send a STREAM command to the server and verify the response.
/// Returns the client's IP address as seen from the TCP connection.
/// The UDP address is constructed using the client's IP from the TCP connection
/// and the provided UDP port, ensuring the server can send UDP packets back.
pub fn send_stream_command(
    server_addr: SocketAddr,
    udp_port: u16,
    tickers: &[String],
) -> Result<IpAddr, QuoteError> {
    debug!("Connecting to TCP server {}", server_addr);
    let mut stream = TcpStream::connect(server_addr)
        .map_err(|err| quote_common::quote_error!(NetworkError, "TCP connect failed: {}", err))?;

    // Get the client's IP address from the TCP connection's local address.
    // This is the IP address the client uses to reach the server, which is
    // the correct address to advertise for UDP reception.
    let client_ip = stream
        .local_addr()
        .map_err(|err| {
            quote_common::quote_error!(NetworkError, "failed to get TCP local address: {}", err)
        })?
        .ip();

    // Construct the UDP address using the client's IP and the provided port
    let udp_addr = SocketAddr::new(client_ip, udp_port);
    let command = build_stream_command(udp_addr, tickers);

    stream
        .set_read_timeout(Some(Duration::from_secs(TCP_READ_TIMEOUT_SECS)))
        .map_err(|err| {
            quote_common::quote_error!(NetworkError, "set_read_timeout failed: {}", err)
        })?;

    stream
        .write_all(command.as_bytes())
        .and_then(|_| stream.flush())
        .map_err(|err| {
            quote_common::quote_error!(NetworkError, "failed to send STREAM command: {}", err)
        })?;

    let mut reader = BufReader::new(stream);
    let mut response = String::new();
    reader.read_line(&mut response).map_err(|err| {
        quote_common::quote_error!(NetworkError, "failed to read server response: {}", err)
    })?;

    interpret_response(response.trim_end())?;
    Ok(client_ip)
}

fn build_stream_command(udp_addr: SocketAddr, tickers: &[String]) -> String {
    let ticker_list = tickers.join(",");
    format!("{STREAM_PREFIX} {UDP_SCHEME_PREFIX}{udp_addr} {ticker_list}\n")
}

fn interpret_response(response: &str) -> Result<(), QuoteError> {
    if response == RESPONSE_OK {
        info!("STREAM command accepted");
        return Ok(());
    }

    if let Some(rest) = response.strip_prefix(RESPONSE_ERR_PREFIX) {
        return Err(quote_common::quote_error!(InvalidCommand, "{}", rest));
    }

    Err(quote_common::quote_error!(
        ParseError,
        "unexpected response from server: {}",
        response
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_stream_command_formats_correctly() {
        let addr: SocketAddr = "127.0.0.1:4000".parse().unwrap();
        let cmd = build_stream_command(addr, &["AAPL".into(), "TSLA".into()]);
        assert_eq!(cmd, "STREAM udp://127.0.0.1:4000 AAPL,TSLA\n");
    }

    #[test]
    fn test_interpret_response_ok() {
        assert!(interpret_response("OK").is_ok());
    }

    #[test]
    fn test_interpret_response_err() {
        let err = interpret_response("ERR invalid").expect_err("should fail");
        assert!(
            matches!(err, QuoteError::InvalidCommand { ref message, .. } if message == "invalid")
        );
    }

    #[test]
    fn test_interpret_response_unexpected() {
        let err = interpret_response("UNKNOWN").expect_err("should fail");
        assert!(matches!(err, QuoteError::ParseError { .. }));
    }
}
