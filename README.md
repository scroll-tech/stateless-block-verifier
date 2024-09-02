# Stateless Block Verifier

This project provides tools for stateless verification of blocks with mpt state roots.

# Example

## Run and verify a trace file
```
cargo run --package stateless-block-verifier -- [--disable-checks] run-file testdata/mainnet_blocks/0x7ea4fb.json 
```

## Run Chunk mode trace files
```
cargo run --package stateless-block-verifier -- run-file --chunk-mode testdata/mainnet_blocks/837*
```

## Fetch and verify traces from Geth rpc
```
cargo run --package stateless-block-verifier -- [--disable-checks] run-rpc --url http://localhost:8545 --start-block latest
```

