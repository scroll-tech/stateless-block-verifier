use crate::utils::ext::*;
use eth_types::l2_types::{ArchivedBlockTraceV2, ArchivedTransactionTrace, TransactionTrace};
use eth_types::{Address, Transaction, H256};
use revm::primitives::{AccessListItem, TransactTo, B256, U256};
use rkyv::Deserialize;

impl BlockTraceExt for ArchivedBlockTraceV2 {
    #[inline(always)]
    fn root_before(&self) -> H256 {
        H256::from(self.storage_trace.root_before.0)
    }
    #[inline(always)]
    fn root_after(&self) -> H256 {
        H256::from(self.storage_trace.root_after.0)
    }
    #[inline(always)]
    fn withdraw_root(&self) -> H256 {
        H256::from(self.withdraw_trie_root.0)
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
                    .map(|(k, v)| (k.as_h256(), v.as_ref())),
            )
        }
    }
    #[inline(always)]
    fn address_hashes(&self) -> impl Iterator<Item = (&Address, &H256)> {
        self.storage_trace
            .address_hashes
            .iter()
            .map(|(k, v)| (k.as_h160(), v.as_h256()))
    }
    #[inline(always)]
    fn store_key_hashes(&self) -> impl Iterator<Item = (&H256, &H256)> {
        self.storage_trace
            .store_key_hashes
            .iter()
            .map(|(k, v)| (k.as_h256(), v.as_h256()))
    }
    #[inline(always)]
    fn account_proofs(&self) -> impl Iterator<Item = (&Address, impl IntoIterator<Item = &[u8]>)> {
        self.storage_trace
            .proofs
            .iter()
            .map(|(addr, b)| (addr.as_h160(), b.iter().map(|b| b.as_ref())))
    }
    #[inline(always)]
    fn storage_proofs(
        &self,
    ) -> impl Iterator<Item = (&Address, &H256, impl IntoIterator<Item = &[u8]>)> {
        self.storage_trace
            .storage_proofs
            .iter()
            .flat_map(|(addr, map)| {
                map.iter().map(move |(sk, bts)| {
                    (addr.as_h160(), sk.as_h256(), bts.iter().map(|b| b.as_ref()))
                })
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

impl BlockTraceRevmExt for ArchivedBlockTraceV2 {
    type Tx = ArchivedTransactionTrace;
    #[inline]
    fn number(&self) -> u64 {
        self.header.number.0[0]
    }
    #[inline]
    fn block_hash(&self) -> B256 {
        self.header.hash.0.into()
    }
    #[inline]
    fn chain_id(&self) -> u64 {
        self.chain_id
    }
    #[inline]
    fn coinbase(&self) -> revm::primitives::Address {
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
        self.header
            .base_fee_per_gas
            .as_ref()
            .map(|b| U256::from_limbs(b.0))
    }
    #[inline]
    fn difficulty(&self) -> U256 {
        U256::from_limbs(self.header.difficulty.0)
    }
    #[inline]
    fn prevrandao(&self) -> Option<B256> {
        self.header
            .mix_hash
            .as_ref()
            .map(|h| revm::primitives::B256::from(h.0))
    }
    #[inline]
    fn transactions(&self) -> impl Iterator<Item = &Self::Tx> {
        self.transactions.iter()
    }
}

impl BlockZktrieExt for ArchivedBlockTraceV2 {}

impl TxRevmExt for ArchivedTransactionTrace {
    #[inline]
    fn raw_type(&self) -> u8 {
        self.type_
    }
    #[inline]
    fn tx_hash(&self) -> B256 {
        B256::new(self.tx_hash.0)
    }
    #[inline]
    fn caller(&self) -> revm::precompile::Address {
        self.from.0.into()
    }
    #[inline]
    fn gas_limit(&self) -> u64 {
        self.gas
    }
    #[inline]
    fn gas_price(&self) -> U256 {
        U256::from_limbs(self.gas_price.0)
    }
    #[inline]
    fn transact_to(&self) -> TransactTo {
        match self.to.as_ref() {
            Some(to) => TransactTo::Call(to.0.into()),
            None => TransactTo::Create,
        }
    }
    #[inline]
    fn value(&self) -> U256 {
        U256::from_limbs(self.value.0)
    }
    #[inline]
    fn data(&self) -> revm::precompile::Bytes {
        revm::precompile::Bytes::copy_from_slice(self.data.as_ref())
    }
    #[inline]
    fn nonce(&self) -> u64 {
        self.nonce
    }
    #[inline]
    fn chain_id(&self) -> u64 {
        self.chain_id.0[0]
    }
    #[inline]
    fn access_list(&self) -> Vec<AccessListItem> {
        self.access_list
            .as_ref()
            .map(|v| {
                v.iter()
                    .map(|e| AccessListItem {
                        address: e.address.0.into(),
                        storage_keys: e.storage_keys.iter().map(|s| s.0.into()).collect(),
                    })
                    .collect()
            })
            .unwrap_or_default()
    }
    #[inline]
    fn gas_priority_fee(&self) -> Option<U256> {
        self.gas_tip_cap.as_ref().map(|g| U256::from_limbs(g.0))
    }
    #[inline]
    fn to_eth_tx(
        &self,
        block_hash: B256,
        block_number: u64,
        transaction_index: usize,
        base_fee_per_gas: Option<U256>,
    ) -> Transaction {
        // FIXME: zero copy here pls
        let tx_trace: TransactionTrace =
            Deserialize::<TransactionTrace, _>::deserialize(self, &mut rkyv::Infallible).unwrap();
        tx_trace.to_eth_tx(
            Some(H256::from(block_hash.0)),
            Some(block_number.into()),
            Some((transaction_index as u64).into()),
            base_fee_per_gas.map(|b| eth_types::U256(*b.as_limbs())),
        )
    }
}

impl BlockChunkExt for ArchivedBlockTraceV2 {}
