// TODO

// #[cfg(test)]
// #[cfg(not(feature = "scroll"))]
// mod tests {
//     use sbv_primitives::{
//         chainspec::{Chain, get_chain_spec},
//         types::BlockWitness,
//     };
//
//     #[rstest::rstest]
//     fn test_mainnet(
//         #[files("../../testdata/holesky_witness/**/*.json")]
//         #[mode = str]
//         witness_json: &str,
//     ) {
//         let witness: BlockWitness = serde_json::from_str(witness_json).unwrap();
//         let chain_spec = get_chain_spec(Chain::from_id(witness.chain_id)).unwrap();
//         crate::verifier::run(
//             &[witness],
//             chain_spec,
//             crate::verifier::StateCommitMode::Block,
//         )
//         .unwrap();
//     }
// }

// pub(super) fn execute(
//     ExecuteInnerArgs {
//         code_db,
//         nodes_provider,
//         block_hashes,
//         pre_state_root,
//         blocks,
//         chain_spec,
//         defer_commit,
//     }: ExecuteInnerArgs,
// ) -> Result<(B256, u64), VerificationError> {
//     let mut gas_used = 0;
//
//     let mut db = manually_drop_on_zkvm!(EvmDatabase::new_from_root(
//         code_db,
//         pre_state_root,
//         nodes_provider,
//         block_hashes
//     )?);
//
//     for block in blocks.iter() {
//         let output =
//             manually_drop_on_zkvm!(EvmExecutor::new(chain_spec.clone(), &db, block).execute()?);
//         gas_used += output.gas_used;
//
//         db.update(
//             nodes_provider,
//             BTreeMap::from_iter(output.state.state.clone()).iter(),
//         )?;
//
//         if !defer_commit {
//             let post_state_root = db.commit_changes();
//             if block.state_root != post_state_root {
//                 dev_error!(
//                     "Block #{} root mismatch: root after in trace = {:x}, root after in reth = {:x}",
//                     block.number,
//                     block.state_root,
//                     post_state_root
//                 );
//                 return Err(VerificationError::block_root_mismatch(
//                     block.state_root,
//                     post_state_root,
//                     output.state,
//                 ));
//             }
//             dev_info!("Block #{} verified successfully", block.number);
//         } else {
//             dev_info!("Block #{} executed successfully", block.number);
//         }
//     }
//
//     let post_state_root = db.commit_changes();
//     let expected_state_root = blocks.last().unwrap().state_root;
//     if expected_state_root != post_state_root {
//         dev_error!(
//             "Final state root mismatch: expected {expected_state_root:x}, found {post_state_root:x}",
//         );
//         return Err(VerificationError::chunk_root_mismatch(
//             expected_state_root,
//             post_state_root,
//         ));
//     }
//     Ok((post_state_root, gas_used))
// }
