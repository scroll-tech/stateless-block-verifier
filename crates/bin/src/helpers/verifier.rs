use crate::helpers::dump::dump_bundle_state;
use anyhow::anyhow;
#[cfg(feature = "dev")]
use sbv::helpers::tracing;
use sbv::{
    core::{EvmDatabase, EvmExecutor, VerificationError},
    kv::nohash::NoHashMap,
    primitives::{
        chainspec::{Chain, ChainSpec, get_chain_spec_or_build},
        ext::{BlockWitnessExt, BlockWitnessRethExt},
    },
    trie::BlockWitnessTrieExt,
};
use std::{
    collections::BTreeMap,
    env,
    panic::{UnwindSafe, catch_unwind},
    sync::Arc,
};

#[cfg_attr(feature = "dev", tracing::instrument(skip_all, fields(block_number = %witness.number()), err))]
pub fn verify_catch_panics<
    T: BlockWitnessRethExt + BlockWitnessTrieExt + BlockWitnessExt + UnwindSafe,
>(
    witness: T,
) -> anyhow::Result<u64> {
    catch_unwind(|| verify(witness))
        .map_err(|e| {
            e.downcast_ref::<&str>()
                .map(|s| anyhow!("task panics with: {s}"))
                .or_else(|| {
                    e.downcast_ref::<String>()
                        .map(|s| anyhow!("task panics with: {s}"))
                })
                .unwrap_or_else(|| anyhow!("task panics"))
        })
        .and_then(|r| r.map_err(anyhow::Error::from))
}

pub fn get_chain_spec(chain_id: u64) -> Arc<ChainSpec> {
    get_chain_spec_or_build(Chain::from_id(chain_id), |_spec| {
        #[cfg(feature = "scroll")]
        {
            use sbv::primitives::hardforks::{ForkCondition, ScrollHardfork};
            _spec
                .inner
                .hardforks
                .insert(ScrollHardfork::EuclidV2, ForkCondition::Timestamp(0));
            _spec
                .inner
                .hardforks
                .insert(ScrollHardfork::Feynman, ForkCondition::Timestamp(0));
        }
    })
}

#[cfg_attr(feature = "dev", tracing::instrument(skip_all, fields(block_number = %witness.number()), err))]
fn verify<T: BlockWitnessRethExt + BlockWitnessTrieExt + BlockWitnessExt>(
    witness: T,
) -> Result<u64, VerificationError> {
    dev_trace!("{witness:#?}");

    let chain_spec = get_chain_spec(witness.chain_id());

    let mut code_db = NoHashMap::default();
    witness.import_codes(&mut code_db);
    let mut nodes_provider = NoHashMap::default();
    witness.import_nodes(&mut nodes_provider).unwrap();
    #[cfg(not(feature = "scroll"))]
    let block_hashes = {
        let mut block_hashes = NoHashMap::default();
        witness.import_block_hashes(&mut block_hashes);
        block_hashes
    };
    #[cfg(feature = "scroll")]
    let block_hashes = &sbv::kv::null::NullProvider;
    let mut db = EvmDatabase::new_from_root(
        code_db,
        witness.pre_state_root(),
        &nodes_provider,
        &block_hashes,
    )?;

    let block = witness.build_reth_block()?;

    #[cfg(not(feature = "scroll"))]
    let executor = EvmExecutor::new(chain_spec, &db, &block);
    #[cfg(feature = "scroll")]
    let executor = EvmExecutor::new(chain_spec, &db, &block, None::<Vec<sbv::primitives::U256>>);

    let output = executor.execute().inspect_err(|_e| {
        dev_error!(
            "Error occurs when executing block #{}: {_e:?}",
            block.number
        );
    })?;

    db.update(
        &nodes_provider,
        BTreeMap::from_iter(output.state.state.clone()).iter(),
    )?;
    let post_state_root = db.commit_changes();

    if block.state_root != post_state_root {
        dev_error!(
            "Block #{} root mismatch: root after in trace = {:x}, root after in reth = {:x}",
            block.number,
            block.state_root,
            post_state_root
        );

        let dump_dir =
            env::temp_dir()
                .join("dumps")
                .join(format!("{}-{}", witness.chain_id(), block.number));
        dump_bundle_state(&output.state, &dump_dir)
            .inspect(|_| {
                dev_info!("Dumped bundle state to: {}", dump_dir.display());
            })
            .inspect_err(|_e| {
                dev_error!(
                    "Failed to dump bundle state to {}: {_e}",
                    dump_dir.display(),
                );
            })
            .ok();

        return Err(VerificationError::root_mismatch(
            block.state_root,
            post_state_root,
        ));
    }
    dev_info!("Block #{} verified successfully", block.number);

    Ok(output.gas_used)
}
