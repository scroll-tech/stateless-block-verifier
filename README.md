# Stateless Block Verifier

This project provides tools for stateless verification of blocks with mpt state roots.

# Example

## Run and verify a trace file
```
cargo run --package stateless-block-verifier -- run-file testdata/holesky_witness/2971844.json
```

## Run Chunk mode trace files
```
cargo run --package stateless-block-verifier -- run-file --chunk-mode testdata/holesky_witness/297184*.json
```

