# Module Project 2: Quote Streaming

## Project Overview

A real-time stock quote streaming system consisting of two Rust applications: a quote server (generator) and a quote client. The system uses TCP for command/control and UDP for high-frequency data streaming.

## Architecture

- **Two standalone applications**: Separate binary crates in a workspace
- **Communication**: TCP for control, UDP for data streaming
- **Concurrency**: Multi-threaded design with channel-based data flow
- **Keep-Alive**: Ping/Pong mechanism to manage active connections

---

## Application 1: Quote Generator (Quote Server)

### Core Functionality

#### 1. Data Generation

Generate artificial stock price data with the following structure:

```rust
#[derive(Debug, Clone)]
pub struct StockQuote {
    pub ticker: String,
    pub price: f64,
    pub volume: u32,
    pub timestamp: u64,
}
```

**Key Features:**
- Support multiple tickers (AAPL, GOOGL, TSLA, etc.)
- Price changes using random walk algorithm
- Volume generation based on stock popularity
- Timestamp in milliseconds since UNIX epoch

**Serialization Methods:**

1. **Text Format**: `ticker|price|volume|timestamp`
   - Example: `AAPL|150.25|3500|1699564800000`

2. **Binary Format**: Byte representation
   - Custom implementation or use existing serialization crates

**Volume Generation Logic:**
- Popular stocks (AAPL, MSFT, TSLA): 1000-6000 shares
- Other stocks: 100-1100 shares

#### 2. TCP Server

- Listen for incoming client connections
- Accept and parse commands from clients
- Manage client sessions and streaming threads

#### 3. Command Protocol

**Primary Command: STREAM**

Format: `STREAM udp://<ip>:<port> <ticker1>,<ticker2>,...`

Example: `STREAM udp://127.0.0.1:34254 AAPL,TSLA`

**Parameters:**
- UDP address and port for data delivery
- Comma-separated list of tickers to stream

**Response:**
- `OK` - Command accepted, streaming started
- `ERR <message>` - Error occurred (invalid format, etc.)

#### 4. Command Processing

Upon receiving a valid STREAM command:
1. Parse UDP address and ticker list
2. Validate parameters
3. Create a new thread for this client
4. Begin streaming filtered data to the specified UDP address

#### 5. Ping/Pong (Keep-Alive) Mechanism

**Purpose**: Detect disconnected clients and cleanup resources

**Implementation:**
- Monitor thread waits for "PING" messages via UDP
- Clients must send PING every ~2 seconds
- Server timeout: 5 seconds
- If no PING received within timeout:
  - Stop streaming data for that client
  - Terminate the client's thread
  - Free associated resources

#### 6. Multithreading Architecture

**Thread Structure:**
- **Main thread**: Accept TCP connections
- **Generator thread**: Continuously generate quote data
- **Client threads**: One per active client, handles UDP streaming
- **Monitor threads**: Track keep-alive status

**Data Flow:**
- Use channels (`std::sync::mpsc` or `crossbeam`) to transfer data
- Generator produces data → Multiple clients consume
- Consider using `crossbeam` for MPMC (Multiple Producer, Multiple Consumer) patterns

### Ticker List (Example)

```
AAPL    MSFT    GOOGL   AMZN    NVDA    META    TSLA    JPM
JNJ     V       PG      UNH     HD      DIS     PYPL    NFLX
ADBE    CRM     INTC    CSCO    PFE     ABT     TMO     ABBV
LLY     PEP     COST    TXN     AVGO    ACN     QCOM    DHR
MDT     NKE     UPS     RTX     HON     ORCL    LIN     AMGN
LOW     SBUX    SPGI    INTU    ISRG    T       BMY     DE
PLD     CI      CAT     GS      UNP     AMT     AXP     MS
BLK     GE      SYK     GILD    MMM     MO      LMT     FISV
ADI     BKNG    C       SO      NEE     ZTS     TGT     DUK
ICE     BDX     PNC     CMCSA   SCHW    MDLZ    TJX     USB
CL      EMR     APD     COF     FDX     AON     WM      ECL
ITW     VRTX    D       NSC     PGR     ETN     FIS     PSA
KLAC    MCD     ADP     APTV    AEP     MCO     SHW     DD
ROP     SLB     HUM     BSX     NOC     EW
```

---

## Application 2: Quote Client

### Core Functionality

#### 1. Command-Line Interface

**Using `clap` crate for argument parsing:**

```bash
quote_client \
  --server-addr 127.0.0.1:8080 \
  --udp-port 34254 \
  --tickers-file tickers.txt
```

**Arguments:**
- `--server-addr`: TCP server address and port
- `--udp-port`: Local port for receiving UDP data
- `--tickers-file`: Path to file containing ticker list

#### 2. Ticker File Format

**Example `tickers.txt`:**
```
AAPL
GOOGL
TSLA
```

**Requirements:**
- One ticker per line
- Ignore empty lines and whitespace
- Handle file not found errors gracefully

#### 3. Connection Flow

1. Read ticker list from file
2. Connect to TCP server
3. Send STREAM command with local UDP port and ticker list
4. Wait for server response (OK/ERR)
5. Begin receiving UDP data

#### 4. UDP Data Reception

**Main Thread Responsibilities:**
- Create UDP socket bound to specified port
- Receive incoming quote packets
- Parse and deserialize quote data
- Display quotes to console (filtered by requested tickers)
- Handle packet loss gracefully

**Output Format:**
```
[AAPL] Price: $150.25 | Volume: 3500 | Time: 1699564800000
[TSLA] Price: $242.50 | Volume: 4200 | Time: 1699564801000
```

#### 5. Ping Thread

**Separate Thread for Keep-Alive:**
- Send "PING" message to server's UDP address
- Frequency: Every 2 seconds
- Use same address/port where data is received
- Continue until application terminates

#### 6. Graceful Termination

**Handle Ctrl+C and cleanup:**
- Register signal handler
- Close TCP connection
- Stop UDP socket
- Join all threads
- Exit cleanly

---

## Quality Checklist

### Project Build and Structure

- [ ] Two binary applications: `quote_server` and `quote_client`
- [ ] Project compiles without errors (`cargo build`)
- [ ] Logical code organization: modules and functions
- [ ] Common structures in `lib.rs`
- [ ] Both binaries configured in `Cargo.toml`
- [ ] `README.md` with setup and usage instructions

### TCP Interaction

- [ ] Server starts and listens on TCP port
- [ ] Client connects to server without errors
- [ ] Client sends correct STREAM command
- [ ] Server parses and validates command
- [ ] Server responds with OK/ERR appropriately
- [ ] Invalid commands return clear error messages

### UDP Data Streaming

- [ ] Server sends data via UDP after STREAM command
- [ ] Each client has dedicated sending thread
- [ ] Data format: JSON `{"ticker":"AAPL","price":...,"volume":...,"timestamp":...}`
- [ ] Client receives and parses UDP messages
- [ ] Ticker filtering works correctly
- [ ] Packet loss doesn't crash application
- [ ] Error handling for network issues

### Keep-Alive (Ping/Pong)

- [ ] Client sends PING every 2 seconds via UDP
- [ ] Server receives PING and updates client activity
- [ ] Server detects timeout (5 seconds)
- [ ] Server stops streaming after timeout
- [ ] Client thread terminates gracefully
- [ ] Resources freed properly
- [ ] New clients don't disrupt existing threads

### Multithreading and Synchronization

- [ ] Quote generator in separate thread
- [ ] Each client served in own thread
- [ ] Thread-safe primitives used (Arc, Mutex, mpsc, Atomic)
- [ ] No data races or deadlocks
- [ ] Threads terminate gracefully
- [ ] No thread hanging or freezing

### File Operations and Arguments

- [ ] Client uses `clap` for argument parsing
- [ ] Arguments: `--server-addr`, `--udp-port`, `--tickers-file`
- [ ] Ticker file read line by line
- [ ] Empty lines and whitespace ignored
- [ ] Clear error messages for missing/invalid files

### Error Handling

- [ ] All network operations return `Result`
- [ ] No unconditional `unwrap()` calls
- [ ] Graceful error messages
- [ ] Connection errors handled
- [ ] Invalid data handled
- [ ] Clean shutdown on errors

### Code Quality

- [ ] Meaningful variable and function names
- [ ] No magic numbers (use constants)
- [ ] Comments on complex logic
- [ ] Logical module separation
- [ ] No code duplication
- [ ] Proper resource management (no leaks)
- [ ] No deadlocks in channels

---

## Technical Implementation Notes

### Recommended Crates

- **`clap`**: Command-line argument parsing
- **`crossbeam`**: Advanced concurrency primitives (MPMC channels)
- **`serde`** + **`serde_json`**: JSON serialization/deserialization
- **`rand`**: Random number generation for price simulation
- **`chrono`**: Time handling (optional, can use std::time)

### Data Serialization Options

1. **Text-based** (Pipe-delimited): Simple, human-readable, easy to debug
2. **JSON**: Standard, widely supported, easy to extend
3. **Binary** (Custom): Most efficient, requires careful implementation
4. **Binary** (MessagePack/Bincode): Efficient, less manual work

### Channel Strategies

**For Generator → Clients:**
- Use `crossbeam::channel::unbounded()` for MPMC
- Alternative: Single generator broadcasts to multiple receivers
- Consider backpressure handling if clients are slow

### Network Considerations

- **TCP**: Reliable, connection-oriented (commands)
- **UDP**: Fast, connectionless, may drop packets (data streaming)
- Handle UDP packet size limits (typical: 65,507 bytes max)
- Consider message batching for efficiency

### Testing Strategy

1. **Unit Tests**: Quote generation, serialization, parsing
2. **Integration Tests**: TCP/UDP communication
3. **Manual Tests**: 
   - Single client connection
   - Multiple simultaneous clients
   - Client timeout simulation
   - Network interruption handling
   - High-frequency data streaming

---

## Example Usage

### Starting the Server

```bash
cd quote_server
cargo run --release
# Server listening on 127.0.0.1:8080
```

### Starting the Client

```bash
cd quote_client
cargo run --release -- \
  --server-addr 127.0.0.1:8080 \
  --udp-port 34254 \
  --tickers-file ../tickers.txt
```

### Expected Behavior

1. Client connects and sends: `STREAM udp://127.0.0.1:34254 AAPL,TSLA`
2. Server responds: `OK`
3. Server begins streaming quotes via UDP
4. Client displays incoming quotes
5. Client sends PING every 2 seconds
6. If client stops (Ctrl+C), server detects timeout and cleans up

---

## Extension Ideas

- **Metrics**: Track quotes sent, clients connected, bandwidth used
- **Logging**: Use `log` and `env_logger` crates
- **Configuration**: TOML config file for server settings
- **Dynamic Subscriptions**: Allow clients to add/remove tickers
- **TLS Support**: Secure TCP connections
- **Compression**: Compress UDP data for efficiency
- **Web UI**: Dashboard showing active clients and data flow
- **Historical Data**: Store quotes in database (SQLite/PostgreSQL)
- **Rate Limiting**: Prevent client abuse
- **Authentication**: Require credentials for streaming access

---

## Learning Objectives

This project teaches:
- Network programming (TCP/UDP)
- Multithreading and synchronization
- Channel-based communication
- Error handling in production code
- Resource management and cleanup
- Protocol design and implementation
- Real-time data streaming
- Keep-alive mechanisms
- Command-line application development

