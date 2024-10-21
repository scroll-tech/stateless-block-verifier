//! This is a simple example of how to use the stateful executor to verify the state transition of the L2 chain.
#[macro_use]
extern crate sbv;

use alloy::providers::{Provider, ProviderBuilder};
use alloy::rpc::types::BlockTransactions;
use clap::Parser;
use revm::primitives::{BlockEnv, TxEnv};
use sbv::{
    core::{EvmExecutorBuilder, GenesisConfig, HardforkConfig},
    primitives::zk_trie::hash::{key_hasher::NoCacheHasher, poseidon::Poseidon},
};
use std::path::PathBuf;
use url::Url;

use sbv::core::VerificationError;
use sbv::primitives::types::{BlockTrace, LegacyStorageTrace};
use sbv::primitives::zk_trie::db::kv::{HashMapDb, SledDb};
use sbv::primitives::zk_trie::db::NodeDb;
use sbv::primitives::{Block, Transaction, TxTrace, U256};
#[cfg(feature = "dev")]
use tracing_subscriber::EnvFilter;

#[derive(Parser)]
struct Cli {
    /// RPC URL
    #[arg(short, long, default_value = "http://localhost:8545")]
    url: Url,
    /// Path to the sled database
    #[arg(short, long)]
    db: PathBuf,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    #[cfg(feature = "dev")]
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let cmd = Cli::parse();

    let db = sled::open(cmd.db)?;

    let mut code_db = SledDb::new(true, db.open_tree("code_db")?);
    let mut zktrie_db = NodeDb::new(SledDb::new(true, db.open_tree("zk_trie")?));

    let provider = ProviderBuilder::new().on_http(cmd.url);
    let chain_id = provider.get_chain_id().await?;
    let hardfork_config = HardforkConfig::default_from_chain_id(chain_id);
    let genesis_config = GenesisConfig::default_from_chain_id(chain_id);

    genesis_config.init_code_db(&mut code_db)?;
    let mut storage_root = {
        let zktrie = genesis_config.init_zktrie::<Poseidon, _, _>(&mut zktrie_db, NoCacheHasher)?;
        *zktrie.root().unwrap_ref()
    };

    for i in 1..100000u64 {
        let mut evm = EvmExecutorBuilder::new(&mut code_db, &mut zktrie_db)
            .chain_id(chain_id)
            .hardfork_config(hardfork_config)
            .build(storage_root)?;

        let l2_trace = provider
            .raw_request::<_, BlockTrace<LegacyStorageTrace>>(
                "scroll_getBlockTraceByNumberOrHash".into(),
                (format!("0x{:x}", i),),
            )
            .await?;
        let root_before = l2_trace.root_before();
        let root_after = l2_trace.root_after();
        dev_debug!(
            "block#{i} root_before={} current_storage_root={}",
            root_before,
            storage_root
        );
        assert_eq!(l2_trace.root_before(), storage_root);

        // check stateless is ok
        {
            let mut code_db = HashMapDb::default();
            let mut zktrie_db = NodeDb::new(HashMapDb::default());
            l2_trace.build_zktrie_db(&mut zktrie_db).unwrap();
            let mut executor = EvmExecutorBuilder::new(&mut code_db, &mut zktrie_db)
                .hardfork_config(hardfork_config)
                .chain_id(chain_id)
                .build(root_before)
                .unwrap();
            executor.insert_codes(&l2_trace).unwrap();
            executor.handle_block(&l2_trace).unwrap();
            let revm_root_after = executor.commit_changes().unwrap();
            assert_eq!(root_after, revm_root_after);
        }

        let mut block = provider.get_block_by_number(i.into(), true).await?.unwrap();
        block.header.miner = l2_trace.coinbase();
        if let BlockTransactions::Full(ref mut txs) = block.transactions {
            for tx in txs.iter_mut() {
                if tx.transaction_type.unwrap_or(0) == 0
                    && tx.signature.unwrap().v.to::<u64>() >= 35
                {
                    tx.chain_id = Some(chain_id);
                }
            }
        }
        // sanity check
        {
            let trace_block_env = BlockEnv {
                number: U256::from_limbs([l2_trace.number(), 0, 0, 0]),
                coinbase: l2_trace.coinbase(),
                timestamp: l2_trace.timestamp(),
                gas_limit: l2_trace.gas_limit(),
                basefee: l2_trace.base_fee_per_gas().unwrap_or_default(),
                difficulty: l2_trace.difficulty(),
                prevrandao: l2_trace.prevrandao(),
                blob_excess_gas_and_price: None,
            };
            let block_block_env = BlockEnv {
                number: U256::from_limbs([block.number(), 0, 0, 0]),
                coinbase: block.coinbase(),
                timestamp: block.timestamp(),
                gas_limit: block.gas_limit(),
                basefee: block.base_fee_per_gas().unwrap_or_default(),
                difficulty: block.difficulty(),
                prevrandao: block.prevrandao(),
                blob_excess_gas_and_price: None,
            };
            assert_eq!(trace_block_env, block_block_env, "block_env mismatch");
            for (i, (trace_tx, block_tx)) in l2_trace
                .transactions
                .iter()
                .zip(block.transactions.as_transactions().unwrap().iter())
                .enumerate()
            {
                dev_debug!(
                    "#{i}: is_l1_tx={:?} {:?}",
                    trace_tx.is_l1_tx(),
                    block_tx.is_l1_tx()
                );
                let trace_tx = trace_tx.try_build_typed_tx().unwrap();
                let block_tx = block_tx.try_build_typed_tx().unwrap();
                assert_eq!(trace_tx, block_tx, "tx mismatch {block_tx:?}");
                let trace_tx_signer = trace_tx.get_or_recover_signer().unwrap();
                let block_tx_signer = block_tx.get_or_recover_signer().unwrap();
                assert_eq!(trace_tx_signer, block_tx_signer, "tx signer mismatch");
                let trace_gas_limit = trace_tx.gas_limit();
                let block_gas_limit = block_tx.gas_limit();
                assert_eq!(trace_gas_limit, block_gas_limit, "tx gas limit mismatch");
                let trace_gas_price = trace_tx
                    .effective_gas_price(l2_trace.base_fee_per_gas().unwrap_or_default().to())
                    .map(U256::from);
                let block_gas_price = block_tx
                    .effective_gas_price(l2_trace.base_fee_per_gas().unwrap_or_default().to())
                    .map(U256::from);
                assert_eq!(trace_gas_price, block_gas_price, "tx gas price mismatch");
                assert_eq!(trace_tx.to(), block_tx.to(), "tx transact_to mismatch");
                assert_eq!(trace_tx.value(), block_tx.value(), "tx value mismatch");
                assert_eq!(trace_tx.data(), block_tx.data(), "tx data mismatch");
                assert_eq!(
                    trace_tx.is_l1_msg(),
                    block_tx.is_l1_msg(),
                    "tx is_l1_msg mismatch"
                );
                assert_eq!(trace_tx.nonce(), block_tx.nonce(), "tx nonce mismatch");
                assert_eq!(
                    trace_tx.chain_id(),
                    block_tx.chain_id(),
                    "tx chain_id mismatch"
                );
                assert_eq!(
                    trace_tx.access_list(),
                    block_tx.access_list(),
                    "tx access_list mismatch"
                );
                assert_eq!(
                    trace_tx.max_priority_fee_per_gas(),
                    block_tx.max_priority_fee_per_gas(),
                    "tx max_priority_fee_per_gas mismatch"
                );
            }
        }

        evm.handle_block(&l2_trace)?;
        storage_root = evm.commit_changes()?;
        assert_eq!(storage_root, root_after);
    }

    Ok(())
}
