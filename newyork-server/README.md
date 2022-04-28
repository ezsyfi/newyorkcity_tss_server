# Newyork Server
![Newyork Server](../misc/server-icon.png)

## Introduction
Newyork server is a RESTful web service exposing APIs for two party ECDSA key generation and signing.

## Installation
### Launching the server
```bash
git clone https://github.com/ezsyfi/newyorkcity_tss_server.git
cd newyork-city/newyork-server
RUST_LOG=info cargo run --release
```

* By default, the server will use a local [RocksDB](https://rocksdb.org/).<br> 

### Running tests
```bash
cargo test --verbose
```

#### Without timing output
```bash
RUST_TEST_THREADS=1 cargo test --release
```

#### With timing output
```bash
RUST_TEST_THREADS=1  cargo test --release -- --nocapture
```
