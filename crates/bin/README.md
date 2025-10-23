# Command Line Utils

This crate provides a set of utilities for working with sbv.

## Build

```shell
cargo build --bin sbv-cli --release --features dev,scroll
```

## Verify Witness

```
> $ ./target/release/sbv-cli run
Run and verify witness

Usage: sbv-cli run <COMMAND>

Commands:
  file  Run and verify a trace file
  rpc   Run and verify from RPC
  help  Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

### Verify witness files

Can be used to verify witness files dumped by `sbv-utils witness dump`.
Accepts multiple json files, requires blocks to be continuous if using `--chunk-mode`.

```
> $ ./target/release/sbv-cli run file --help
Run and verify a trace file

Usage: sbv-cli run file [OPTIONS] [PATH]...

Arguments:
  [PATH]...  Path to the witness file [default: witness.json]

Options:
  -c, --chunk-mode                                 Chunk mode
      --prev-msg-queue-hash <PREV_MSG_QUEUE_HASH>
  -h, --help                                       Print help
```

e.g. verify `./testdata/scroll/euclid_v2/*.json` files.
```
> $ ./target/release/sbv-cli run file ./testdata/scroll/euclid_v2/*.json
2025-03-31T05:15:50.357835Z  INFO run_witness{path=./testdata/scroll/euclid_v2/1.json}: sbv_cli::commands::run::file: verified
2025-03-31T05:15:50.369316Z  INFO run_witness{path=./testdata/scroll/euclid_v2/2.json}: sbv_cli::commands::run::file: verified
2025-03-31T05:15:50.372989Z  INFO run_witness{path=./testdata/scroll/euclid_v2/3.json}: sbv_cli::commands::run::file: verified
2025-03-31T05:15:50.376463Z  INFO run_witness{path=./testdata/scroll/euclid_v2/4.json}: sbv_cli::commands::run::file: verified
2025-03-31T05:15:50.379678Z  INFO run_witness{path=./testdata/scroll/euclid_v2/5.json}: sbv_cli::commands::run::file: verified
2025-03-31T05:15:50.382717Z  INFO run_witness{path=./testdata/scroll/euclid_v2/6.json}: sbv_cli::commands::run::file: verified
2025-03-31T05:15:50.386070Z  INFO run_witness{path=./testdata/scroll/euclid_v2/7.json}: sbv_cli::commands::run::file: verified
2025-03-31T05:15:50.388808Z  INFO run_witness{path=./testdata/scroll/euclid_v2/8.json}: sbv_cli::commands::run::file: verified
```

#### Continuous verify blocks from a rpc server

This will verify blocks from a rpc server, starting from the block number provided.
If it reaches the latest block, it will wait for the next block to be mined and verify it.

```
> $ ./target/release/sbv-cli run rpc --help
Run and verify from RPC

Usage: sbv-cli run rpc [OPTIONS] --start-block <START_BLOCK>

Options:
      --start-block <START_BLOCK>
          start block number
      --rpc <RPC>
          URL to the RPC server, defaults to localhost:8545
      --mainnet
          using mainnet default rpc url: https://euclid-l2-mpt.scroll.systems
      --sepolia
          using sepolia default rpc url: https://sepolia-rpc.scroll.io
      --max-concurrency <MAX_CONCURRENCY>
          Concurrency Limit: maximum number of concurrent requests [default: 10]
      --max-retry <MAX_RETRY>
          Retry Backoff: maximum number of retries [default: 10]
      --backoff <BACKOFF>
          Retry Backoff: backoff duration in milliseconds [default: 100]
      --cups <CUPS>
          Retry Backoff: compute units per second [default: 100]
  -h, --help
          Print help
```

e.g. verify blocks from 1 on scroll mainnet.
```
> $ ./target/release/sbv-cli run rpc --mainnet --start-block 1
2025-03-31T05:17:05.347763Z  INFO sbv_cli::helpers: Using RPC: https://euclid-l2-mpt.scroll.systems/
...
```

Using `ctrl-c` to stop fetching new blocks, and exit after all fetched blocks are verified.
Press `ctrl-c` again to exit immediately.

## Witness Operations

```
> $ ./target/release/sbv-cli witness help
Witness helpers

Usage: sbv-cli witness <COMMAND>

Commands:
  dump  Dump a witness from reth RPC
  rkyv  Convert a witness json to rkyv
  help  Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

### Dump witness from RPC

```
> $ ./target/release/sbv-cli witness dump --help
Dump a witness from reth RPC

Usage: sbv-cli witness dump [OPTIONS] --block <BLOCK>

Options:
      --block <BLOCK>
          Block number
      --out-dir <OUT_DIR>
          Output directory [default: /Users/hhq/workspace/stateless-block-verifier]
      --json
          Output json
      --rkyv
          Output rkyv
      --rpc <RPC>
          URL to the RPC server, defaults to localhost:8545
      --mainnet
          using mainnet default rpc url: https://euclid-l2-mpt.scroll.systems
      --sepolia
          using sepolia default rpc url: https://sepolia-rpc.scroll.io
      --max-concurrency <MAX_CONCURRENCY>
          Concurrency Limit: maximum number of concurrent requests [default: 10]
      --max-retry <MAX_RETRY>
          Retry Backoff: maximum number of retries [default: 10]
      --backoff <BACKOFF>
          Retry Backoff: backoff duration in milliseconds [default: 100]
      --cups <CUPS>
          Retry Backoff: compute units per second [default: 100]
  -h, --help
          Print help
```

e.g. dump block#2971844 from RPC server running at `http://localhost:58545` and output both json and rkyv files to `./witness` directory.
```sh
./target/release/sbv-cli witness dump --rpc http://localhost:58545 --block 2971844 --out-dir ./witness --json --rkyv
```
 
> #### Mainnet mode: Fetch 256 ancestors takes too long?
> You can reduce the number of ancestors to fetch by using `--ancestors` option. e.g. `--ancestors 1`.
> Then run the `stateless-block-verifier` see where it fails and increase the number of ancestors accordingly.

### Convert JSON witness to rkyv

```
> $ ./target/release/sbv-cli witness rkyv --help
Convert a witness json to rkyv

Usage: sbv-cli witness rkyv [OPTIONS] [WITNESSES]...

Arguments:
  [WITNESSES]...  Path to the witness json file

Options:
      --chunk              Make single chunk rkyv instead of multiple blocks
      --out-dir <OUT_DIR>  Output directory
  -h, --help               Print help
```

e.g. convert `./testdata/holesky_witness/*.json` to rkyv format.
```
> $ ./target/release/sbv-cli witness rkyv ./testdata/holesky_witness/*.json
Converted ./testdata/holesky_witness/2971844.json to ./testdata/holesky_witness/2971844.rkyv
Converted ./testdata/holesky_witness/2971845.json to ./testdata/holesky_witness/2971845.rkyv
Converted ./testdata/holesky_witness/2971846.json to ./testdata/holesky_witness/2971846.rkyv
Converted ./testdata/holesky_witness/2971847.json to ./testdata/holesky_witness/2971847.rkyv
```

e.g. build a 4 blocks chunk from `./testdata/holesky_witness/*.json` to rkyv format.
```
$ ./target/release/sbv-cli witness rkyv --chunk ./testdata/holesky_witness/*.json
Converted 4 witnesses to chunk testdata/holesky_witness/chunk-2971844-4.rkyv
```
