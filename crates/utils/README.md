# Command Line Utils

This crate provides a set of utilities for working with sbv.

## Witness

### Dump witness from RPC

```
$ sbv-utils witness dump --help
Dump a witness from reth RPC

Usage: sbv-utils witness dump [OPTIONS] --block <BLOCK>

Options:
      --rpc <RPC>              URL to the RPC server [default: http://localhost:8545]
      --block <BLOCK>          Block number
      --ancestors <ANCESTORS>  Ancestor blocks [default: 256]
      --out-dir <OUT_DIR>      Output directory
      --json                   Output json
      --rkyv                   Output rkyv
      --max-retry <MAX_RETRY>  Retry Backoff: maximum number of retries [default: 10]
      --backoff <BACKOFF>      Retry Backoff: backoff duration in milliseconds [default: 100]
      --cups <CUPS>            Retry Backoff: compute units per second [default: 100]
  -h, --help                   Print help
```

e.g. dump block#2971844 from RPC server running at `http://localhost:58545` and output both json and rkyv files to `./witness` directory.
```sh
sbv-utils witness dump --rpc http://localhost:58545 --block 2971844 --out-dir ./witness --json --rkyv
```

> **Fetch 256 ancestors takes too long?**
> 
> You can reduce the number of ancestors to fetch by using `--ancestors` option. e.g. `--ancestors 1`.
> Then run the `stateless-block-verifier` see where it fails and increase the number of ancestors accordingly.

### Convert JSON witness to rkyv

```
$ sbv-utils witness rkyv --help
Convert a witness json to rkyv

Usage: sbv-utils witness rkyv [OPTIONS] [WITNESSES]...

Arguments:
  [WITNESSES]...  Path to the witness json file

Options:
      --out-dir <OUT_DIR>  Output directory
  -h, --help               Print help
```

e.g. convert `./testdata/holesky_witness/*.json` to rkyv format.
```
$ sbv-utils witness rkyv ./testdata/holesky_witness/*.json
Converted ./testdata/holesky_witness/2971844.json to ./testdata/holesky_witness/2971844.rkyv
Converted ./testdata/holesky_witness/2971845.json to ./testdata/holesky_witness/2971845.rkyv
Converted ./testdata/holesky_witness/2971846.json to ./testdata/holesky_witness/2971846.rkyv
Converted ./testdata/holesky_witness/2971847.json to ./testdata/holesky_witness/2971847.rkyv
```