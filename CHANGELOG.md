# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Add a new struct `LegacyStorageTrace` to support legacy storage trace support
  ([#58](https://github.com/scroll-tech/stateless-block-verifier/pull/58))
- Add a cli flag `--legacy` to enable support of legacy rpc node
  ([#58](https://github.com/scroll-tech/stateless-block-verifier/pull/58))

### Changed

- `flatten_proofs` in `StorageTrace` is changed from `Vec<(B256, Bytes)` to `Vec<Bytes>`
  since the node hash will be recalculated when adding to zktrie
  ([#58](https://github.com/scroll-tech/stateless-block-verifier/pull/58))
- `BlockTrace` now has a generic parameter `S` for the storage trace type, default to `StorageTrace`
  ([#58](https://github.com/scroll-tech/stateless-block-verifier/pull/58))


## [2.0.0] - 2024-09-04

### Added

- Add rkyv support, support zero-copy deserialization
- Add an `ordered-db` feature gate to ensure the order when iterating over the database
- Add a new sp1 cycle tracker macro `cycle_track!` which returns wrapped expression
- Add chunk mode to work in chunk mode ([#29](https://github.com/scroll-tech/stateless-block-verifier/pull/29))
- Add openmetrics support and a `metrics` feature gate ([#33](https://github.com/scroll-tech/stateless-block-verifier/pull/33))
- Add zktrie lazy commitment ([#39](https://github.com/scroll-tech/stateless-block-verifier/pull/39))

### Fixed

- revm v40 upgrade cause `EXTCODEHASH` loads code to check if it's EOF, fixed by [revm#17](https://github.com/scroll-tech/revm/pull/17/files)
- The tx hash is ignored and the tx hash is calculated from the tx body instead
- The `from` field of the transaction trace is ignored if it's not l1 msg, the `tx.from` will be recovered from the signature instead
- `BLOBHASH` & `BLOBBASEFEE` opcodes were accidentally enabled in CURIE ([#40](https://github.com/scroll-tech/stateless-block-verifier/pull/40))

### Changed

- Code database now use the keccak code hash as key, instead of the poseidon hash of the code ([#20](https://github.com/scroll-tech/stateless-block-verifier/pull/20))
- Remove StateDB, direct query the zktrie db ([#38](https://github.com/scroll-tech/stateless-block-verifier/pull/38))
- Dependency of `eth-types` is removed ([#43](https://github.com/scroll-tech/stateless-block-verifier/pull/43))
- Dependency of `mpt-zktrie` is removed ([#45](https://github.com/scroll-tech/stateless-block-verifier/pull/45))
- Dependency of `ethers-rs` is removed ([#46](https://github.com/scroll-tech/stateless-block-verifier/pull/46))

### Removed

- `post_check` is removed as long as the command line argument `--disable-check`
- Support of legacy trace format is removed, only support the trace with codes and flatten proofs now.

## [1.0.0] - 2024-07-26

### Added

- Initial release

[unreleased]: https://github.com/scroll-tech/stateless-block-verifier/compare/2.0.0...HEAD
[2.0.0]: https://github.com/scroll-tech/stateless-block-verifier/compare/v1.0.0...v2.0.0
[1.0.0]: https://github.com/scroll-tech/stateless-block-verifier/releases/tag/v1.0.0