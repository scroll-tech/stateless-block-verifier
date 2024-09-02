use crate::*;
use eth_types::l2_types::{BlockTrace, TransactionTrace};
use eth_types::{Address, H256};
use revm_primitives::{B256, U256};

impl BlockTraceExt for BlockTrace {
    #[inline(always)]
    fn root_before(&self) -> H256 {
        self.storage_trace.root_before
    }
    #[inline(always)]
    fn root_after(&self) -> H256 {
        self.storage_trace.root_after
    }
    #[inline(always)]
    fn withdraw_root(&self) -> H256 {
        self.withdraw_trie_root
    }
    #[inline(always)]
    fn flatten_proofs(&self) -> Option<impl Iterator<Item = (&H256, &[u8])>> {
        if self.storage_trace.flatten_proofs.is_empty() {
            None
        } else {
            Some(
                self.storage_trace
                    .flatten_proofs
                    .iter()
                    .map(|(k, v)| (k, v.as_ref())),
            )
        }
    }
    #[inline(always)]
    fn address_hashes(&self) -> impl Iterator<Item = (&Address, &H256)> {
        self.storage_trace
            .address_hashes
            .iter()
            .map(|(k, v)| (k, v))
    }
    #[inline(always)]
    fn store_key_hashes(&self) -> impl Iterator<Item = (&H256, &H256)> {
        self.storage_trace
            .store_key_hashes
            .iter()
            .map(|(k, v)| (k, v))
    }
    #[inline(always)]
    fn account_proofs(&self) -> impl Iterator<Item = (&Address, impl IntoIterator<Item = &[u8]>)> {
        self.storage_trace
            .proofs
            .iter()
            .map(|(addr, b)| (addr, b.iter().map(|b| b.as_ref())))
    }
    #[inline(always)]
    fn storage_proofs(
        &self,
    ) -> impl Iterator<Item = (&Address, &H256, impl IntoIterator<Item = &[u8]>)> {
        self.storage_trace
            .storage_proofs
            .iter()
            .flat_map(|(addr, map)| {
                map.iter()
                    .map(move |(sk, bts)| (addr, sk, bts.iter().map(|b| b.as_ref())))
            })
    }
    #[inline(always)]
    fn additional_proofs(&self) -> impl Iterator<Item = &[u8]> {
        self.storage_trace
            .deletion_proofs
            .iter()
            .map(|s| s.as_ref())
    }
    #[inline]
    fn codes(&self) -> impl ExactSizeIterator<Item = &[u8]> {
        self.codes.iter().map(|code| code.code.as_ref())
    }
    #[inline]
    fn start_l1_queue_index(&self) -> u64 {
        self.start_l1_queue_index
    }
}

impl BlockTraceRevmExt for BlockTrace {
    type Tx = TransactionTrace;

    #[inline]
    fn number(&self) -> u64 {
        self.header.number.expect("incomplete block").as_u64()
    }
    #[inline]
    fn block_hash(&self) -> B256 {
        self.header.hash.expect("incomplete block").0.into()
    }
    #[inline]
    fn chain_id(&self) -> u64 {
        self.chain_id
    }
    #[inline]
    fn coinbase(&self) -> revm_primitives::Address {
        self.coinbase.address.0.into()
    }
    #[inline]
    fn timestamp(&self) -> U256 {
        U256::from_limbs(self.header.timestamp.0)
    }
    #[inline]
    fn gas_limit(&self) -> U256 {
        U256::from_limbs(self.header.gas_limit.0)
    }
    #[inline]
    fn base_fee_per_gas(&self) -> Option<U256> {
        self.header.base_fee_per_gas.map(|b| U256::from_limbs(b.0))
    }
    #[inline]
    fn difficulty(&self) -> U256 {
        U256::from_limbs(self.header.difficulty.0)
    }
    #[inline]
    fn prevrandao(&self) -> Option<B256> {
        self.header.mix_hash.map(|h| B256::from(h.0))
    }
    #[inline]
    fn transactions(&self) -> impl Iterator<Item = &Self::Tx> {
        self.transactions.iter()
    }
}

impl BlockZktrieExt for BlockTrace {}

impl BlockChunkExt for BlockTrace {}
