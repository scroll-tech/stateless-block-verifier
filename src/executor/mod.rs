use crate::database::ReadOnlyDB;
use eth_types::{geth_types::TxType, Signature, ToBigEndian, H160, H256, U256};
use ethers_core::{
    types::{transaction::eip2718::TypedTransaction, RecoveryMessage},
    utils::{hash_message, keccak256},
};
use mpt_zktrie::AccountData;
use revm::{
    db::CacheDB,
    primitives::{AccountInfo, Env, SpecId},
};
use secp256k1::{
    ecdsa::{RecoverableSignature, RecoveryId},
    Message, Secp256k1,
};
use std::fmt::Debug;
use zktrie::ZkTrie;

mod builder;
/// Execute hooks
pub mod hooks;
use crate::utils::ext::{BlockTraceRevmExt, TxRevmExt};
use crate::{cycle_tracker_end, cycle_tracker_start};
pub use builder::EvmExecutorBuilder;

/// EVM executor that handles the block.
pub struct EvmExecutor {
    db: CacheDB<ReadOnlyDB>,
    zktrie: ZkTrie,
    spec_id: SpecId,
    hooks: hooks::ExecuteHooks,
}
impl EvmExecutor {
    /// Get reference to the DB
    pub fn db(&self) -> &CacheDB<ReadOnlyDB> {
        &self.db
    }

    /// Handle a block.
    pub fn handle_block<T: BlockTraceRevmExt>(&mut self, l2_trace: &T) -> H256 {
        debug!("handle block {:?}", l2_trace.number());
        let mut env = Box::<Env>::default();
        env.cfg.chain_id = l2_trace.chain_id();
        cycle_tracker_start!("create BlockEnv");
        env.block = l2_trace.env();
        cycle_tracker_end!("create BlockEnv");

        for (idx, tx) in l2_trace.transactions().enumerate() {
            cycle_tracker_start!("handle tx {}", idx);
            trace!("handle {idx}th tx");
            trace!("{tx:#?}");
            let mut env = env.clone();
            env.tx = tx.tx_env();
            if tx.raw_type() == 0 {
                env.tx.chain_id = Some(l2_trace.chain_id());
            }
            let eth_tx = &tx.to_eth_tx(
                l2_trace.block_hash(),
                l2_trace.number(),
                idx,
                l2_trace.base_fee_per_gas(),
            );

            // let recovered_address = eth_tx.recover_from().unwrap();

            // // verify that the transaction is valid
            // if recovered_address != eth_tx.from {
            //     panic!(
            //         "Invalid transaction: tx.from = {:?}, recover(tx.from) = {:?}",
            //         eth_tx.from, recovered_address
            //     );
            // }

            let sig = Signature {
                r: eth_tx.r,
                s: eth_tx.s,
                v: eth_tx.v.as_u64(),
            };

            // let digest: Transaction
            let typed_tx: TypedTransaction = eth_tx.into();
            let typed_tx_hash = typed_tx.sighash();
            let msg = Message::from_digest(typed_tx_hash.0);

            let mut sig_r_le = sig.r.to_be_bytes();
            let mut sig_s_le = sig.s.to_be_bytes();

            let mut sig_le_bytes = [0u8; 64];
            sig_le_bytes[..32].copy_from_slice(&sig_r_le);
            sig_le_bytes[32..64].copy_from_slice(&sig_s_le);

            let recovery_id = RecoveryId::from_i32(sig.v as i32).unwrap();
            let signature = RecoverableSignature::from_compact(&sig_le_bytes, recovery_id).unwrap();

            // The recovery ID is the last byte of the signature.

            let secp = Secp256k1::new();

            let result = secp.recover_ecdsa(&msg, &signature);

            let secp = Secp256k1::new();
            let result = secp.recover_ecdsa(&msg, &signature);

            match result {
                Ok(public_key) => {
                    // Convert the public key to an Ethereum address
                    let public_key_bytes = public_key.serialize_uncompressed();
                    let hash = keccak256(&public_key_bytes[1..]); // Skip the first byte (0x04) which indicates uncompressed key
                    let recovered_address = H160::from_slice(&hash[12..]); // Take the last 20 bytes

                    // Compare the recovered address with eth_tx.from
                    if recovered_address != eth_tx.from {
                        panic!(
                            "Invalid transaction: tx.from = {:?}, recovered address = {:?}",
                            eth_tx.from, recovered_address
                        );
                    } else {
                        println!("Transaction signature verified successfully");
                    }
                }
                Err(e) => {
                    panic!("Failed to recover public key: {:?}", e);
                }
            }

            // Continue with the rest of your transaction processing...

            // let message = typed_tx_hash.into();
            // let message_hash = match message {
            //     RecoveryMessage::Data(ref message) => hash_message(message),
            //     RecoveryMessage::Hash(hash) => hash,
            // };
            // // Reverse the first 32 bytes (r) and the second 32 bytes (s) of the signature
            // // and concatenate them to get the signature in big-endian format.

            // let verifying_key = sp1_ecdsa::VerifyingKey::recover_from_prehash(
            //     typed_tx_hash.as_bytes(),
            //     &signature,
            //     recovery_id,
            // )
            // .unwrap();
            // let verifying_key_bytes = {
            //     let bytes = verifying_key.to_encoded_point(false).to_bytes();
            //     let mut array = [0u8; 64];
            //     array.copy_from_slice(&bytes[1..65]);
            //     array
            // };

            // unsafe {
            //     let k = key::PublicKey(crate::ffi::PublicKey::from_array_unchecked(
            //         verifying_key_bytes,
            //     ));
            //     return Ok(k);
            // }

            let tx_type = TxType::get_tx_type(&eth_tx);
            if tx_type.is_l1_msg() {
                env.tx.nonce = None; // clear nonce for l1 msg
                env.cfg.disable_base_fee = true; // disable base fee for l1 msg
            }
            env.tx.scroll.is_l1_msg = tx_type.is_l1_msg();
            env.tx.scroll.rlp_bytes = Some(revm::primitives::Bytes::from(eth_tx.rlp().to_vec()));
            trace!("{env:#?}");
            {
                cycle_tracker_start!("build Evm");
                let mut revm = revm::Evm::builder()
                    .with_spec_id(self.spec_id)
                    .with_db(&mut self.db)
                    .with_env(env)
                    .build();
                cycle_tracker_end!("build Evm");

                trace!("handler cfg: {:?}", revm.handler.cfg);

                cycle_tracker_start!("transact_commit");
                let result = revm.transact_commit().unwrap(); // TODO: handle error
                cycle_tracker_end!("transact_commit");
                trace!("{result:#?}");
            }
            self.hooks.post_tx_execution(self, idx);
            debug!("handle {idx}th tx done");
            cycle_tracker_end!("handle tx {}", idx);
        }
        cycle_tracker_start!("commit_changes");
        self.commit_changes();
        cycle_tracker_end!("commit_changes");
        H256::from(self.zktrie.root())
    }

    fn commit_changes(&mut self) {
        // let changes = self.db.accounts;
        let sdb = &self.db.db.sdb;

        #[cfg(any(feature = "debug-account", feature = "debug-storage"))]
        std::fs::create_dir_all("/tmp/sbv-debug").expect("failed to create debug dir");

        #[cfg(feature = "debug-account")]
        let mut debug_account = std::collections::BTreeMap::new();

        for (addr, db_acc) in self.db.accounts.iter() {
            let Some(info): Option<AccountInfo> = db_acc.info() else {
                continue;
            };
            let (_, acc) = sdb.get_account(&H160::from(*addr.0));
            if acc.is_empty() && info.is_empty() {
                continue;
            }
            trace!("committing {addr}, {:?} {db_acc:?}", db_acc.account_state);
            cycle_tracker_start!("commit account {}", addr);

            cycle_tracker_start!("get acc_data");
            let mut acc_data = self
                .zktrie
                .get_account(addr.as_slice())
                .map(AccountData::from)
                .unwrap_or_default();
            cycle_tracker_end!("get acc_data");

            acc_data.nonce = info.nonce;
            acc_data.balance = U256(*info.balance.as_limbs());
            if !db_acc.storage.is_empty() {
                #[cfg(feature = "debug-storage")]
                let mut debug_storage = std::collections::BTreeMap::new();

                #[cfg(feature = "debug-storage")]
                #[derive(serde::Serialize)]
                struct StorageOps {
                    kind: &'static str,
                    key: revm::primitives::U256,
                    value: Option<revm::primitives::U256>,
                }

                // get current storage root
                let storage_root_before = acc_data.storage_root;
                // get storage tire
                cycle_tracker_start!("update storage_tire");
                let mut storage_tire = self
                    .zktrie
                    .get_db()
                    .new_trie(storage_root_before.as_fixed_bytes())
                    .expect("unable to get storage trie");
                for (key, value) in db_acc.storage.iter() {
                    if !value.is_zero() {
                        cycle_tracker_start!("Zktrie::update_store");
                        storage_tire
                            .update_store(&key.to_be_bytes::<32>(), &value.to_be_bytes())
                            .expect("failed to update storage");
                        cycle_tracker_end!("Zktrie::update_store");

                        #[cfg(feature = "debug-storage")]
                        debug_storage.insert(
                            *key,
                            StorageOps {
                                kind: "update",
                                key: *key,
                                value: Some(*value),
                            },
                        );
                    } else {
                        cycle_tracker_start!("Zktrie::delete");
                        storage_tire.delete(&key.to_be_bytes::<32>());
                        cycle_tracker_end!("Zktrie::delete");

                        #[cfg(feature = "debug-storage")]
                        debug_storage.insert(
                            *key,
                            StorageOps {
                                kind: "delete",
                                key: *key,
                                value: None,
                            },
                        );
                    }
                }
                cycle_tracker_end!("update storage_tire");
                acc_data.storage_root = H256::from(storage_tire.root());

                #[cfg(feature = "debug-storage")]
                {
                    let output = std::fs::File::create(format!(
                        "/tmp/sbv-debug/storage_{:?}_{:?}.csv",
                        addr, acc_data.storage_root
                    ))
                    .expect("failed to create debug file");
                    let mut wtr = csv::Writer::from_writer(output);
                    for ops in debug_storage.into_values() {
                        wtr.serialize(ops).expect("failed to write record");
                    }
                }
            }
            if (acc.is_empty() && !info.is_empty()) || acc.code_hash.0 != info.code_hash.0 {
                acc_data.poseidon_code_hash = H256::from(info.code_hash.0);
                acc_data.keccak_code_hash = H256::from(info.keccak_code_hash.0);
                acc_data.code_size = self
                    .db
                    .contracts
                    .get(&db_acc.info.code_hash)
                    .map(|c| c.len())
                    .unwrap_or_default() as u64;
            }

            #[cfg(feature = "debug-account")]
            debug_account.insert(*addr, acc_data.clone());

            cycle_tracker_start!("Zktrie::update_account");
            self.zktrie
                .update_account(addr.as_slice(), &acc_data.into())
                .expect("failed to update account");
            cycle_tracker_end!("Zktrie::update_account");

            cycle_tracker_end!("commit account {}", addr);
        }

        #[cfg(feature = "debug-account")]
        {
            let output = std::fs::File::create(format!(
                "/tmp/sbv-debug/account_0x{}.csv",
                hex::encode(self.zktrie.root())
            ))
            .expect("failed to create debug file");
            let mut wtr = csv::Writer::from_writer(output);

            #[derive(serde::Serialize)]
            pub struct AccountData {
                addr: revm::primitives::Address,
                nonce: u64,
                balance: U256,
                keccak_code_hash: H256,
                poseidon_code_hash: H256,
                code_size: u64,
                storage_root: H256,
            }

            for (addr, acc) in debug_account.into_iter() {
                wtr.serialize(AccountData {
                    addr,
                    nonce: acc.nonce,
                    balance: acc.balance,
                    keccak_code_hash: acc.keccak_code_hash,
                    poseidon_code_hash: acc.poseidon_code_hash,
                    code_size: acc.code_size,
                    storage_root: acc.storage_root,
                })
                .expect("failed to write record");
            }
        }
    }
}

impl Debug for EvmExecutor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EvmExecutor")
            .field("db", &self.db)
            .field("zktrie", &self.zktrie.root())
            .finish()
    }
}

// fn recover<C: Verification>(
//     secp: &Secp256k1<C>,
//     msg: &[u8],
//     sig: [u8; 64],
//     recovery_id: u8,
// ) -> Result<PublicKey, Error> {
//     let msg = sha256::Hash::hash(msg);
//     let msg = Message::from_digest_slice(msg.as_ref())?;
//     let id = ecdsa::RecoveryId::from_i32(recovery_id as i32)?;
//     let sig = ecdsa::RecoverableSignature::from_compact(&sig, id)?;

//     secp.recover_ecdsa(&msg, &sig)
// }
