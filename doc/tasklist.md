# Development Task List

## 📊 Progress Report

| Iteration | Description | Status | Completion |
|-----------|-------------|--------|------------|
| 0️⃣ | Project Setup | ✅ Completed | 100% |
| 1️⃣ | Common Library | ✅ Completed | 100% |
| 2️⃣ | Quote Generator | ✅ Completed | 100% |
| 3️⃣ | TCP Server | ⏳ Pending | 0% |
| 4️⃣ | UDP Streaming | ⏳ Pending | 0% |
| 5️⃣ | Keep-Alive Mechanism | ⏳ Pending | 0% |
| 6️⃣ | Basic Client | ⏳ Pending | 0% |
| 7️⃣ | Client Ping Thread | ⏳ Pending | 0% |
| 8️⃣ | Configuration & Logging | ⏳ Pending | 0% |
| 9️⃣ | Testing & Polish | ⏳ Pending | 0% |

**Legend:**
- ⏳ Pending
- 🔄 In Progress
- ✅ Completed
- ⚠️ Blocked

---

## Iteration 0️⃣: Project Setup

**Goal:** Create workspace structure and configuration files

### Tasks:
- [x] Create `Cargo.toml` workspace with 3 crates
- [x] Create `quote_common/` library crate structure
- [x] Create `quote_server/` binary crate structure
- [x] Create `quote_client/` binary crate structure
- [x] Create `tickers.txt` with sample tickers
- [x] Create `server_config.toml` with defaults
- [x] Create `README.md` with basic instructions

### Testing:
- [x] Verify `cargo build` succeeds for all crates
- [x] Verify workspace structure matches `vision.md`

---

## Iteration 1️⃣: Common Library

**Goal:** Implement shared data structures and utilities

### Tasks:
- [x] Define `StockQuote` struct with serde derives
- [x] Define `QuoteError` enum with all variants
- [x] Implement `From<std::io::Error>` for `QuoteError`
- [x] Define constants (timeouts, rates, popular tickers)
- [x] Add dependencies: `serde`, `serde_json`, `chrono`
- [x] Write unit tests for serialization/deserialization

### Testing:
- [x] Test `StockQuote` JSON serialization
- [x] Test `StockQuote` JSON deserialization
- [x] Verify error conversions work
- [x] Run `cargo test` in `quote_common`

---

## Iteration 2️⃣: Quote Generator

**Goal:** Generate continuous stream of stock quotes

### Tasks:
- [x] Create `generator.rs` in `quote_server`
- [x] Implement `QuoteGenerator` struct with ticker state (HashMap)
- [x] Implement random walk price algorithm
- [x] Implement volume generation (popular vs regular tickers)
- [x] Create MPMC channel for broadcasting quotes
- [x] Spawn generator thread in main
- [x] Add dependencies: `rand`, `crossbeam`
- [x] Write unit tests for price/volume generation

### Testing:
- [x] Run server, verify quotes generate continuously
- [x] Check price changes are within ±2% range
- [x] Verify volume ranges (popular: 1000-6000, others: 100-1100)
- [x] Log generated quotes to console (temporary debug)

---

## Iteration 3️⃣: TCP Server

**Goal:** Accept client connections and parse STREAM commands

### Tasks:
- [ ] Create `tcp_handler.rs` in `quote_server`
- [ ] Implement TCP listener on configured address
- [ ] Accept incoming connections in loop
- [ ] Parse STREAM command format
- [ ] Validate UDP address and ticker list
- [ ] Respond with `OK` or `ERR <message>`
- [ ] Write unit tests for command parsing

### Testing:
- [ ] Start server, connect with `telnet` or `nc`
- [ ] Send valid STREAM command, verify `OK` response
- [ ] Send invalid commands, verify `ERR` responses
- [ ] Test malformed UDP addresses
- [ ] Test empty ticker lists

---

## Iteration 4️⃣: UDP Streaming

**Goal:** Stream filtered quotes to clients via UDP

### Tasks:
- [ ] Create `udp_streamer.rs` in `quote_server`
- [ ] Spawn client thread on valid STREAM command
- [ ] Subscribe client thread to MPMC channel
- [ ] Filter quotes by client's ticker list
- [ ] Serialize quotes to JSON
- [ ] Send UDP packets to client address
- [ ] Handle UDP send errors gracefully
- [ ] Write tests for filtering logic

### Testing:
- [ ] Start server, send STREAM command
- [ ] Use `nc -u -l <port>` to listen for UDP packets
- [ ] Verify only requested tickers are received
- [ ] Verify JSON format matches `StockQuote` structure
- [ ] Test with multiple simultaneous clients

---

## Iteration 5️⃣: Keep-Alive Mechanism

**Goal:** Detect disconnected clients and cleanup resources

### Tasks:
- [ ] Add PING monitoring to client thread
- [ ] Create UDP socket for receiving PINGs
- [ ] Implement 5-second timeout logic
- [ ] Terminate thread on timeout
- [ ] Log client connection/disconnection events
- [ ] Add `log` and `env_logger` dependencies
- [ ] Write tests for timeout detection

### Testing:
- [ ] Connect client, stop sending PINGs
- [ ] Verify server detects timeout after 5 seconds
- [ ] Verify thread terminates and resources freed
- [ ] Check server logs for disconnect events
- [ ] Verify other clients unaffected by timeout

---

## Iteration 6️⃣: Basic Client

**Goal:** Connect to server and receive quotes

### Tasks:
- [ ] Create `cli.rs` with clap argument parsing
- [ ] Create `tcp_client.rs` for server connection
- [ ] Create `udp_receiver.rs` for quote reception
- [ ] Parse tickers from file
- [ ] Connect to TCP server
- [ ] Send STREAM command with local UDP address
- [ ] Bind UDP socket and receive quotes
- [ ] Deserialize and log received quotes
- [ ] Add dependencies: `clap`
- [ ] Handle Ctrl+C for graceful shutdown

### Testing:
- [ ] Run server and client together
- [ ] Verify client connects successfully
- [ ] Verify quotes received and logged
- [ ] Verify only requested tickers appear
- [ ] Test Ctrl+C cleanup

---

## Iteration 7️⃣: Client Ping Thread

**Goal:** Send keep-alive PINGs to server

### Tasks:
- [ ] Create ping thread in client
- [ ] Send "PING" via UDP every 2 seconds
- [ ] Use same UDP socket as quote receiver
- [ ] Handle send errors gracefully
- [ ] Ensure thread terminates on shutdown
- [ ] Write tests for ping timing

### Testing:
- [ ] Run client, verify server receives PINGs
- [ ] Check server logs for PING activity
- [ ] Stop client, verify server detects timeout
- [ ] Verify ping frequency (every 2 seconds)
- [ ] Test long-running connection (>1 minute)

---

## Iteration 8️⃣: Configuration & Logging

**Goal:** Load settings from files and implement proper logging

### Tasks:
- [ ] Implement TOML config parsing in server
- [ ] Load tickers from file with error handling
- [ ] Load initial prices from config
- [ ] Replace all debug prints with log macros
- [ ] Initialize `env_logger` in both binaries
- [ ] Add appropriate log levels (info/warn/error/debug)
- [ ] Add `toml` dependency to server

### Testing:
- [ ] Test with missing config file (should error)
- [ ] Test with invalid TOML syntax
- [ ] Test with missing ticker in initial_prices
- [ ] Run with different RUST_LOG levels
- [ ] Verify no `println!` statements remain

---

## Iteration 9️⃣: Testing & Polish

**Goal:** Complete testing and code quality improvements

### Tasks:
- [ ] Write unit tests for all main functions
- [ ] Test error handling paths
- [ ] Test edge cases (empty files, malformed data)
- [ ] Run `cargo clippy` and fix all warnings
- [ ] Run `cargo fmt` on all code
- [ ] Review all error messages for clarity
- [ ] Add doc comments to public APIs
- [ ] Update README with complete usage instructions
- [ ] Create example `tickers.txt` and `server_config.toml`
- [ ] Test multi-client scenarios

### Testing:
- [ ] Run full integration test: server + 3 clients
- [ ] Verify resource cleanup (no leaked threads/sockets)
- [ ] Test network error scenarios (blocked ports, etc.)
- [ ] Verify all checklist items from `idea.md`
- [ ] Performance check: handle 100+ quotes/second
- [ ] Memory check: run for extended period

---

## 🎯 Final Deliverables

- [ ] Working `quote_server` binary
- [ ] Working `quote_client` binary
- [ ] Comprehensive test coverage
- [ ] Clean code (no clippy warnings)
- [ ] Complete README with examples
- [ ] Sample configuration files
- [ ] All requirements from `idea.md` satisfied

---

## 📝 Notes

- Test each iteration thoroughly before moving to next
- Keep commits small and focused on one iteration
- Update progress table after completing each iteration
- Refer to `vision.md` for architecture details
- Follow all rules in `conventions.md`
- If blocked, document reason in progress table

---

## 🔄 Progress Updates

### Template for Updates:
```
### [Date] - Iteration X Completed
- Duration: X hours
- Challenges: [any issues encountered]
- Next: Iteration Y
```

---

### [2025-11-10] - Iteration 0 Completed
- Duration: 45 minutes
- Challenges: None; standard cargo scaffolding
- Next: Iteration 1 – Common Library

---

### [2025-11-11] - Iteration 1 Completed
- Duration: 35 minutes
- Challenges: Adjusted to workspace dependency updates
- Next: Iteration 2 – Quote Generator

---

### [2025-11-11] - Iteration 2 Completed
- Duration: 50 minutes
- Challenges: Reworking borrow semantics for ticker iteration
- Next: Iteration 3 – TCP Server

---

**Start Date:** 2025-11-10

**Target Completion:** To be estimated after Iteration 2

