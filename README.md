# Stateless Block Verifier

This project provides tools for stateless verification of blocks with mpt state roots.

# Example

## Run and verify a trace file
```
cargo run --bin stateless-block-verifier --features="bin-deps" -- [--disable-checks] run-file --path testdata/mainnet_blocks/0x7ea4fb.json 
```

## Fetch and verify traces from Geth rpc
```
cargo run --bin stateless-block-verifier --features="bin-deps" -- [--disable-checks] run-rpc --url http://localhost:8545 --start-block latest
```
