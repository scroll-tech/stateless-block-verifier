use crate::hardfork::{SCROLL_MAINNET_CHAIN_ID, SCROLL_TESTNET_CHAIN_ID};
use once_cell::sync::Lazy;
use revm::primitives::{poseidon, KECCAK_EMPTY, POSEIDON_EMPTY};
use sbv_primitives::{
    alloy_primitives::{keccak256, Bytes},
    zk_trie::{
        db::{kv::KVDatabase, NodeDb},
        hash::{key_hasher::KeyHasher, HashScheme},
        scroll_types::Account,
        trie::{ZkTrie, ZkTrieError},
    },
    Address, B256, U256,
};
use serde::Deserialize;
use std::borrow::Cow;
use std::collections::HashMap;

static SCROLL_MAINNET_GENESIS: Lazy<GethGenesisConfig> = Lazy::new(|| {
    serde_json::from_str(include_str!("./data/genesis/genesis.mainnet.json")).unwrap()
});

static SCROLL_TESTNET_GENESIS: Lazy<GethGenesisConfig> = Lazy::new(|| {
    serde_json::from_str(include_str!("./data/genesis/genesis.sepolia.json")).unwrap()
});

/// Genesis configuration for Scroll networks.
#[derive(Debug)]
pub struct GenesisConfig {
    config: Cow<'static, GethGenesisConfig>,
}

impl GenesisConfig {
    /// Create a new genesis configuration from the given chain ID.
    pub fn default_from_chain_id(chain_id: u64) -> Self {
        match chain_id {
            SCROLL_MAINNET_CHAIN_ID => Self::mainnet(),
            SCROLL_TESTNET_CHAIN_ID => Self::testnet(),
            _ => panic!("unsupported chain id: {}", chain_id),
        }
    }

    /// Create a new mainnet genesis configuration.
    pub fn mainnet() -> Self {
        Self {
            config: Cow::Borrowed(&*SCROLL_MAINNET_GENESIS),
        }
    }

    /// Create a new testnet genesis configuration.
    pub fn testnet() -> Self {
        Self {
            config: Cow::Borrowed(&*SCROLL_TESTNET_GENESIS),
        }
    }

    /// Initialize the code database with the code of the accounts.
    pub fn init_code_db<Db: KVDatabase>(&self, code_db: &mut Db) -> Result<(), Db::Error> {
        for acc in self.config.alloc.values() {
            if acc.code.is_empty() {
                continue;
            }

            let code_hash = keccak256(&acc.code);
            code_db.put(code_hash.as_ref(), acc.code.as_ref())?;
        }

        code_db.put(KECCAK_EMPTY.as_ref(), &[])?;

        Ok(())
    }

    /// Initialize the zkTrie with the accounts.
    pub fn init_zktrie<H: HashScheme, ZkDb: KVDatabase, K: KeyHasher<H> + Clone>(
        &self,
        db: &mut NodeDb<ZkDb>,
        key_hasher: K,
    ) -> Result<ZkTrie<H, K>, ZkTrieError<H::Error, ZkDb::Error>> {
        let mut zktrie = ZkTrie::<H, _>::new(key_hasher.clone());
        for (addr, acc) in self.config.alloc.iter() {
            let storage_root = if !acc.storage.is_empty() {
                let mut storage_trie = ZkTrie::<H, _>::new(key_hasher.clone());
                for (key, value) in acc.storage.iter() {
                    storage_trie.update(db, key.to_be_bytes::<32>(), value)?;
                }
                storage_trie.commit(db)?;
                *storage_trie.root().unwrap_ref()
            } else {
                B256::ZERO
            };

            zktrie.update(
                db,
                addr,
                Account {
                    nonce: 0,
                    code_size: acc.code.len() as u64,
                    balance: acc.balance,
                    storage_root,
                    code_hash: if acc.code.is_empty() {
                        KECCAK_EMPTY
                    } else {
                        keccak256(&acc.code)
                    },
                    poseidon_code_hash: if acc.code.is_empty() {
                        POSEIDON_EMPTY
                    } else {
                        poseidon(&acc.code)
                    },
                },
            )?;
        }
        zktrie.commit(db)?;

        Ok(zktrie)
    }

    /// Get the genesis coinbase configuration.
    #[inline(always)]
    pub fn coinbase(&self) -> Address {
        self.config.config.scroll.fee_vault_address
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GethGenesisConfig {
    pub config: GethGenesisBaseConfig,
    // pub timestamp: U256,
    // pub extra_data: Bytes,
    // pub gas_limit: U256,
    // pub coinbase: Address,
    pub alloc: HashMap<Address, AllocAccount>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GethGenesisBaseConfig {
    // pub chain_id: ChainId,
    pub scroll: ScrollGenesisConfig,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScrollGenesisConfig {
    // pub max_tx_per_block: usize,
    // pub max_tx_payload_bytes_per_block: usize,
    pub fee_vault_address: Address,
    // pub l1_config: ScrollL1Config,
}

// #[derive(Clone, Debug, Deserialize)]
// #[serde(rename_all = "camelCase")]
// pub struct ScrollL1Config {
//     pub l1_chain_id: U64,
//     pub l1_message_queue_address: Address,
//     pub num_l1_messages_per_block: U64,
// }

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AllocAccount {
    #[serde(default)]
    pub balance: U256,
    #[serde(default)]
    pub code: Bytes,
    #[serde(default)]
    pub storage: HashMap<U256, U256>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_genesis() {
        let _ = SCROLL_MAINNET_GENESIS.clone();
        let _ = SCROLL_TESTNET_GENESIS.clone();
    }
}
