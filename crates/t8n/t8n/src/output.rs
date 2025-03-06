use reth_execution_types::BlockExecutionOutput;
use reth_primitives_traits::proofs::calculate_receipt_root;
use sbv_primitives::types::reth::{Block, Receipt, RecoveredBlock};
use t8n_types::{
    AllocAccount, TransactionReceipt, TransitionToolInput, TransitionToolOutput,
    TransitionToolResult,
};

pub(crate) fn make_output(
    input: TransitionToolInput,
    block: RecoveredBlock<Block>,
    output: BlockExecutionOutput<Receipt>,
) -> TransitionToolOutput {
    let mut alloc = input.alloc;
    for (addr, acc) in output.state.state.into_iter() {
        let info = acc.info.unwrap_or_default();
        let alloc_acc = AllocAccount {
            nonce: info.nonce,
            balance: info.balance.to(),
            code: info.code.unwrap_or_default().original_bytes(),
            storage: acc
                .storage
                .into_iter()
                .map(|(k, v)| (k, v.present_value))
                .collect(),
        };
        alloc.insert(addr, alloc_acc);
    }

    let receipts = output
        .receipts
        .iter()
        .map(|receipt| TransactionReceipt {
            gas_used: Some(receipt.cumulative_gas_used),
            cumulative_gas_used: Some(receipt.cumulative_gas_used),
            ..Default::default()
        })
        .collect();

    let result = TransitionToolResult {
        receipts,
        transactions_trie: block.header().transactions_root,
        gas_used: output.gas_used,
        base_fee_per_gas: block.header().base_fee_per_gas,
        withdrawals_root: block.header().withdrawals_root,
        excess_blob_gas: block.header().excess_blob_gas,
        ..Default::default()
    };

    TransitionToolOutput {
        alloc,
        result,
        body: None,
    }
}
