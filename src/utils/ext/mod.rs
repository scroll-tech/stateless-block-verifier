use eth_types::{state_db, Address, Transaction, Word, H256};
use mpt_zktrie::state::StorageData;
use mpt_zktrie::{AccountData, ZktrieState};
use revm::primitives::{AccessListItem, TransactTo, TxEnv, B256, U256};
use std::collections::hash_map::Entry::{Occupied, Vacant};
use std::collections::HashMap;
use std::fmt::Debug;

mod imp;

/// Common extension trait for BlockTrace
pub trait BlockTraceExt {
    fn root_before(&self) -> H256;
    fn root_after(&self) -> H256;
    fn withdraw_root(&self) -> H256;
    fn account_proofs(&self) -> impl Iterator<Item = (&Address, impl IntoIterator<Item = &[u8]>)>;
    fn storage_proofs(
        &self,
    ) -> impl Iterator<Item = (&Address, &H256, impl IntoIterator<Item = &[u8]>)>;
    fn additional_proofs(&self) -> impl Iterator<Item = &[u8]>;
    fn flatten_proofs(&self) -> Option<impl Iterator<Item = (&H256, &[u8])>>;
    fn address_hashes(&self) -> impl Iterator<Item = (&Address, &H256)>;
    fn store_key_hashes(&self) -> impl Iterator<Item = (&H256, &H256)>;
    fn codes(&self) -> impl Iterator<Item = &[u8]>;
    fn start_l1_queue_index(&self) -> u64;
}

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
    #[inline]
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
pub trait BlockRevmDbExt: BlockTraceExt {
    fn accounts(
        &self,
        zktrie_state: &ZktrieState,
    ) -> impl Iterator<Item = (Address, state_db::Account)> {
        let trie = zktrie_state
            .zk_db
            .new_ref_trie(&self.root_before().0)
            .unwrap();
        self.account_proofs()
            .map(|(addr, _)| addr)
            .map(move |&addr| {
                let account = trie.get_account(addr.as_bytes()).map(AccountData::from);
                (addr, account)
            })
            .map(|(addr, acc)| {
                (
                    addr,
                    acc.map(|acc| state_db::Account::from(&acc))
                        .unwrap_or_else(state_db::Account::zero),
                )
            })
    }
    fn storages(
        &self,
        zktrie_state: &ZktrieState,
    ) -> impl Iterator<Item = ((Address, H256), Word)> {
        let zk_db = zktrie_state.zk_db.clone();
        let account_trie = zk_db.new_ref_trie(&self.root_before().0).unwrap();
        let mut trie_cache = HashMap::new();
        self.storage_proofs()
            .map(|(addr, key, _)| (addr, key))
            .map(move |(&addr, &key)| {
                let store_val = match trie_cache.entry(addr) {
                    Occupied(entry) => Some(entry.into_mut()),
                    Vacant(entry) => account_trie
                        .get_account(addr.as_bytes())
                        .map(AccountData::from)
                        .and_then(|account| {
                            zk_db
                                .new_ref_trie(&account.storage_root.0)
                                .map(|tr| entry.insert(tr))
                        }),
                }
                .and_then(|tr| tr.get_store(key.as_bytes()).map(StorageData::from));
                ((addr, key), store_val)
            })
            .map(|((addr, key), val)| ((addr, key), val.map(|val| val.into()).unwrap_or_default()))
    }
}

pub trait BlockZktrieExt: BlockTraceExt {
    fn build_zktrie_state(&self, zktrie_state: &mut ZktrieState) {
        measure_duration_histogram!(
            build_zktrie_state_duration_microseconds,
            if let Some(flatten_proofs) = self.flatten_proofs() {
                dev_debug!("init zktrie state with flatten proofs");
                let zk_db = zktrie_state.expose_db();

                for (k, bytes) in flatten_proofs {
                    zk_db.add_node_bytes(bytes, Some(k.as_bytes())).unwrap();
                }
            } else {
                dev_warn!("no flatten proofs, fallback to update zktrie state from trace");
                zktrie_state.update_from_trace(
                    self.account_proofs(),
                    self.storage_proofs(),
                    self.additional_proofs(),
                );
            }
        );
    }
}

pub trait BlockChunkExt: BlockTraceExt + BlockTraceRevmExt {
    #[inline]
    fn num_l1_txs(&self) -> u64 {
        // 0x7e is l1 tx
        match self
            .transactions()
            .filter(|tx| tx.is_l1_tx())
            // tx.nonce for l1 tx is the l1 queue index, which is a globally index,
            // not per user as suggested by the name...
            .map(|tx| tx.nonce())
            .max()
        {
            None => 0, // not l1 tx in this block
            Some(end_l1_queue_index) => end_l1_queue_index - self.start_l1_queue_index() + 1,
        }
    }
    #[inline]
    fn num_l2_txs(&self) -> u64 {
        // 0x7e is l1 tx
        self.transactions().filter(|tx| !tx.is_l1_tx()).count() as u64
    }

    #[inline]
    fn hash_da_header(&self, hasher: &mut impl tiny_keccak::Hasher) {
        let num_txs = (self.num_l1_txs() + self.num_l2_txs()) as u16;
        hasher.update(&self.number().to_be_bytes());
        hasher.update(&self.timestamp().to::<u64>().to_be_bytes());
        hasher.update(
            &self
                .base_fee_per_gas()
                .unwrap_or_default()
                .to_be_bytes::<{ U256::BYTES }>(),
        );
        hasher.update(&self.gas_limit().to::<u64>().to_be_bytes());
        hasher.update(&num_txs.to_be_bytes());
    }

    #[inline]
    fn hash_l1_msg(&self, hasher: &mut impl tiny_keccak::Hasher) {
        for tx_hash in self
            .transactions()
            .filter(|tx| tx.is_l1_tx())
            .map(|tx| tx.tx_hash())
        {
            hasher.update(tx_hash.as_slice())
        }
    }
}

pub trait TxRevmExt {
    /// get the raw tx type
    fn raw_type(&self) -> u8;
    fn is_l1_tx(&self) -> bool {
        self.raw_type() == 0x7e
    }
    fn tx_hash(&self) -> B256;
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
        assert_eq!(archived.0, h160.0);
        let ptr_to_archived: usize = archived as *const _ as usize;
        let ptr_to_archived_inner: usize = (&archived.0) as *const _ as usize;
        assert_eq!(ptr_to_archived, ptr_to_archived_inner);
        let transmuted: &H160 = unsafe { transmute(archived) };
        assert_eq!(transmuted, &h160);
        let transmuted: &H160 = unsafe { transmute(&archived.0) };
        assert_eq!(transmuted, &h160);
    }
}
