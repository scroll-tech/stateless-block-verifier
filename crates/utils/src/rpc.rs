//! Rpc Extension

use alloy_provider::Provider;
use alloy_transport::TransportResult;
use sbv_primitives::{BlockWitness, ExecutionWitness, Network, eips::BlockNumberOrTag};

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

    /// Dump the block witness for a block.
    async fn dump_block_witness(
        &self,
        number: BlockNumberOrTag,
        #[cfg(not(feature = "scroll"))] ancestors: Option<usize>,
    ) -> TransportResult<Option<BlockWitness>> {
        let builder = crate::witness::WitnessBuilder::new();
        let Some(block) = self.get_block_by_number(number).full().await? else {
            return Ok(None);
        };
        let parent_block = self
            .get_block_by_hash(block.header.parent_hash)
            .await?
            .expect("parent block should exist");
        let number = block.header.number;

        let builder = builder
            .block(block)
            .prev_state_root(parent_block.header.state_root)
            .chain_id(self.get_chain_id().await?)
            .execution_witness(self.debug_execution_witness(number.into()).await?);

        #[cfg(not(feature = "scroll"))]
        let builder = builder
            .ancestor_blocks(self.dump_block_ancestors(number, ancestors).await?.unwrap())
            .unwrap();

        Ok(Some(builder.build().unwrap()))
    }

    /// Dump the ancestor blocks for a block.
    #[doc(hidden)]
    #[cfg(not(feature = "scroll"))]
    async fn dump_block_ancestors(
        &self,
        number: sbv_primitives::BlockNumber,
        ancestors: Option<usize>,
    ) -> TransportResult<Option<Vec<sbv_primitives::rpc::Block>>> {
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
