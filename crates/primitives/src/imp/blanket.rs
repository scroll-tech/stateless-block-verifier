use crate::*;
use eth_types::{Address, H256};
use revm_primitives::{AccessListItem, TransactTo, B256, U256};

impl<T: BlockTraceExt> BlockTraceExt for &T {
    #[inline(always)]
    fn root_before(&self) -> H256 {
        (*self).root_before()
    }
    #[inline(always)]
    fn root_after(&self) -> H256 {
        (*self).root_after()
    }
    #[inline(always)]
    fn withdraw_root(&self) -> H256 {
        (*self).withdraw_root()
    }
    #[inline(always)]
    fn account_proofs(&self) -> impl Iterator<Item = (&Address, impl IntoIterator<Item = &[u8]>)> {
        (*self).account_proofs()
    }
    #[inline(always)]
    fn storage_proofs(
        &self,
    ) -> impl Iterator<Item = (&Address, &H256, impl IntoIterator<Item = &[u8]>)> {
        (*self).storage_proofs()
    }
    #[inline(always)]
    fn additional_proofs(&self) -> impl Iterator<Item = &[u8]> {
        (*self).additional_proofs()
    }
    #[inline(always)]
    fn flatten_proofs(&self) -> Option<impl Iterator<Item = (&H256, &[u8])>> {
        (*self).flatten_proofs()
    }
    #[inline(always)]
    fn address_hashes(&self) -> impl Iterator<Item = (&Address, &H256)> {
        (*self).address_hashes()
    }
    #[inline(always)]
    fn store_key_hashes(&self) -> impl Iterator<Item = (&H256, &H256)> {
        (*self).store_key_hashes()
    }
    #[inline(always)]
    fn codes(&self) -> impl ExactSizeIterator<Item = &[u8]> {
        (*self).codes()
    }
    #[inline(always)]
    fn start_l1_queue_index(&self) -> u64 {
        (*self).start_l1_queue_index()
    }
}

impl<T: BlockTraceRevmExt> BlockTraceRevmExt for &T {
    type Tx = T::Tx;

    #[inline(always)]
    fn number(&self) -> u64 {
        (*self).number()
    }

    #[inline(always)]
    fn block_hash(&self) -> B256 {
        (*self).block_hash()
    }

    #[inline(always)]
    fn chain_id(&self) -> u64 {
        (*self).chain_id()
    }

    #[inline(always)]
    fn coinbase(&self) -> revm_primitives::Address {
        (*self).coinbase()
    }

    #[inline(always)]
    fn timestamp(&self) -> U256 {
        (*self).timestamp()
    }

    #[inline(always)]
    fn gas_limit(&self) -> U256 {
        (*self).gas_limit()
    }

    #[inline(always)]
    fn base_fee_per_gas(&self) -> Option<U256> {
        (*self).base_fee_per_gas()
    }

    #[inline(always)]
    fn difficulty(&self) -> U256 {
        (*self).difficulty()
    }

    #[inline(always)]
    fn prevrandao(&self) -> Option<B256> {
        (*self).prevrandao()
    }

    #[inline(always)]
    fn transactions(&self) -> impl Iterator<Item = &Self::Tx> {
        (*self).transactions()
    }
}

impl<T: BlockZktrieExt> BlockZktrieExt for &T {}

impl<T: Transaction> Transaction for &T {
    #[inline(always)]
    fn raw_type(&self) -> u8 {
        (*self).raw_type()
    }
    #[inline(always)]
    fn tx_hash(&self) -> B256 {
        (*self).tx_hash()
    }

    #[inline(always)]
    fn caller(&self) -> revm_primitives::Address {
        (*self).caller()
    }

    #[inline(always)]
    fn gas_limit(&self) -> u64 {
        (*self).gas_limit()
    }

    #[inline(always)]
    fn gas_price(&self) -> U256 {
        (*self).gas_price()
    }

    #[inline(always)]
    fn transact_to(&self) -> TransactTo {
        (*self).transact_to()
    }

    #[inline(always)]
    fn value(&self) -> U256 {
        (*self).value()
    }

    #[inline(always)]
    fn data(&self) -> revm_primitives::Bytes {
        (*self).data()
    }

    #[inline(always)]
    fn nonce(&self) -> u64 {
        (*self).nonce()
    }

    #[inline(always)]
    fn chain_id(&self) -> u64 {
        (*self).chain_id()
    }

    #[inline(always)]
    fn access_list(&self) -> Vec<AccessListItem> {
        (*self).access_list()
    }

    #[inline(always)]
    fn gas_priority_fee(&self) -> Option<U256> {
        (*self).gas_priority_fee()
    }

    #[inline(always)]
    fn to_eth_tx(
        &self,
        block_hash: B256,
        block_number: u64,
        transaction_index: usize,
        base_fee_per_gas: Option<U256>,
    ) -> eth_types::Transaction {
        (*self).to_eth_tx(
            block_hash,
            block_number,
            transaction_index,
            base_fee_per_gas,
        )
    }
}

impl<T: BlockChunkExt> BlockChunkExt for &T {}
