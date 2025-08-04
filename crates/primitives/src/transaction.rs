use crate::{
    AccessList, Address, B256, Bytes, ChainId, Signature, SignedAuthorization, TxHash, U256,
};

/// Transaction object used in RPC
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Transaction {
    /// Hash
    pub hash: TxHash,
    /// Nonce
    #[cfg_attr(feature = "serde", serde(with = "alloy_serde::quantity"))]
    pub nonce: u64,
    /// Sender
    pub from: Address,
    /// Recipient
    pub to: Option<Address>,
    /// Transferred value
    pub value: U256,
    /// Gas Price
    #[cfg_attr(
        feature = "serde",
        serde(default, with = "alloy_serde::quantity::opt",)
    )]
    pub gas_price: Option<u128>,
    /// Gas amount
    #[cfg_attr(feature = "serde", serde(with = "alloy_serde::quantity"))]
    pub gas: u64,
    /// Max BaseFeePerGas the user is willing to pay.
    #[cfg_attr(feature = "serde", serde(with = "alloy_serde::quantity"))]
    pub max_fee_per_gas: u128,
    /// The miner's tip.
    #[cfg_attr(
        feature = "serde",
        serde(default, with = "alloy_serde::quantity::opt",)
    )]
    pub max_priority_fee_per_gas: Option<u128>,
    /// Configured max fee per blob gas for eip-4844 transactions
    #[cfg_attr(
        feature = "serde",
        serde(default, with = "alloy_serde::quantity::opt",)
    )]
    pub max_fee_per_blob_gas: Option<u128>,
    /// Data
    pub input: Bytes,
    /// All _flattened_ fields of the transaction signature.
    ///
    /// Note: this is an option so special transaction types without a signature (e.g. <https://github.com/ethereum-optimism/optimism/blob/0bf643c4147b43cd6f25a759d331ef3a2a61a2a3/specs/deposits.md#the-deposited-transaction-type>) can be supported.
    pub signature: Option<Signature>,
    /// The chain id of the transaction, if any.
    #[cfg_attr(feature = "serde", serde(default, with = "alloy_serde::quantity::opt"))]
    pub chain_id: Option<ChainId>,
    /// Contains the blob hashes for eip-4844 transactions.
    pub blob_versioned_hashes: Option<Vec<B256>>,
    /// EIP2930
    ///
    /// Pre-pay to warm storage access.
    pub access_list: Option<AccessList>,
    /// EIP7702
    ///
    /// Authorizations are used to temporarily set the code of its signer to
    /// the code referenced by `address`. These also include a `chain_id` (which
    /// can be set to zero and not evaluated) as well as an optional `nonce`.
    pub authorization_list: Option<Vec<SignedAuthorization>>,
    /// EIP2718
    ///
    /// Transaction type,
    /// Some(4) for EIP-7702 transaction, Some(3) for EIP-4844 transaction, Some(2) for EIP-1559
    /// transaction, Some(1) for AccessList transaction, None or Some(0) for Legacy
    #[doc(alias = "tx_type")]
    pub transaction_type: u8,
    /// L1Msg queueIndex
    #[cfg(feature = "scroll")]
    #[cfg_attr(
        feature = "serde",
        serde(default, with = "alloy_serde::quantity::opt",)
    )]
    pub queue_index: Option<u64>,
}
