use sbv_primitives::{types::TypedTransaction, Header};

/// Temp helper struct for integrating [`reth_primitives::NodePrimitives`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NodePrimitives;

impl reth_primitives::NodePrimitives for NodePrimitives {
    type Block = ();
    type BlockHeader = Header;
    type BlockBody = ();
    type SignedTx = TypedTransaction;
    type Receipt = ();
}
