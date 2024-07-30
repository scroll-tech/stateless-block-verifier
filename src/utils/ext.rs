use eth_types::l2_types::{
    ArchivedBlockTraceV2, ArchivedTransactionTrace, BlockTrace, BlockTraceV2, TransactionTrace,
};
use eth_types::{state_db, Address, Transaction, Word, H256};
use mpt_zktrie::ZktrieState;
use revm::primitives::{AccessListItem, TransactTo, TxEnv, B256, U256};
use rkyv::Deserialize;
use std::fmt::Debug;
use std::mem;
use zktrie::ZkTrie;

/// Revm extension trait for BlockTrace
pub trait BlockTraceRevmExt {
    type Tx: TxRevmExt + Debug;

    /// block number
    fn number(&self) -> u64;
    /// block hash
    fn block_hash(&self) -> B256;
    /// chain id
    fn chain_id(&self) -> u64;
    /// coinbase address
    fn coinbase(&self) -> revm::primitives::Address;
    /// timestamp
    fn timestamp(&self) -> U256;
    /// gas limit
    fn gas_limit(&self) -> U256;
    /// base fee per gas
    fn base_fee_per_gas(&self) -> Option<U256>;
    /// difficulty
    fn difficulty(&self) -> U256;
    /// prevrandao
    fn prevrandao(&self) -> Option<B256>;

    /// transactions
    fn transactions(&self) -> impl Iterator<Item = &Self::Tx>;

    /// creates `revm::primitives::BlockEnv`
    fn env(&self) -> revm::primitives::BlockEnv {
        revm::primitives::BlockEnv {
            number: U256::from_limbs([self.number(), 0, 0, 0]),
            coinbase: self.coinbase(),
            timestamp: self.timestamp(),
            gas_limit: self.gas_limit(),
            basefee: self.base_fee_per_gas().unwrap_or_default(),
            difficulty: self.difficulty(),
            prevrandao: self.prevrandao(),
            blob_excess_gas_and_price: None,
        }
    }
}

/// Revm extension trait for init db
pub trait BlockRevmDbExt {
    fn accounts(&self) -> impl Iterator<Item = (Address, state_db::Account)>;
    fn storages(&self) -> impl Iterator<Item = ((Address, H256), Word)>;
    fn codes(&self) -> impl Iterator<Item = (H256, Vec<u8>)>;
}

pub trait BlockZktrieExt {
    fn zktrie(&self) -> ZkTrie;
}

pub trait TxRevmExt {
    /// get the raw tx type
    fn raw_type(&self) -> u8;
    fn caller(&self) -> revm::primitives::Address;
    fn gas_limit(&self) -> u64;
    fn gas_price(&self) -> U256;
    fn transact_to(&self) -> TransactTo;
    fn value(&self) -> U256;
    fn data(&self) -> revm::primitives::Bytes;
    fn nonce(&self) -> u64;
    fn chain_id(&self) -> u64;
    fn access_list(&self) -> Vec<AccessListItem>;
    fn gas_priority_fee(&self) -> Option<U256>;

    /// creates `revm::primitives::TxEnv`
    fn tx_env(&self) -> TxEnv {
        TxEnv {
            caller: self.caller(),
            gas_limit: self.gas_limit(),
            gas_price: self.gas_price(),
            transact_to: self.transact_to(),
            value: self.value(),
            data: self.data(),
            nonce: Some(self.nonce()),
            chain_id: Some(self.chain_id()),
            access_list: self.access_list(),
            gas_priority_fee: self.gas_priority_fee(),
            ..Default::default()
        }
    }

    fn to_eth_tx(
        &self,
        block_hash: B256,
        block_number: u64,
        transaction_index: usize,
        base_fee_per_gas: Option<U256>,
    ) -> Transaction;
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
        self.header.base_fee_per_gas.map(|b| U256::from_limbs(b.0))
    }
    #[inline]
    fn difficulty(&self) -> U256 {
        U256::from_limbs(self.header.difficulty.0)
    }
    #[inline]
    fn prevrandao(&self) -> Option<B256> {
        self.header
            .mix_hash
            .map(|h| revm::primitives::B256::from(h.0))
    }
    #[inline]
    fn transactions(&self) -> impl Iterator<Item = &Self::Tx> {
        self.transactions.iter()
    }
}

impl BlockTraceRevmExt for BlockTraceV2 {
    type Tx = TransactionTrace;
    #[inline]
    fn number(&self) -> u64 {
        self.header.number.as_u64()
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
        self.header.base_fee_per_gas.map(|b| U256::from_limbs(b.0))
    }
    #[inline]
    fn difficulty(&self) -> U256 {
        U256::from_limbs(self.header.difficulty.0)
    }
    #[inline]
    fn prevrandao(&self) -> Option<B256> {
        self.header
            .mix_hash
            .map(|h| revm::primitives::B256::from(h.0))
    }
    #[inline]
    fn transactions(&self) -> impl Iterator<Item = &Self::Tx> {
        self.transactions.iter()
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

impl BlockRevmDbExt for BlockTrace {
    #[inline]
    fn accounts(&self) -> impl Iterator<Item = (Address, state_db::Account)> {
        ZktrieState::parse_account_from_proofs(
            self.storage_trace
                .proofs
                .iter()
                .map(|(addr, b)| (addr, b.iter().map(|b| b.as_ref()))),
        )
        .map(|parsed| {
            let (addr, acc) = parsed.unwrap();
            (addr, state_db::Account::from(&acc))
        })
    }
    #[inline]
    fn storages(&self) -> impl Iterator<Item = ((Address, H256), Word)> {
        ZktrieState::parse_storage_from_proofs(self.storage_trace.storage_proofs.iter().flat_map(
            |(addr, map)| {
                map.iter()
                    .map(move |(sk, bts)| (addr, sk, bts.iter().map(|b| b.as_ref())))
            },
        ))
        .map(|parsed| {
            let ((addr, key), val) = parsed.unwrap();
            ((addr, key), val.into())
        })
    }
    #[inline]
    fn codes(&self) -> impl Iterator<Item = (H256, Vec<u8>)> {
        self.codes
            .iter()
            .map(|trace| (trace.hash, trace.code.to_vec()))
    }
}

impl BlockRevmDbExt for BlockTraceV2 {
    #[inline]
    fn accounts(&self) -> impl Iterator<Item = (Address, state_db::Account)> {
        ZktrieState::parse_account_from_proofs(
            self.storage_trace
                .proofs
                .iter()
                .map(|(addr, b)| (addr, b.iter().map(|b| b.as_ref()))),
        )
        .map(|parsed| {
            let (addr, acc) = parsed.unwrap();
            (addr, state_db::Account::from(&acc))
        })
    }
    #[inline]
    fn storages(&self) -> impl Iterator<Item = ((Address, H256), Word)> {
        ZktrieState::parse_storage_from_proofs(self.storage_trace.storage_proofs.iter().flat_map(
            |(addr, map)| {
                map.iter()
                    .map(move |(sk, bts)| (addr, sk, bts.iter().map(|b| b.as_ref())))
            },
        ))
        .map(|parsed| {
            let ((addr, key), val) = parsed.unwrap();
            ((addr, key), val.into())
        })
    }
    #[inline]
    fn codes(&self) -> impl Iterator<Item = (H256, Vec<u8>)> {
        self.codes
            .iter()
            .map(|trace| (trace.hash, trace.code.to_vec()))
    }
}

impl BlockRevmDbExt for ArchivedBlockTraceV2 {
    #[inline]
    fn accounts(&self) -> impl Iterator<Item = (Address, state_db::Account)> {
        ZktrieState::parse_account_from_proofs(self.storage_trace.proofs.iter().map(|(addr, b)| {
            let addr: &Address = unsafe { mem::transmute(&addr.0) };
            (addr, b.iter().map(|b| b.as_ref()))
        }))
        .map(|parsed| {
            let (addr, acc) = parsed.unwrap();
            (addr, state_db::Account::from(&acc))
        })
    }
    #[inline]
    fn storages(&self) -> impl Iterator<Item = ((Address, H256), Word)> {
        ZktrieState::parse_storage_from_proofs(self.storage_trace.storage_proofs.iter().flat_map(
            |(addr, map)| {
                let addr: &Address = unsafe { mem::transmute(&addr.0) };

                map.iter().map(move |(sk, bts)| {
                    let sk: &H256 = unsafe { mem::transmute(&sk.0) };
                    (addr, sk, bts.iter().map(|b| b.as_ref()))
                })
            },
        ))
        .map(|parsed| {
            let ((addr, key), val) = parsed.unwrap();
            ((addr, key), val.into())
        })
    }
    #[inline]
    fn codes(&self) -> impl Iterator<Item = (H256, Vec<u8>)> {
        self.codes
            .iter()
            .map(|trace| (trace.hash.0.into(), trace.code.to_vec()))
    }
}

impl BlockZktrieExt for BlockTrace {
    fn zktrie(&self) -> ZkTrie {
        let old_root = self.storage_trace.root_before;
        let zktrie_state = ZktrieState::from_trace_with_additional(
            old_root,
            self.storage_trace
                .proofs
                .iter()
                .map(|(addr, b)| (addr, b.iter().map(|b| b.as_ref()))),
            self.storage_trace
                .storage_proofs
                .iter()
                .flat_map(|(addr, map)| {
                    map.iter()
                        .map(move |(sk, bts)| (addr, sk, bts.iter().map(|b| b.as_ref())))
                }),
            self.storage_trace
                .deletion_proofs
                .iter()
                .map(|s| s.as_ref()),
        )
        .unwrap();
        let root = *zktrie_state.root();
        debug!("building partial statedb done, root {}", hex::encode(root));

        let mem_db = zktrie_state.into_inner();
        mem_db.new_trie(&root).unwrap()
    }
}

impl BlockZktrieExt for BlockTraceV2 {
    fn zktrie(&self) -> ZkTrie {
        let old_root = self.storage_trace.root_before;
        let zktrie_state = ZktrieState::from_trace_with_additional(
            old_root,
            self.storage_trace
                .proofs
                .iter()
                .map(|(addr, b)| (addr, b.iter().map(|b| b.as_ref()))),
            self.storage_trace
                .storage_proofs
                .iter()
                .flat_map(|(addr, map)| {
                    map.iter()
                        .map(move |(sk, bts)| (addr, sk, bts.iter().map(|b| b.as_ref())))
                }),
            self.storage_trace
                .deletion_proofs
                .iter()
                .map(|s| s.as_ref()),
        )
        .unwrap();
        let root = *zktrie_state.root();
        debug!("building partial statedb done, root {}", hex::encode(root));

        let mem_db = zktrie_state.into_inner();
        mem_db.new_trie(&root).unwrap()
    }
}

impl BlockZktrieExt for ArchivedBlockTraceV2 {
    fn zktrie(&self) -> ZkTrie {
        let old_root: H256 = self.storage_trace.root_before.0.into();
        let zktrie_state = ZktrieState::from_trace_with_additional(
            old_root,
            self.storage_trace.proofs.iter().map(|(addr, b)| {
                let addr = unsafe { mem::transmute::<&[u8; 20], &Address>(&addr.0) };
                (addr, b.iter().map(|b| b.as_ref()))
            }),
            self.storage_trace
                .storage_proofs
                .iter()
                .flat_map(|(addr, map)| {
                    let addr = unsafe { mem::transmute::<&[u8; 20], &Address>(&addr.0) };
                    map.iter().map(move |(sk, bts)| {
                        let sk = unsafe { mem::transmute::<&[u8; 32], &eth_types::H256>(&sk.0) };
                        (addr, sk, bts.iter().map(|b| b.as_ref()))
                    })
                }),
            self.storage_trace
                .deletion_proofs
                .iter()
                .map(|s| s.as_ref()),
        )
        .unwrap();
        let root = *zktrie_state.root();
        debug!("building partial statedb done, root {}", hex::encode(root));

        let mem_db = zktrie_state.into_inner();
        mem_db.new_trie(&root).unwrap()
    }
}

impl TxRevmExt for TransactionTrace {
    #[inline]
    fn raw_type(&self) -> u8 {
        self.type_
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
        match self.to {
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
        self.chain_id.as_u64()
    }
    #[inline]
    fn access_list(&self) -> Vec<AccessListItem> {
        self.access_list
            .as_ref()
            .map(|v| {
                v.iter()
                    .map(|e| AccessListItem {
                        address: e.address.0.into(),
                        storage_keys: e
                            .storage_keys
                            .iter()
                            .map(|s| s.to_fixed_bytes().into())
                            .collect(),
                    })
                    .collect()
            })
            .unwrap_or_default()
    }
    #[inline]
    fn gas_priority_fee(&self) -> Option<U256> {
        self.gas_tip_cap.map(|g| U256::from_limbs(g.0))
    }
    #[inline]
    fn to_eth_tx(
        &self,
        block_hash: B256,
        block_number: u64,
        transaction_index: usize,
        base_fee_per_gas: Option<U256>,
    ) -> Transaction {
        self.to_eth_tx(
            Some(H256::from(block_hash.0)),
            Some(block_number.into()),
            Some((transaction_index as u64).into()),
            base_fee_per_gas.map(|b| eth_types::U256(*b.as_limbs())),
        )
    }
}

impl TxRevmExt for ArchivedTransactionTrace {
    #[inline]
    fn raw_type(&self) -> u8 {
        self.type_
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
    fn coinbase(&self) -> revm::precompile::Address {
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

impl<T: BlockRevmDbExt> BlockRevmDbExt for &T {
    #[inline(always)]
    fn accounts(&self) -> impl Iterator<Item = (Address, state_db::Account)> {
        (*self).accounts()
    }

    #[inline(always)]
    fn storages(&self) -> impl Iterator<Item = ((Address, H256), Word)> {
        (*self).storages()
    }

    #[inline(always)]
    fn codes(&self) -> impl Iterator<Item = (H256, Vec<u8>)> {
        (*self).codes()
    }
}

impl<T: BlockZktrieExt> BlockZktrieExt for &T {
    #[inline(always)]
    fn zktrie(&self) -> ZkTrie {
        (*self).zktrie()
    }
}

impl<T: TxRevmExt> TxRevmExt for &T {
    #[inline(always)]
    fn raw_type(&self) -> u8 {
        (*self).raw_type()
    }

    #[inline(always)]
    fn caller(&self) -> revm::precompile::Address {
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
    fn data(&self) -> revm::precompile::Bytes {
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
    ) -> Transaction {
        (*self).to_eth_tx(
            block_hash,
            block_number,
            transaction_index,
            base_fee_per_gas,
        )
    }
}

#[cfg(test)]
mod tests {
    use std::array;
    use std::mem::transmute;

    #[test]
    fn test_memory_layout() {
        use eth_types::{ArchivedH160, H160};
        // H160 and ArchivedH160 should have the same memory layout
        assert_eq!(size_of::<H160>(), 20);
        assert_eq!(size_of::<ArchivedH160>(), 20);
        assert_eq!(size_of::<&[u8; 20]>(), size_of::<usize>());
        assert_eq!(size_of::<&H160>(), size_of::<usize>());
        assert_eq!(size_of::<&ArchivedH160>(), size_of::<usize>());

        let h160 = eth_types::H160::from(array::from_fn(|i| i as u8));
        let serialized = rkyv::to_bytes::<_, 20>(&h160).unwrap();
        let archived: &ArchivedH160 = unsafe { rkyv::archived_root::<H160>(&serialized[..]) };
        assert_eq!(archived, &h160);
        let ptr_to_archived: usize = archived as *const _ as usize;
        let ptr_to_archived_inner: usize = (&archived.0) as *const _ as usize;
        assert_eq!(ptr_to_archived, ptr_to_archived_inner);
        let transmuted: &H160 = unsafe { transmute(archived) };
        assert_eq!(transmuted, &h160);
        let transmuted: &H160 = unsafe { transmute(&archived.0) };
        assert_eq!(transmuted, &h160);
    }
}
