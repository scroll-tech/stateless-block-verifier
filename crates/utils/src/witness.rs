//! Witness builder.

use sbv_primitives::{
    B256, ChainId,
    types::{BlockWitness, ExecutionWitness, Transaction, rpc::Block as RpcBlock},
};

/// Block witness builder.
#[derive(Debug, Default)]
pub struct WitnessBuilder {
    chain_id: Option<ChainId>,
    block: Option<RpcBlock>,
    execution_witness: Option<ExecutionWitness>,
    prev_state_root: Option<B256>,

    #[cfg(not(feature = "scroll"))]
    blocks_hash: Option<Vec<B256>>,
}

/// Witness build error.
#[derive(Debug, thiserror::Error)]
pub enum WitnessBuildError {
    /// Missing field.
    #[error("missing field: {0}")]
    MissingField(&'static str),
    /// At least one ancestor block is required.
    #[cfg(not(feature = "scroll"))]
    #[error("at least one ancestor block is required")]
    AtLeastOneAncestorBlock,
}

impl WitnessBuilder {
    /// Create a new witness builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the chain ID.
    pub fn chain_id(mut self, chain_id: ChainId) -> Self {
        self.chain_id = Some(chain_id);
        self
    }

    /// Set the block.
    pub fn block(mut self, block: RpcBlock) -> Self {
        self.block = Some(block);
        self
    }

    /// Set the execution witness
    pub fn execution_witness(mut self, execution_witness: ExecutionWitness) -> Self {
        self.execution_witness = Some(execution_witness);
        self
    }

    /// Set the `blocks_hash` and `prev_state_root` from an iterator of ancestor blocks.
    #[cfg(not(feature = "scroll"))]
    pub fn ancestor_blocks<I>(mut self, iter: I) -> Result<Self, WitnessBuildError>
    where
        I: IntoIterator<Item = RpcBlock>,
    {
        self.blocks_hash = Some(iter.into_iter().map(|b| b.header.hash).collect());
        Ok(self)
    }

    /// Set the previous state root.
    pub fn prev_state_root(mut self, prev_state_root: B256) -> Self {
        self.prev_state_root = Some(prev_state_root);
        self
    }

    /// Build the block witness.
    pub fn build(self) -> Result<BlockWitness, WitnessBuildError> {
        let block = self.block.ok_or(WitnessBuildError::MissingField("block"))?;
        let execution_witness = self
            .execution_witness
            .ok_or(WitnessBuildError::MissingField("execution_witness"))?;
        Ok(BlockWitness {
            chain_id: self
                .chain_id
                .ok_or(WitnessBuildError::MissingField("chain_id"))?,
            header: block.header.into(),
            pre_state_root: self
                .prev_state_root
                .ok_or(WitnessBuildError::MissingField("prev_state_root"))?,
            transaction: block
                .transactions
                .as_transactions()
                .expect("expect transactions, got hashes")
                .iter()
                .map(Transaction::from_rpc)
                .collect(),
            #[cfg(not(feature = "scroll"))]
            block_hashes: self
                .blocks_hash
                .ok_or(WitnessBuildError::MissingField("ancestor_blocks"))?,
            withdrawals: block
                .withdrawals
                .map(|w| w.iter().map(From::from).collect()),
            states: execution_witness.state.into_values().collect(),
            codes: execution_witness.codes.into_values().collect(),
        })
    }
}
