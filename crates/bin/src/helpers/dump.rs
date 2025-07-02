use sbv::primitives::{Address, B256, Bytes, U256, types::revm::database::BundleState};
use serde::Serialize;
use serde_json::json;
use std::{
    collections::BTreeMap,
    fs::{self, File},
    path::Path,
};

#[derive(Serialize)]
struct AccountChanged {
    kind: &'static str,
    address: Address,
    balance: U256,
    nonce: u64,
    code_hash: B256,
    code: Bytes,
}

#[derive(Serialize)]
struct StorageChanged {
    address: Address,
    key: U256,
    previous_or_original_value: U256,
    present_value: U256,
}

#[derive(Serialize)]
struct ContractCreated {
    code_hash: B256,
    code_size: usize,
    code: Bytes,
}

pub fn dump_bundle_state(bundle_state: &BundleState, out_dir: &Path) -> anyhow::Result<()> {
    fs::create_dir_all(out_dir)
        .inspect_err(|_e| dev_error!("Failed to create output directory: {_e}"))?;

    serde_json::to_writer_pretty(
        File::create(out_dir.join("bundle-state.json"))?,
        &json!({
            "states_changed": bundle_state.state.len(),
            "contracts_created": bundle_state.contracts.len(),
            "state_size": bundle_state.state_size,
            "reverts_size": bundle_state.reverts_size,
        }),
    )
    .inspect_err(|_e| {
        dev_error!("Failed to write bundle state summary: {_e}");
    })?;

    let mut states_changed = csv::Writer::from_writer(
        File::create(&out_dir.join("states-changed.csv"))
            .inspect_err(|_e| dev_error!("Failed to create states-changed.csv: {_e}"))?,
    );
    let mut storages_changed = csv::Writer::from_writer(
        File::create(&out_dir.join("storage-changed.csv"))
            .inspect_err(|_e| dev_error!("Failed to create storage-changed.csv: {_e}"))?,
    );

    for (address, account) in BTreeMap::from_iter(bundle_state.state.clone()).into_iter() {
        let original = account.original_info.unwrap();
        let after = account.info.unwrap();
        if original != after {
            states_changed
                .serialize(AccountChanged {
                    address,
                    kind: "before",
                    balance: original.balance,
                    nonce: original.nonce,
                    code_hash: original.code_hash,
                    code: original.code.unwrap_or_default().original_bytes(),
                })
                .inspect_err(|_e| {
                    dev_error!("Failed to serialize before state for address {address:?}: {_e}");
                })?;
            states_changed
                .serialize(AccountChanged {
                    kind: "after",
                    address,
                    balance: after.balance,
                    nonce: after.nonce,
                    code_hash: after.code_hash,
                    code: after.code.unwrap_or_default().original_bytes(),
                })
                .inspect_err(|_e| {
                    dev_error!("Failed to serialize after state for address {address:?}: {_e}");
                })?;
        }

        for (key, slot) in BTreeMap::from_iter(account.storage).into_iter() {
            storages_changed.serialize(StorageChanged {
                address,
                key,
                previous_or_original_value: slot.previous_or_original_value,
                present_value: slot.present_value,
            })
                .inspect_err(|_e| {
                    dev_error!("Failed to serialize storage change for address {address:?}, key {key:?}: {_e}");
                })?;
        }
    }

    let mut contracts = csv::Writer::from_writer(
        File::create(&out_dir.join("contracts.csv"))
            .inspect_err(|_e| dev_error!("Failed to create contracts.csv: {_e}"))?,
    );

    for (hash, code) in BTreeMap::from_iter(bundle_state.contracts.clone()).into_iter() {
        contracts.serialize(ContractCreated {
            code_hash: hash,
            code_size: code.len(),
            code: code.original_bytes(),
        })?;
    }
    Ok(())
}
