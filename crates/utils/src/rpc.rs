//! Rpc Extension

use crate::witness::WitnessBuilder;
use alloy_provider::Provider;
use alloy_transport::TransportResult;
use sbv_primitives::{
    B256, BlockNumber, Bytes, ChainId,
    alloy_primitives::map::B256HashMap,
    types::{
        BlockWitness, Network,
        eips::BlockNumberOrTag,
        rpc::{Block, ExecutionWitness},
    },
};
use serde::Deserialize;

/// Extension trait for [`Provider`](Provider).
#[async_trait::async_trait]
pub trait ProviderExt: Provider<Network> {
    /// Get the execution witness for a block.
    async fn debug_execution_witness(
        &self,
        number: BlockNumberOrTag,
    ) -> TransportResult<ExecutionWitness> {
        /// Represents the execution witness of a block. Contains an optional map of state preimages.
        #[derive(Debug, Deserialize)]
        struct GethExecutionWitness {
            pub state: B256HashMap<Bytes>,
            pub codes: B256HashMap<Bytes>,
        }

        #[derive(Debug, Deserialize)]
        #[serde(untagged)]
        enum ExecutionWitnessDeHelper {
            Standard(ExecutionWitness),
            Geth(GethExecutionWitness),
        }

        self.client()
            .request::<_, ExecutionWitnessDeHelper>("debug_executionWitness", (number,))
            .await
            .map(|response| match response {
                ExecutionWitnessDeHelper::Standard(witness) => witness,
                ExecutionWitnessDeHelper::Geth(witness) => ExecutionWitness {
                    state: witness.state.into_values().collect(),
                    codes: witness.codes.into_values().collect(),
                    ..Default::default()
                },
            })
    }

    /// Dump the block witness for a block.
    ///
    /// # Panics
    ///
    /// This function will panic if the block number is 0.
    fn dump_block_witness(&self, number: BlockNumber) -> DumpBlockWitness<Self>
    where
        Self: Sized,
    {
        assert_ne!(number, 0, "genesis block is not traceable");
        DumpBlockWitness::new(self, number)
    }

    /// Dump the ancestor blocks for a block.
    #[doc(hidden)]
    #[cfg(not(feature = "scroll"))]
    async fn dump_block_ancestors(
        &self,
        number: sbv_primitives::BlockNumber,
        ancestors: Option<usize>,
    ) -> TransportResult<Option<Vec<sbv_primitives::types::rpc::Block>>> {
        use std::future::IntoFuture;

        let ancestors = ancestors
            .unwrap_or_default()
            .clamp(1, (number as usize).min(256));

        let ancestors = futures::future::try_join_all((1..=ancestors).map(|offset| {
            let block_number = number - offset as sbv_primitives::BlockNumber;
            self.get_block_by_number(block_number.into()).into_future()
        }))
        .await?;

        if ancestors.iter().any(Option::is_none) {
            return Ok(None);
        }

        Ok(Some(ancestors.into_iter().map(Option::unwrap).collect()))
    }
}

impl<P: Provider<Network>> ProviderExt for P {}

/// DumpBlockWitness created via [`ProviderExt::dump_block_witness`].
#[must_use = "DumpBlockWitness does not execute until you call `send`"]
#[derive(Debug)]
pub struct DumpBlockWitness<'a, P> {
    provider: &'a P,
    number: BlockNumber,
    #[cfg(not(feature = "scroll"))]
    ancestors: Option<usize>,

    builder: WitnessBuilder,
}

impl<'a, P: ProviderExt> DumpBlockWitness<'a, P> {
    fn new(provider: &'a P, number: BlockNumber) -> Self {
        Self {
            provider,
            number,
            #[cfg(not(feature = "scroll"))]
            ancestors: None,

            builder: WitnessBuilder::default(),
        }
    }

    /// Set the builder
    pub fn builder(mut self, builder: WitnessBuilder) -> Self {
        self.builder = builder;
        self
    }

    /// Set the number of ancestors to include in the witness.
    #[cfg(not(feature = "scroll"))]
    pub fn ancestors(mut self, ancestors: usize) -> Self {
        self.ancestors = Some(ancestors);
        self
    }

    /// Set the block number to dump.
    ///
    /// # Panics
    ///
    /// This function will panic if the block number is 0.
    pub fn with_number(mut self, number: BlockNumber) -> Self {
        assert_ne!(number, 0, "genesis block is not traceable");
        self.number = number;
        self
    }

    /// Set the chain ID.
    pub fn with_chain_id(mut self, chain_id: ChainId) -> Self {
        self.builder = self.builder.chain_id(chain_id);
        self
    }

    /// Use cached block.
    ///
    /// # Panics
    ///
    /// This function will panic if the block number does not match the builder's block number.
    pub fn with_cached_block(mut self, block: Block) -> Self {
        assert_eq!(
            block.header.number, self.number,
            "block number does not match builder's block number"
        );

        self.builder = self.builder.block(block);
        self
    }

    /// Use cached previous block.
    ///
    /// # Panics
    ///
    /// This function will panic if the block number
    pub fn with_cached_prev_block(mut self, prev_block: &Block) -> Self {
        assert_eq!(
            prev_block.header.number,
            self.number.checked_sub(1).expect("block number underflow"),
            "block number does not match builder's block number"
        );

        self.builder = self.builder.prev_state_root(prev_block.header.state_root);
        self
    }

    /// Set the execution witness of current block.
    pub fn with_cached_execution_witness(mut self, execution_witness: ExecutionWitness) -> Self {
        self.builder = self.builder.execution_witness(execution_witness);
        self
    }

    /// Use cached ancestor blocks.
    #[cfg(not(feature = "scroll"))]
    pub fn with_cached_ancestor_blocks<I>(mut self, iter: I) -> Self
    where
        I: IntoIterator<Item = sbv_primitives::types::rpc::Block>,
    {
        self.builder = self.builder.ancestor_blocks(iter);
        self
    }

    /// Set the previous state root.
    pub fn with_prev_state_root(mut self, prev_state_root: B256) -> Self {
        self.builder = self.builder.prev_state_root(prev_state_root);
        self
    }

    /// Send the request to dump the block witness.
    pub async fn send(mut self) -> TransportResult<Option<BlockWitness>> {
        if self.builder.chain_id.is_none() {
            self.builder = self.builder.chain_id(self.provider.get_chain_id().await?);
        }

        if self.builder.block.is_none() {
            let Some(block) = self
                .provider
                .get_block_by_number(self.number.into())
                .full()
                .await?
            else {
                return Ok(None);
            };
            self.builder = self.builder.block(block);
        }

        if self.builder.prev_state_root.is_none() {
            let block = self.builder.block.as_ref().unwrap();
            let parent_block = self
                .provider
                .get_block_by_hash(block.header.parent_hash)
                .await?
                .expect("parent block should exist");

            self.builder = self.builder.prev_state_root(parent_block.header.state_root);
        }

        if self.builder.execution_witness.is_none() {
            let execution_witness = self
                .provider
                .debug_execution_witness(self.number.into())
                .await?;
            self.builder = self.builder.execution_witness(execution_witness);
        }

        #[cfg(not(feature = "scroll"))]
        if self.builder.blocks_hash.is_none() {
            let ancestors = self
                .provider
                .dump_block_ancestors(self.number, self.ancestors)
                .await?
                .unwrap();

            self.builder = self.builder.ancestor_blocks(ancestors);
        }

        Ok(Some(self.builder.build().unwrap()))
    }
}
