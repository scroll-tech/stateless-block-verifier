use revm::{
    primitives::{
        Account, AccountStatus, Address, Bytecode, Bytes, EvmStorage, EvmStorageSlot, SpecId, U256,
    },
    Database, DatabaseCommit, DatabaseRef,
};
use sbv_primitives::{predeployed::l1_gas_price_oracle, BlockNumber, ChainId};
use std::collections::HashMap;
use std::convert::Infallible;
use std::sync::LazyLock;

pub type MigrateChanges = Box<
    dyn Fn(&dyn DatabaseRef<Error = Infallible>) -> revm::primitives::HashMap<Address, Account>
        + Send
        + Sync
        + 'static,
>;

#[cfg(feature = "scroll")]
mod scroll;

/// Hardfork heights for networks, grouped by chain id.
static HARDFORK_HEIGHTS: LazyLock<HashMap<ChainId, HashMap<SpecId, BlockNumber>>> =
    LazyLock::new(|| {
        let mut map = HashMap::new();

        #[cfg(feature = "scroll")]
        scroll::add_hardforks(&mut map);

        map
    });

/// Hardfork migrate changes for networks, grouped by spec id.
static HARDFORK_MIGRATES: LazyLock<HashMap<SpecId, MigrateChanges>> = LazyLock::new(|| {
    let mut map = HashMap::new();

    #[cfg(feature = "scroll")]
    scroll::add_migrates(&mut map);

    map
});

/// Hardfork configuration for Scroll networks.
#[derive(Debug, Clone)]
pub struct HardforkConfig {
    heights: &'static HashMap<SpecId, BlockNumber>,
    overrides: HashMap<SpecId, BlockNumber>,
}

static DEFAULT_HEIGHTS: LazyLock<HashMap<SpecId, u64>> = LazyLock::new(HashMap::new);

static AVAILABLE_SPEC_IDS: LazyLock<Vec<SpecId>> =
    LazyLock::new(|| (0..u8::MAX).rev().flat_map(SpecId::try_from_u8).collect());

impl HardforkConfig {
    /// Get the default hardfork configuration for a chain id.
    pub fn default_from_chain_id(chain_id: u64) -> Self {
        let heights = HARDFORK_HEIGHTS.get(&chain_id).unwrap_or_else(|| {
            dev_warn!(
                "Chain id {} not found in hardfork heights, all forks are enabled by default",
                chain_id
            );
            &DEFAULT_HEIGHTS
        });

        Self {
            heights,
            overrides: HashMap::new(),
        }
    }

    /// Set the specified hardfork height.
    pub fn set_height(&mut self, spec_id: SpecId, block: u64) -> &mut Self {
        self.overrides.insert(spec_id, block);
        self
    }

    /// Get the hardfork height for a spec id.
    pub fn get_height(&self, spec_id: SpecId) -> Option<u64> {
        self.overrides
            .get(&spec_id)
            .copied()
            .or_else(|| self.heights.get(&spec_id).copied())
    }

    /// Get the hardfork spec id for a block number.
    pub fn get_spec_id(&self, block_number: u64) -> SpecId {
        for spec_id in AVAILABLE_SPEC_IDS.iter() {
            if let Some(height) = self.get_height(*spec_id) {
                if block_number >= height {
                    return *spec_id;
                }
            }
        }
        SpecId::LATEST
    }

    /// Migrate the database to a new hardfork.
    pub fn migrate<DB: DatabaseRef>(
        &self,
        block_number: u64,
        db: &DB,
    ) -> Result<Option<revm::primitives::HashMap<Address, Account>>, DB::Error> {
        let spec_id = self.get_spec_id(block_number);
        if let Some(height) = self.get_height(spec_id) {
            if block_number == height {
                if let Some(migrate) = HARDFORK_MIGRATES.get(&spec_id) {
                    return Ok(Some(migrate(db)));
                }
            }
        }
        Ok(None)
    }
}
