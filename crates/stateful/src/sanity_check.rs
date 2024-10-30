use crate::{retry_if_transport_error, Result};
use alloy::primitives::ChainId;
use alloy::providers::{Provider, ReqwestProvider};
use revm::primitives::BlockEnv;
use sbv::{
    core::{EvmExecutorBuilder, HardforkConfig},
    primitives::{
        types::{AlloyTransaction, BlockTrace, LegacyStorageTrace},
        zk_trie::db::{kv::HashMapDb, NodeDb},
        Block, Transaction, TxTrace, B256, U256,
    },
};

/// Assert that the given L2 trace and block are equal.
pub fn assert_equal(l2_trace: impl Block, block: impl Block) {
    let trace_block_env = BlockEnv {
        number: U256::from_limbs([l2_trace.number(), 0, 0, 0]),
        coinbase: l2_trace.coinbase(),
        timestamp: l2_trace.timestamp(),
        gas_limit: l2_trace.gas_limit(),
        basefee: l2_trace.base_fee_per_gas().unwrap_or_default(),
        difficulty: l2_trace.difficulty(),
        prevrandao: l2_trace.prevrandao(),
        blob_excess_gas_and_price: None,
    };
    let block_block_env = BlockEnv {
        number: U256::from_limbs([block.number(), 0, 0, 0]),
        coinbase: block.coinbase(),
        timestamp: block.timestamp(),
        gas_limit: block.gas_limit(),
        basefee: block.base_fee_per_gas().unwrap_or_default(),
        difficulty: block.difficulty(),
        prevrandao: block.prevrandao(),
        blob_excess_gas_and_price: None,
    };
    assert_eq!(trace_block_env, block_block_env, "block_env mismatch");
    for (i, (trace_tx, block_tx)) in l2_trace
        .transactions()
        .zip(block.transactions())
        .enumerate()
    {
        let trace_tx = trace_tx.try_build_typed_tx().unwrap();
        let block_tx = block_tx.try_build_typed_tx().unwrap();
        assert_eq!(trace_tx, block_tx, "tx#{i} mismatch {block_tx:?}");
        let trace_tx_signer = trace_tx.get_or_recover_signer().unwrap();
        let block_tx_signer = block_tx.get_or_recover_signer().unwrap();
        assert_eq!(trace_tx_signer, block_tx_signer, "tx#{i} signer mismatch");
        let trace_gas_limit = trace_tx.gas_limit();
        let block_gas_limit = block_tx.gas_limit();
        assert_eq!(
            trace_gas_limit, block_gas_limit,
            "tx#{i} gas limit mismatch"
        );
        let trace_gas_price = trace_tx
            .effective_gas_price(l2_trace.base_fee_per_gas().unwrap_or_default().to())
            .map(U256::from);
        let block_gas_price = block_tx
            .effective_gas_price(l2_trace.base_fee_per_gas().unwrap_or_default().to())
            .map(U256::from);
        assert_eq!(
            trace_gas_price, block_gas_price,
            "tx#{i} gas price mismatch"
        );
        assert_eq!(trace_tx.to(), block_tx.to(), "tx#{i} transact_to mismatch");
        assert_eq!(trace_tx.value(), block_tx.value(), "tx#{i} value mismatch");
        assert_eq!(trace_tx.data(), block_tx.data(), "tx#{i} data mismatch");
        assert_eq!(
            trace_tx.is_l1_msg(),
            block_tx.is_l1_msg(),
            "tx#{i} is_l1_msg mismatch"
        );
        assert_eq!(trace_tx.nonce(), block_tx.nonce(), "tx#{i} nonce mismatch");
        assert_eq!(
            trace_tx.chain_id(),
            block_tx.chain_id(),
            "tx#{i} chain_id mismatch"
        );
        assert_eq!(
            trace_tx.access_list(),
            block_tx.access_list(),
            "tx#{i} access_list mismatch"
        );
        assert_eq!(
            trace_tx.max_priority_fee_per_gas(),
            block_tx.max_priority_fee_per_gas(),
            "tx#{i} max_priority_fee_per_gas mismatch"
        );
        assert_eq!(trace_tx.rlp(), block_tx.rlp(), "tx#{i} rlp mismatch");
    }
}

/// Check the stateful execution of the given block.
pub async fn check_stateless(
    provider: &ReqwestProvider,
    chain_id: ChainId,
    hardfork_config: HardforkConfig,
    storage_root_before: B256,
    block: &alloy::rpc::types::Block<AlloyTransaction>,
) -> Result<B256> {
    let block_number = block.header.number;
    let l2_trace = retry_if_transport_error!(provider
        .raw_request::<_, BlockTrace<LegacyStorageTrace>>(
            "scroll_getBlockTraceByNumberOrHash".into(),
            (format!("0x{:x}", block_number),),
        ))?;
    let l2_trace: BlockTrace = l2_trace.into();
    let root_before = l2_trace.root_before();
    let root_after = l2_trace.root_after();

    assert_eq!(root_before, storage_root_before);
    dev_info!(
        "block#{block_number} trace fetched, root_before: {root_before}, root_after: {root_after}"
    );

    {
        let mut code_db = HashMapDb::default();
        let mut zktrie_db = NodeDb::new(HashMapDb::default());
        l2_trace.build_zktrie_db(&mut zktrie_db).unwrap();
        let mut executor = EvmExecutorBuilder::new(&mut code_db, &mut zktrie_db)
            .hardfork_config(hardfork_config)
            .chain_id(chain_id)
            .build(root_before)?;
        executor.insert_codes(&l2_trace)?;
        executor.handle_block(&l2_trace)?;
        let revm_root_after = executor.commit_changes()?;
        assert_eq!(root_after, revm_root_after);
        dev_info!("block#{block_number} stateless check ok");
    }
    assert_equal(&l2_trace, block);

    Ok(root_after)
}
