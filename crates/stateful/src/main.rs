#[macro_use]
extern crate sbv;

use alloy::primitives::address;
use alloy::providers::{Provider, ProviderBuilder};
use alloy::rpc::types::BlockTransactions;
use clap::Parser;
use sbv::{
    core::{EvmExecutorBuilder, GenesisConfig, HardforkConfig},
    primitives::zk_trie::{
        db::SledDb,
        hash::{key_hasher::NoCacheHasher, poseidon::Poseidon},
        trie::ZkTrie,
    },
};
use std::path::PathBuf;
use url::Url;

use sbv::primitives::types::BlockTrace;
use sbv::primitives::Block;
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
    let zktrie_db = SledDb::new(true, db.open_tree("zk_trie")?);

    let mut zktrie = ZkTrie::<Poseidon, _, _>::new(zktrie_db.clone(), NoCacheHasher);

    let provider = ProviderBuilder::new().on_http(cmd.url);
    let chain_id = provider.get_chain_id().await?;
    let hardfork_config = HardforkConfig::default_from_chain_id(chain_id);
    let genesis_config = GenesisConfig::default_from_chain_id(chain_id);

    genesis_config.init_code_db(&mut code_db)?;
    genesis_config.init_zktrie(&mut zktrie)?;

    for i in 1..100000u64 {
        let l2_trace = provider
            .raw_request::<_, BlockTrace>(
                "scroll_getBlockTraceByNumberOrHash".into(),
                (
                    format!("0x{:x}", i),
                    serde_json::json!({
                        "ExcludeExecutionResults": true,
                        "ExcludeTxStorageTraces": true,
                        "StorageProofFormat": "flatten",
                        "FlattenProofsOnly": true
                    }),
                ),
            )
            .await?;

        let block = provider.get_block_by_number(i.into(), true).await?.unwrap();
        // if let BlockTransactions::Full(ref mut txs) = block.transactions {
        //     for tx in txs.iter_mut() {
        //         tx.chain_id = Some(chain_id);
        //     }
        // }

        let mut evm = EvmExecutorBuilder::new()
            .chain_id(chain_id)
            .hardfork_config(hardfork_config)
            .evm_db_from_root(
                *zktrie.root().unwrap_ref(),
                code_db.clone(),
                zktrie_db.clone(),
            )?
            .build();

        evm.handle_block(&block)?;
        let new_root = evm.commit_changes(code_db.clone(), zktrie_db.clone())?;
        assert_eq!(new_root, l2_trace.root_after());

        zktrie.gc()?;

        zktrie = ZkTrie::new_with_root(zktrie_db.clone(), NoCacheHasher, new_root)?;
    }

    Ok(())
}
