use crate::hardfork::MigrateChanges;
use revm::primitives::{
    Account, AccountStatus, Bytecode, Bytes, EvmStorage, EvmStorageSlot, SpecId,
};
use revm::DatabaseRef;
use sbv_primitives::predeployed::l1_gas_price_oracle;
use sbv_primitives::{Address, BlockNumber, ChainId, U256};
use std::collections::HashMap;
use std::convert::Infallible;

/// Scroll devnet chain id
pub const SCROLL_DEVNET_CHAIN_ID: u64 = 222222;
/// Scroll testnet chain id
pub const SCROLL_TESTNET_CHAIN_ID: u64 = 534351;
/// Scroll mainnet chain id
pub const SCROLL_MAINNET_CHAIN_ID: u64 = 534352;

fn curie_migrate(
    db: &dyn DatabaseRef<Error = Infallible>,
) -> revm::primitives::HashMap<Address, Account> {
    let l1_gas_price_oracle_addr = Address::from(l1_gas_price_oracle::ADDRESS.0);
    let mut l1_gas_price_oracle_info = db
        .basic_ref(l1_gas_price_oracle_addr)
        .unwrap()
        .unwrap_or_default();
    // Set the new code
    let code = Bytecode::new_raw(Bytes::from_static(l1_gas_price_oracle::V2_BYTECODE));
    l1_gas_price_oracle_info.code_size = code.len();
    l1_gas_price_oracle_info.code_hash = code.hash_slow();
    l1_gas_price_oracle_info.poseidon_code_hash = code.poseidon_hash_slow();
    l1_gas_price_oracle_info.code = Some(code);

    let l1_gas_price_oracle_acc = Account {
        info: l1_gas_price_oracle_info,
        storage: EvmStorage::from_iter([
            (
                l1_gas_price_oracle::IS_CURIE_SLOT,
                EvmStorageSlot::new(U256::from(1)),
            ),
            (
                l1_gas_price_oracle::L1_BLOB_BASEFEE_SLOT,
                EvmStorageSlot::new(U256::from(1)),
            ),
            (
                l1_gas_price_oracle::COMMIT_SCALAR_SLOT,
                EvmStorageSlot::new(l1_gas_price_oracle::INITIAL_COMMIT_SCALAR),
            ),
            (
                l1_gas_price_oracle::BLOB_SCALAR_SLOT,
                EvmStorageSlot::new(l1_gas_price_oracle::INITIAL_BLOB_SCALAR),
            ),
        ]),
        status: AccountStatus::Touched,
    };

    revm::primitives::HashMap::from_iter([(l1_gas_price_oracle_addr, l1_gas_price_oracle_acc)])
}

pub(super) fn add_hardforks(map: &mut HashMap<ChainId, HashMap<SpecId, BlockNumber>>) {
    map.insert(
        SCROLL_DEVNET_CHAIN_ID,
        HashMap::from([
            (SpecId::BERNOULLI, 0),
            (SpecId::CURIE, 5),
            (SpecId::EUCLID, u64::MAX),
        ]),
    );
    map.insert(
        SCROLL_TESTNET_CHAIN_ID,
        HashMap::from([
            (SpecId::BERNOULLI, 3747132),
            (SpecId::CURIE, 4740239),
            (SpecId::EUCLID, u64::MAX),
        ]),
    );
    map.insert(
        SCROLL_MAINNET_CHAIN_ID,
        HashMap::from([
            (SpecId::BERNOULLI, 5220340),
            (SpecId::CURIE, 7096836),
            (SpecId::EUCLID, u64::MAX),
        ]),
    );
}

pub(super) fn add_migrates(map: &mut HashMap<SpecId, MigrateChanges>) {
    map.insert(SpecId::CURIE, Box::new(curie_migrate));
}
