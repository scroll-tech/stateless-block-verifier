//! Rpc Extension

use crate::verifier::verify;
use alloy_provider::{Provider, network::primitives::BlockTransactionsKind};
use alloy_transport::{RpcError, TransportResult};
use sbv_primitives::{
    B256, Bytes,
    types::{
        BlockWitness, ExecutionWitness, Network,
        eips::{BlockId, BlockNumberOrTag},
    },
};

const MAX_AUTO_FIX_ATTEMPTS: usize = 100;

/// Options for [`dump_block_witness`](ProviderExt::dump_block_witness).
///
/// Defaults to:
/// - the latest block
/// - 256 ancestors (if `scroll` feature is not enabled)
/// - auto-fix enabled
#[derive(Debug, Copy, Clone)]
pub struct DumpBlockWitnessOptions {
    #[cfg(not(feature = "scroll"))]
    ancestors: usize,
    auto_fix: bool,
}

/// Extension trait for [`Provider`](Provider).
#[async_trait::async_trait]
pub trait ProviderExt: Provider<Network> {
    /// Get the execution witness for a block.
    async fn debug_execution_witness(
        &self,
        number: BlockNumberOrTag,
    ) -> TransportResult<ExecutionWitness> {
        self.client()
            .request::<_, ExecutionWitness>("debug_executionWitness", (number,))
            .await
    }

    /// Get the disk root for a block.
    #[cfg(feature = "scroll")]
    async fn scroll_disk_root(
        &self,
        number: BlockNumberOrTag,
    ) -> TransportResult<sbv_primitives::types::scroll::DiskRoot> {
        self.client()
            .request::<_, sbv_primitives::types::scroll::DiskRoot>("scroll_diskRoot", (number,))
            .await
    }

    /// Dump the block witness for a block.
    async fn dump_block_witness<T: Into<BlockId> + Send + Sync>(
        &self,
        block: T,
        options: DumpBlockWitnessOptions,
    ) -> TransportResult<Option<BlockWitness>> {
        let builder = crate::witness::WitnessBuilder::new();
        let Some(block) = self
            .get_block(block.into(), BlockTransactionsKind::Full)
            .await?
        else {
            return Ok(None);
        };
        let number = block.header.number;
        let builder = builder
            .block(block)
            .chain_id(self.get_chain_id().await?)
            .execution_witness(self.debug_execution_witness(number.into()).await?);

        #[cfg(not(feature = "scroll"))]
        let builder = builder
            .ancestor_blocks(
                self.dump_block_ancestors(number, options.ancestors)
                    .await?
                    .unwrap(),
            )
            .unwrap();

        #[cfg(feature = "scroll")]
        let builder = builder
            .state_root(self.scroll_disk_root(number.into()).await?.disk_root)
            .unwrap()
            .prev_state_root(self.scroll_disk_root((number - 1).into()).await?.disk_root);

        let mut witness = builder.build().unwrap();

        if options.auto_fix {
            let mut attempts = 0;
            loop {
                if attempts >= MAX_AUTO_FIX_ATTEMPTS {
                    let e =
                        format!("failed to fetch fix state after {MAX_AUTO_FIX_ATTEMPTS} attempts");
                    dev_error!("{e}");
                    return Err(RpcError::LocalUsageError(e.into()));
                }
                attempts += 1;
                if let Err(e) = verify(&[&witness]) {
                    if let Some(hash) = e.as_blinded_node_err() {
                        // use `debug_dbGet` to fetch the missing state
                        let state = self.debug_db_get(hash).await.inspect(|_e| {
                            dev_error!("unable to fetch missing state due to: {_e}")
                        })?;
                        dev_info!("fetched missing state: {hash:#x} => {state:#x}");
                        witness.states.push(state);
                        continue;
                    }
                    dev_error!("unable to verify witness due to: {e}");
                }
                break;
            }
        }

        Ok(Some(witness))
    }

    /// Dump the chunk witness for a chunk.
    async fn dump_chunk_witness<I, T>(
        &self,
        blocks: I,
        options: DumpBlockWitnessOptions,
    ) -> TransportResult<Option<()>>
    where
        I: IntoIterator<Item = T> + Send + Sync,
        T: Into<BlockId> + Send + Sync,
    {
        let blocks = futures::future::try_join_all(
            blocks
                .into_iter()
                .map(|b| self.dump_block_witness(b, options)),
        )
        .await?
        .into_iter()
        .collect::<Option<Vec<_>>>();

        if blocks.is_none() {
            return Ok(None);
        }
        let mut blocks = blocks.unwrap();
        blocks.sort_by_key(|b| b.header.number);

        if options.auto_fix {
            let mut attempts = 0;
            loop {
                if attempts >= MAX_AUTO_FIX_ATTEMPTS {
                    let e =
                        format!("failed to fetch fix state after {MAX_AUTO_FIX_ATTEMPTS} attempts");
                    dev_error!("{e}");
                    return Err(RpcError::LocalUsageError(e.into()));
                }
                attempts += 1;
                if let Err(e) = verify(&*blocks) {
                    if let Some(hash) = e.as_blinded_node_err() {
                        // use `debug_dbGet` to fetch the missing state
                        let state = self.debug_db_get(hash).await.inspect(|_e| {
                            dev_error!("unable to fetch missing state due to: {_e}")
                        })?;
                        dev_info!("fetched missing state: {hash:#x} => {state:#x}");
                        blocks.first_mut().unwrap().states.push(state);
                        continue;
                    }
                    dev_error!("unable to verify witness due to: {e}");
                }
                break;
            }
        }

        Ok(Some(()))
    }

    /// Get a trie node by its hash using `debug_dbGet`
    #[doc(hidden)]
    async fn debug_db_get(&self, hash: B256) -> TransportResult<Bytes> {
        self.client()
            .request::<_, Bytes>("debug_dbGet", [hash])
            .await
    }

    /// Dump the ancestor blocks for a block.
    #[doc(hidden)]
    #[cfg(not(feature = "scroll"))]
    async fn dump_block_ancestors(
        &self,
        number: sbv_primitives::BlockNumber,
        ancestors: usize,
    ) -> TransportResult<Option<Vec<sbv_primitives::types::rpc::Block>>> {
        let ancestors = ancestors.clamp(1, (number as usize).min(256));

        let ancestors = futures::future::try_join_all((1..=ancestors).map(|offset| {
            let block_number = number - offset as sbv_primitives::BlockNumber;
            self.get_block_by_number(block_number.into(), BlockTransactionsKind::Hashes)
        }))
        .await?;

        if ancestors.iter().any(Option::is_none) {
            return Ok(None);
        }

        Ok(Some(ancestors.into_iter().map(Option::unwrap).collect()))
    }
}

impl<P: Provider<Network>> ProviderExt for P {}

impl DumpBlockWitnessOptions {
    /// Create a new [`DumpBlockWitnessOptions`].
    pub const fn const_new() -> Self {
        Self {
            #[cfg(not(feature = "scroll"))]
            ancestors: 256,
            auto_fix: true,
        }
    }

    /// Set the number of ancestors to dump.
    ///
    /// This is no-op if the `scroll` feature is enabled.
    #[allow(unused_mut, unused_variables)]
    pub const fn ancestors(mut self, ancestors: usize) -> Self {
        #[cfg(not(feature = "scroll"))]
        {
            self.ancestors = ancestors;
        }
        self
    }

    /// Set the auto-fix option, default is `true`.
    ///
    /// If `true`, the witness will be executed and missing states will be fetched.
    pub const fn auto_fix(mut self, auto_fix: bool) -> Self {
        self.auto_fix = auto_fix;
        self
    }
}

impl Default for DumpBlockWitnessOptions {
    fn default() -> Self {
        Self::const_new()
    }
}
