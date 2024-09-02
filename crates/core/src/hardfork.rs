use eth_types::{
    forks::{hardfork_heights, HardforkId},
    l2_predeployed::l1_gas_price_oracle,
};
use itertools::Itertools;
use revm::{
    primitives::{Account, AccountStatus, Address, Bytecode, Bytes, EvmStorageSlot, SpecId, U256},
    Database, DatabaseCommit,
};
use std::{collections::HashMap, sync::LazyLock};

/// Hardfork heights for Scroll networks, grouped by chain id.
static HARDFORK_HEIGHTS: LazyLock<HashMap<u64, HashMap<SpecId, u64>>> = LazyLock::new(|| {
    #[allow(clippy::let_and_return)]
    let heights = hardfork_heights()
        .into_iter()
        .sorted_by_key(|(_, chain_id, _)| *chain_id)
        .chunk_by(|(_, chain_id, _)| *chain_id)
        .into_iter()
        .map(|(chain_id, slice)| {
            (
                chain_id,
                slice
                    .map(|(fork_id, _chain_id, height)| {
                        let fork_id = match fork_id {
                            HardforkId::Bernoulli => SpecId::BERNOULLI,
                            HardforkId::Curie => SpecId::CURIE,
                        };
                        (fork_id, height)
                    })
                    .collect::<HashMap<_, _>>(),
            )
        })
        .collect();

    dev_info!("Hardfork heights: {:#?}", heights);
    #[allow(clippy::let_and_return)]
    heights
});

/// Hardfork configuration for Scroll networks.
#[derive(Debug, Default, Copy, Clone)]
pub struct HardforkConfig {
    bernoulli_block: u64,
    curie_block: u64,
}

impl HardforkConfig {
    /// Get the default hardfork configuration for a chain id.
    pub fn default_from_chain_id(chain_id: u64) -> Self {
        if let Some(heights) = HARDFORK_HEIGHTS.get(&chain_id) {
            Self {
                bernoulli_block: heights.get(&SpecId::BERNOULLI).copied().unwrap_or(0),
                curie_block: heights.get(&SpecId::CURIE).copied().unwrap_or(0),
            }
        } else {
            dev_warn!(
                "Chain id {} not found in hardfork heights, all forks are enabled by default",
                chain_id
            );
            Self::default()
        }
    }

    /// Set the Bernoulli block number.
    pub fn set_bernoulli_block(&mut self, bernoulli_block: u64) -> &mut Self {
        self.bernoulli_block = bernoulli_block;
        self
    }

    /// Set the Curie block number.
    pub fn set_curie_block(&mut self, curie_block: u64) -> &mut Self {
        self.curie_block = curie_block;
        self
    }

    /// Get the hardfork spec id for a block number.
    pub fn get_spec_id(&self, block_number: u64) -> SpecId {
        match block_number {
            n if n < self.bernoulli_block => SpecId::PRE_BERNOULLI,
            n if n < self.curie_block => SpecId::BERNOULLI,
            _ => SpecId::CURIE,
        }
    }

    /// Migrate the database to a new hardfork.
    pub fn migrate<DB: Database + DatabaseCommit>(
        &self,
        block_number: u64,
        db: &mut DB,
    ) -> Result<(), DB::Error> {
        if block_number == self.curie_block {
            dev_info!("Apply curie migrate at height #{}", block_number);
            self.curie_migrate(db)?;
        };
        Ok(())
    }

    fn curie_migrate<DB: Database + DatabaseCommit>(&self, db: &mut DB) -> Result<(), DB::Error> {
        let l1_gas_price_oracle_addr = Address::from(l1_gas_price_oracle::ADDRESS.0);
        let mut l1_gas_price_oracle_info = db.basic(l1_gas_price_oracle_addr)?.unwrap_or_default();
        // Set the new code
        let code = Bytecode::new_raw(Bytes::copy_from_slice(
            l1_gas_price_oracle::V2_BYTECODE.as_slice(),
        ));
        l1_gas_price_oracle_info.code_size = code.len();
        l1_gas_price_oracle_info.code_hash = code.hash_slow();
        l1_gas_price_oracle_info.poseidon_code_hash = code.poseidon_hash_slow();
        l1_gas_price_oracle_info.code = Some(code);

        let l1_gas_price_oracle_acc = Account {
            info: l1_gas_price_oracle_info,
            storage: HashMap::from([
                (
                    U256::from_limbs(l1_gas_price_oracle::IS_CURIE_SLOT.0),
                    EvmStorageSlot::new(U256::from(1)),
                ),
                (
                    U256::from_limbs(l1_gas_price_oracle::L1_BLOB_BASEFEE_SLOT.0),
                    EvmStorageSlot::new(U256::from(1)),
                ),
                (
                    U256::from_limbs(l1_gas_price_oracle::COMMIT_SCALAR_SLOT.0),
                    EvmStorageSlot::new(U256::from_limbs(
                        l1_gas_price_oracle::INITIAL_COMMIT_SCALAR.0,
                    )),
                ),
                (
                    U256::from_limbs(l1_gas_price_oracle::BLOB_SCALAR_SLOT.0),
                    EvmStorageSlot::new(U256::from_limbs(
                        l1_gas_price_oracle::INITIAL_BLOB_SCALAR.0,
                    )),
                ),
            ]),
            status: AccountStatus::Touched,
        };

        db.commit(HashMap::from([(
            l1_gas_price_oracle_addr,
            l1_gas_price_oracle_acc,
        )]));

        Ok(())
    }
}
