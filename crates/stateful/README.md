# Stateful Block Verifier

With the `sled` feature enabled, the zkTrie can persist storage on disk,
so we can run the verifier in stateful mode, which means we do not need the
zktrie proofs to execute the transaction since we have all storage available.

It runs much faster than stateless mode since `eth_getBlockByNumber` is faster 
than `scroll_getBlockTraceByNumberOrHash`.

## How to run

*Note: In debug mode, it runs both stateless and stateful to perform, performed sanity
check, which requires the rpc endpoint supports `scroll_getBlockTraceByNumberOrHash`

```bash
cargo run [--release] --bin stateful-block-verifier -- --db <sled-db-path> --url <rpc-url>
```
