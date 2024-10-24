//! stateful module
#[macro_use]
extern crate sbv;

use crate::pipeline::Fetcher;
use alloy::providers::{Provider, ReqwestProvider};
use alloy::rpc::types::Block;
use sbv::{
    core::{EvmExecutorBuilder, GenesisConfig, HardforkConfig},
    primitives::{
        alloy_primitives::ChainId,
        types::AlloyTransaction,
        zk_trie::{
            db::{kv::SledDb, NodeDb},
            hash::{key_hasher::NoCacheHasher, poseidon::Poseidon, ZkHash},
        },
    },
};
use sled::Tree;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

mod error;
/// pipeline
pub mod pipeline;
/// sanity check
pub mod sanity_check;
/// utils
pub mod utils;

pub use error::Error;

/// Result alias
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// Stateful Block Executor
#[derive(Debug)]
pub struct StatefulBlockExecutor {
    db: sled::Db,
    provider: ReqwestProvider,

    chain_id: ChainId,
    genesis_config: GenesisConfig,
    hardfork_config: HardforkConfig,

    metadata: Metadata,

    history_db: HistoryDb,
    code_db: SledDb,
    zktrie_db: NodeDb<SledDb>,

    pipeline_rx: tokio::sync::mpsc::Receiver<Block<AlloyTransaction>>,
    shutdown: Arc<AtomicBool>,
}

impl StatefulBlockExecutor {
    /// Create a new stateful block executor
    pub async fn new(db: sled::Db, provider: ReqwestProvider) -> Result<Self> {
        let chain_id = retry_if_transport_error!(provider.get_chain_id())?;
        dev_info!("chain_id: {chain_id}");

        let genesis_config = GenesisConfig::default_from_chain_id(chain_id);
        dev_info!("genesis_config: {genesis_config:?}");
        let hardfork_config = HardforkConfig::default_from_chain_id(chain_id);
        dev_info!("hardfork_config: {hardfork_config:?}");

        let metadata = Metadata::open(&db, chain_id)?;
        let history_db = metadata.open_history_db(&db)?;

        let mut code_db = metadata.open_code_db(&db)?;
        let mut zktrie_db = metadata.open_zktrie_db(&db)?;
        if metadata.needs_init() {
            genesis_config.init_code_db(&mut code_db)?;
            let zktrie =
                genesis_config.init_zktrie::<Poseidon, _, _>(&mut zktrie_db, NoCacheHasher)?;
            history_db.set_block_storage_root(0, *zktrie.root().unwrap_ref())?;
        }

        let shutdown = Arc::new(AtomicBool::new(false));

        let pipeline_rx = Fetcher::spawn(
            20,
            provider.clone(),
            genesis_config.coinbase(),
            chain_id,
            metadata.latest_block_number() + 1,
            shutdown.clone(),
        );

        Ok(Self {
            db,
            provider,
            chain_id,
            genesis_config,
            hardfork_config,
            metadata,
            history_db,
            code_db,
            zktrie_db,
            pipeline_rx,
            shutdown,
        })
    }

    /// Execute a block
    fn execute_block(&mut self, block: &Block<AlloyTransaction>) -> Result<()> {
        if self.metadata.latest_block_number() + 1 != block.header.number {
            return Err(Error::ExpectedSequentialBlock);
        }

        let block_number = block.header.number;
        let storage_root_before = self
            .history_db
            .get_block_storage_root(block_number - 1)?
            .expect("prev block storage root not found");

        let mut evm = EvmExecutorBuilder::new(&mut self.code_db, &mut self.zktrie_db)
            .chain_id(self.chain_id)
            .hardfork_config(self.hardfork_config)
            .build(storage_root_before)?;
        evm.handle_block(&block)?;
        let storage_root_after = evm.commit_changes()?;
        self.history_db
            .set_block_storage_root(block_number, storage_root_after)?;

        if block.header.state_root != storage_root_after {
            return Err(Error::PostStateRootMismatch);
        }
        self.metadata.set_latest_block_number(block_number)?;
        Ok(())
    }

    /// Run forever
    pub async fn run(&mut self) -> Result<()> {
        let mut blocks_handled = 0;
        let mut last_time = std::time::Instant::now();
        loop {
            let fetch_start = std::time::Instant::now();
            match self.pipeline_rx.recv().await {
                Some(block) => {
                    let block_number = block.header.number;

                    let elapsed = fetch_start.elapsed();
                    if elapsed > std::time::Duration::from_millis(500) {
                        dev_warn!("receive block#{block_number} from pipeline took {elapsed:?}, is the provider overloaded?");
                    }

                    #[cfg(debug_assertions)]
                    {
                        sanity_check::check_stateless(
                            &self.provider,
                            self.chain_id,
                            self.hardfork_config,
                            self.history_db
                                .get_block_storage_root(block_number - 1)?
                                .unwrap(),
                            &block,
                        )
                        .await?;
                    }

                    let execute_start = std::time::Instant::now();
                    match self.execute_block(&block) {
                        Ok(_) => {
                            dev_trace!(
                                "block#{block_number} stateful check ok in {:?}",
                                execute_start.elapsed()
                            );

                            blocks_handled += 1;
                            if last_time.elapsed() > std::time::Duration::from_secs(10) {
                                let elapsed = last_time.elapsed();
                                last_time = std::time::Instant::now();
                                let blocks_per_sec = blocks_handled as f64 / elapsed.as_secs_f64();
                                dev_info!(
                                    "handled {blocks_handled} blocks in {elapsed:.2?}, {blocks_per_sec:.2} blocks/s, latest block: {block_number}",
                                );
                                if let Ok(latest) = self.provider.get_block_number().await {
                                    let estimate_hours = (latest - block_number) as f64
                                        / blocks_per_sec
                                        / 60.0
                                        / 60.0;
                                    dev_info!("estimate time to catch up: {estimate_hours:.2?}h",);
                                }

                                blocks_handled = 0;
                            }
                        }
                        Err(e) => {
                            dev_error!("block#{block_number} stateful check failed: {e}");
                            self.shutdown
                                .store(true, std::sync::atomic::Ordering::SeqCst);
                            return Err(e);
                        }
                    }
                }
                None => {
                    dev_error!("pipeline shutdown");
                    return Err(Error::PipelineShutdown);
                }
            }
        }
    }

    /// Shutdown the executor
    #[inline(always)]
    pub fn shutdown(&self) {
        self.shutdown
            .store(true, std::sync::atomic::Ordering::SeqCst);
    }

    /// Get the sled db
    #[inline(always)]
    pub fn db(&self) -> &sled::Db {
        &self.db
    }

    /// Get the provider
    #[inline(always)]
    pub fn provider(&self) -> ReqwestProvider {
        self.provider.clone()
    }

    /// Get the chain id
    #[inline(always)]
    pub fn chain_id(&self) -> ChainId {
        self.chain_id
    }

    /// Get the genesis config
    #[inline(always)]
    pub fn genesis_config(&self) -> &GenesisConfig {
        &self.genesis_config
    }

    /// Get the hardfork config
    #[inline(always)]
    pub fn hardfork_config(&self) -> &HardforkConfig {
        &self.hardfork_config
    }

    /// Get the metadata
    #[inline(always)]
    pub fn metadata(&self) -> &Metadata {
        &self.metadata
    }

    /// Get the history db
    #[inline(always)]
    pub fn history_db(&self) -> &HistoryDb {
        &self.history_db
    }
}

/// Metadata
#[derive(Debug)]
pub struct Metadata {
    db: Tree,
    chain_id: u64,
    latest_block_number: u64,
}

impl Metadata {
    /// Open metadata from sled db
    pub fn open(db: &sled::Db, chain_id: u64) -> Result<Self> {
        let db = db.open_tree(format!("metadata_chain_{chain_id}"))?;

        let latest_block_number = db
            .get("latest_block_number")?
            .map(|v| u64::from_le_bytes(v.as_ref().try_into().unwrap()))
            .unwrap_or_default();

        Ok(Self {
            db,
            chain_id,
            latest_block_number,
        })
    }

    /// Set the latest block number
    #[inline(always)]
    pub fn set_latest_block_number(&mut self, block_number: u64) -> Result<()> {
        self.db
            .insert("latest_block_number", &block_number.to_le_bytes())?;
        self.latest_block_number = block_number;
        Ok(())
    }

    /// Get the latest block number
    #[inline(always)]
    pub fn latest_block_number(&self) -> u64 {
        self.latest_block_number
    }

    /// Check if the db needs initialization
    #[inline(always)]
    pub fn needs_init(&self) -> bool {
        self.latest_block_number == 0
    }

    /// Open the code db
    #[inline(always)]
    pub fn open_code_db(&self, db: &sled::Db) -> Result<SledDb> {
        Ok(SledDb::new(
            true,
            db.open_tree(format!("code_db_chain_{}", self.chain_id))?,
        ))
    }

    /// Open the zktrie db
    #[inline(always)]
    pub fn open_zktrie_db(&self, db: &sled::Db) -> Result<NodeDb<SledDb>> {
        Ok(NodeDb::new(SledDb::new(
            true,
            db.open_tree(format!("zktrie_db_chain_{}", self.chain_id))?,
        )))
    }

    /// Open the history db
    #[inline(always)]
    pub fn open_history_db(&self, db: &sled::Db) -> Result<HistoryDb> {
        Ok(HistoryDb {
            db: db.open_tree(format!("history_db_chain_{}", self.chain_id))?,
        })
    }
}

/// History database
#[derive(Debug)]
pub struct HistoryDb {
    db: Tree,
}

impl HistoryDb {
    /// Set the block storage root
    #[inline(always)]
    pub fn set_block_storage_root(&self, block_number: u64, storage_root: ZkHash) -> Result<()> {
        self.db
            .insert(block_number.to_le_bytes(), &storage_root.0)?;
        Ok(())
    }

    /// Get the block storage root
    #[inline(always)]
    pub fn get_block_storage_root(&self, block_number: u64) -> Result<Option<ZkHash>> {
        Ok(self
            .db
            .get(block_number.to_le_bytes())?
            .map(|v| ZkHash::from_slice(v.as_ref())))
    }
}
