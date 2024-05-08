use bus_mapping::circuit_input_builder::{CircuitInputBuilder, CircuitsParams};
use eth_types::l2_types::BlockTrace;
use eth_types::{ToBigEndian, ToWord, H160, U256};
use halo2_proofs::halo2curves::bn256::Fr;
use stateless_block_verifier::EvmExecutor;
use std::collections::HashMap;
use zkevm_circuits::table::AccountFieldTag;
use zkevm_circuits::witness::{block_convert, Block, Key};

pub fn verify(l2_trace: BlockTrace, disable_checks: bool, log_error: bool) -> bool {
    trace!("{:#?}", l2_trace);
    let root_after = l2_trace.storage_trace.root_after.to_word();
    info!("Root after in trace: {:x}", root_after);

    let now = std::time::Instant::now();

    #[cfg(feature = "profiling")]
    let guard = pprof::ProfilerGuardBuilder::default()
        .frequency(1000)
        .blocklist(&["libc", "libgcc", "pthread", "vdso"])
        .build()
        .unwrap();

    let mut executor = EvmExecutor::new(&l2_trace, disable_checks);
    let revm_root_after = executor.handle_block(&l2_trace).to_word();

    #[cfg(feature = "profiling")]
    if let Ok(report) = guard.report().build() {
        let dir = std::env::temp_dir()
            .join(env!("CARGO_PKG_NAME"))
            .join("profiling");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(format!(
            "block-{}.svg",
            l2_trace.header.number.unwrap().as_u64()
        ));
        let file = std::fs::File::create(&path).unwrap();
        report.flamegraph(file).unwrap();
        info!("Profiling report saved to: {:?}", path);
    }

    info!("Root after in revm: {:x}", revm_root_after);
    let elapsed = now.elapsed();

    let mut builder = CircuitInputBuilder::new_from_l2_trace(
        CircuitsParams {
            max_rws: 1000000,
            max_txs: 100,
            ..Default::default()
        },
        l2_trace,
        false,
        false,
    )
    .unwrap();
    builder.finalize_building().unwrap();
    let mut block: Block<Fr> = block_convert(&builder.block, &builder.code_db).unwrap();
    block
        .mpt_updates
        .fill_state_roots(builder.mpt_init_state.as_ref().unwrap());

    let dirty = executor.db.cache;
    let sdb = executor.db.sdb;
    let mut baseline = HashMap::new();
    for (key, update) in block.mpt_updates.updates {
        match key {
            Key::Account { address, field_tag } => {
                let acc = baseline.entry(address).or_insert_with(|| {
                    let (_exist, acc) = sdb.get_account(&address);
                    acc.clone()
                });
                match field_tag {
                    AccountFieldTag::Nonce => acc.nonce = update.new_value,
                    AccountFieldTag::Balance => acc.balance = update.new_value,
                    AccountFieldTag::KeccakCodeHash => {
                        acc.keccak_code_hash = update.new_value.to_be_bytes().into()
                    }
                    AccountFieldTag::CodeHash => {
                        acc.code_hash = update.new_value.to_be_bytes().into()
                    }
                    AccountFieldTag::CodeSize => acc.code_size = update.new_value,
                    AccountFieldTag::NonExisting => {}
                }
            }
            Key::AccountStorage {
                address,
                storage_key,
                exists,
                ..
            } => {
                let acc = baseline.entry(address).or_insert_with(|| {
                    let (_exist, acc) = sdb.get_account(&address);
                    acc.clone()
                });
                if exists {
                    acc.storage.insert(storage_key, update.new_value);
                } else {
                    acc.storage.remove(&storage_key);
                }
            }
        }
    }

    for (addr, acc) in baseline.iter() {
        let local_acc = dirty.get(&revm::primitives::Address::new(addr.0));
        if local_acc.is_none() {
            error!("Account not found in dirty: {:?}", addr);
            continue;
        }
        let local_acc = local_acc.unwrap();
        if local_acc.info.nonce != acc.nonce.as_u64() {
            error!(
                "Nonce mismatch for account: {:?}, baseline: {:?}, dirty: {:?}",
                addr, acc.nonce, local_acc.info.nonce
            );
        }
        if local_acc.info.balance.to_be_bytes() != acc.balance.to_be_bytes() {
            error!(
                "Balance mismatch for account: {:?}, baseline: {:?}, dirty: {:?}",
                addr, acc.balance, local_acc.info.balance
            );
        }
        if local_acc.info.code_hash.0 != acc.code_hash.0 {
            error!(
                "Code hash mismatch for account: {:?}, baseline: {:?}, dirty: {:?}",
                addr, acc.code_hash, local_acc.info.code_hash
            );
        }
        if local_acc.info.keccak_code_hash.0 != acc.keccak_code_hash.0 {
            error!(
                "Keccak code hash mismatch for account: {:?}, baseline: {:?}, dirty: {:?}",
                addr, acc.keccak_code_hash, local_acc.info.keccak_code_hash
            );
        }
        for (k, v) in &acc.storage {
            let local_v = local_acc
                .storage
                .get(&revm::primitives::U256::from_limbs(k.0));
            if local_v.is_none() {
                error!("Storage key not found in dirty: {:?}, key: {:?}", addr, k);
                continue;
            }
            let local_v = local_v.unwrap();
            if local_v.present_value.to_be_bytes() != v.to_be_bytes() {
                error!("Storage value mismatch for account: {:?}, key: {:?}, baseline: {:?}, dirty: {:?}", addr, k, v, local_v);
            }
        }
    }

    for (addr, acc) in dirty.iter() {
        let baseline_acc = baseline.get(&H160::from(addr.0 .0));
        if baseline_acc.is_none() {
            error!("Account not found in baseline: {:?}", addr);
            continue;
        }
        let baseline_acc = baseline_acc.unwrap();
        if acc.info.nonce != baseline_acc.nonce.as_u64() {
            error!(
                "Nonce mismatch for account: {:?}, baseline: {:?}, dirty: {:?}",
                addr, baseline_acc.nonce, acc.info.nonce
            );
        }
        if acc.info.balance.to_be_bytes() != baseline_acc.balance.to_be_bytes() {
            error!(
                "Balance mismatch for account: {:?}, baseline: {:?}, dirty: {:?}",
                addr, baseline_acc.balance, acc.info.balance
            );
        }
        if acc.info.code_hash.0 != baseline_acc.code_hash.0 {
            error!(
                "Code hash mismatch for account: {:?}, baseline: {:?}, dirty: {:?}",
                addr, baseline_acc.code_hash, acc.info.code_hash
            );
        }
        if acc.info.keccak_code_hash.0 != baseline_acc.keccak_code_hash.0 {
            error!(
                "Keccak code hash mismatch for account: {:?}, baseline: {:?}, dirty: {:?}",
                addr, baseline_acc.keccak_code_hash, acc.info.keccak_code_hash
            );
        }
        for (k, v) in acc.storage.iter() {
            let baseline_v = baseline_acc
                .storage
                .get(&U256::from_big_endian(&k.to_be_bytes::<32>()));
            if baseline_v.is_none() {
                error!(
                    "Storage key not found in baseline: {:?}, key: {:?}",
                    addr, k
                );
                continue;
            }
            let baseline_v = baseline_v.unwrap();
            if baseline_v.to_be_bytes() != v.present_value.to_be_bytes() {
                error!("Storage value mismatch for account: {:?}, key: {:?}, baseline: {:?}, dirty: {:?}", addr, k, baseline_v, v);
            }
        }
    }

    if root_after != revm_root_after {
        error!("Root mismatch");
        if !log_error {
            std::process::exit(1);
        }
        return false;
    }
    info!("Root matches in: {} ms", elapsed.as_millis());
    true
}
