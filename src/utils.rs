use eth_types::forks::{hardfork_heights, HardforkId};
use eth_types::l2_predeployed::l1_gas_price_oracle;
use eth_types::{l2_types::StorageTrace, H256};
use revm::primitives::{Account, Address, Bytecode, Bytes, SpecId, StorageSlot, U256};
use revm::{Database, DatabaseCommit};
use std::collections::HashMap;
use std::sync::LazyLock;

/// Hardfork heights for Scroll networks, grouped by chain id.
pub(crate) static HARDFORK_HEIGHTS: LazyLock<HashMap<u64, HashMap<SpecId, u64>>> =
    LazyLock::new(|| {
        hardfork_heights()
            .group_by(|a, b| a.1 == b.1)
            .map(|slice| {
                let chain_id = slice[0].1;

                (
                    chain_id,
                    slice
                        .iter()
                        .map(|(fork_id, _chain_id, height)| {
                            let fork_id = match fork_id {
                                HardforkId::Curie => SpecId::CURIE,
                            };
                            (fork_id, *height)
                        })
                        .collect::<HashMap<_, _>>(),
                )
            })
            .collect()
    });

/// Hardfork configuration for Scroll networks.
#[derive(Debug, Default, Copy, Clone)]
pub struct HardforkConfig {
    curie_block: u64,
}

impl HardforkConfig {
    /// Get the default hardfork configuration for a chain id.
    pub fn default_from_chain_id(chain_id: u64) -> Self {
        if let Some(heights) = HARDFORK_HEIGHTS.get(&chain_id) {
            Self {
                curie_block: heights.get(&SpecId::CURIE).copied().unwrap_or(0),
            }
        } else {
            warn!("Chain id {} not found in hardfork heights", chain_id);
            Self::default()
        }
    }

    /// Set the Curie block number.
    pub fn set_curie_block(&mut self, curie_block: u64) -> &mut Self {
        self.curie_block = curie_block;
        self
    }

    /// Get the hardfork spec id for a block number.
    pub fn get_spec_id(&self, block_number: u64) -> SpecId {
        if block_number < self.curie_block {
            SpecId::BERNOULLI
        } else {
            SpecId::CURIE
        }
    }

    /// Migrate the database to a new hardfork.
    pub fn migrate<DB: Database + DatabaseCommit>(
        &self,
        block_number: u64,
        db: &mut DB,
    ) -> Result<(), DB::Error> {
        if block_number == self.curie_block {
            let l1_gas_price_oracle_addr = Address::from(l1_gas_price_oracle::ADDRESS.0);
            let mut l1_gas_price_oracle_info =
                db.basic(l1_gas_price_oracle_addr)?.unwrap_or_default();
            // Set the new code
            l1_gas_price_oracle_info.set_code_rehash_slow(Some(Bytecode::new_raw(
                Bytes::copy_from_slice(l1_gas_price_oracle::V2_BYTECODE.as_slice()),
            )));
            let mut l1_gas_price_oracle_acc = Account::from(l1_gas_price_oracle_info);
            l1_gas_price_oracle_acc.storage.insert(
                U256::from_limbs(l1_gas_price_oracle::IS_CURIE_SLOT.0),
                StorageSlot::new(U256::from(1)),
            );
            l1_gas_price_oracle_acc.storage.insert(
                U256::from_limbs(l1_gas_price_oracle::L1_BLOB_BASEFEE_SLOT.0),
                StorageSlot::new(U256::from(1)),
            );
            l1_gas_price_oracle_acc.storage.insert(
                U256::from_limbs(l1_gas_price_oracle::COMMIT_SCALAR_SLOT.0),
                StorageSlot::new(U256::from_limbs(
                    l1_gas_price_oracle::INITIAL_COMMIT_SCALAR.0,
                )),
            );
            l1_gas_price_oracle_acc.storage.insert(
                U256::from_limbs(l1_gas_price_oracle::BLOB_SCALAR_SLOT.0),
                StorageSlot::new(U256::from_limbs(l1_gas_price_oracle::INITIAL_BLOB_SCALAR.0)),
            );

            db.commit(HashMap::from([(
                l1_gas_price_oracle_addr,
                l1_gas_price_oracle_acc,
            )]));
        };
        Ok(())
    }
}

pub(crate) fn collect_account_proofs(
    storage_trace: &StorageTrace,
) -> impl Iterator<Item = (&eth_types::Address, impl IntoIterator<Item = &[u8]>)> + Clone {
    storage_trace.proofs.iter().flat_map(|kv_map| {
        kv_map
            .iter()
            .map(|(k, bts)| (k, bts.iter().map(|b| b.as_ref())))
    })
}

pub(crate) fn collect_storage_proofs(
    storage_trace: &StorageTrace,
) -> impl Iterator<Item = (&eth_types::Address, &H256, impl IntoIterator<Item = &[u8]>)> + Clone {
    storage_trace.storage_proofs.iter().flat_map(|(k, kv_map)| {
        kv_map
            .iter()
            .map(move |(sk, bts)| (k, sk, bts.iter().map(|b| b.as_ref())))
    })
}
