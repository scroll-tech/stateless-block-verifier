use reth_primitives_traits::proofs::{calculate_transaction_root, calculate_withdrawals_root};
use sbv_primitives::{
    B256, PrimitiveSignature, U256,
    alloy_primitives::normalize_v,
    types::{
        consensus::{
            Header, SignableTransaction, TxEip1559, TxEip2930, TxEip4844, TxEip7702, TxLegacy,
        },
        eips::eip4895::Withdrawals,
        reth::{Block, BlockBody, RecoveredBlock, TransactionSigned},
    },
};
use t8n_types::TransitionToolInput;

pub(crate) fn build_block(input: &TransitionToolInput) -> RecoveredBlock<Block> {
    let senders = input.txs.iter().map(|tx| tx.sender).collect();
    let transactions = input.txs.iter().map(to_reth_tx).collect::<Vec<_>>();
    let withdrawals = input.env.withdrawals.clone().map(Withdrawals::new);
    let header = Header {
        parent_hash: input.env.parent_hash.unwrap_or_default(),
        ommers_hash: input.env.parent_ommers_hash,
        beneficiary: input.env.fee_recipient,
        transactions_root: calculate_transaction_root(&transactions),
        difficulty: U256::from(input.env.difficulty.unwrap_or_default()),
        number: input.env.number,
        gas_limit: input.env.gas_limit,
        timestamp: input.env.timestamp,
        mix_hash: B256::from(input.env.prev_randao.unwrap_or_default().to_be_bytes()),
        base_fee_per_gas: input
            .env
            .base_fee_per_gas
            .or(input.env.parent_base_fee_per_gas), //?
        blob_gas_used: input.env.blob_gas_used.or(input.env.parent_blob_gas_used),
        excess_blob_gas: input
            .env
            .excess_blob_gas
            .or(input.env.parent_excess_blob_gas), //?
        parent_beacon_block_root: input.env.parent_beacon_block_root,
        withdrawals_root: withdrawals
            .as_ref()
            .map(|w| calculate_withdrawals_root(&w.0)),
        ..Default::default() // FIXME: can we omit other fields?
    };
    let body = BlockBody {
        transactions,
        withdrawals,
        ..Default::default()
    };
    let block = Block::new(header, body);
    RecoveredBlock::new_unhashed(block, senders)
}

fn to_reth_tx(tx: &t8n_types::Transaction) -> TransactionSigned {
    let tx_type = tx.ty;

    let sig = PrimitiveSignature::new(tx.r, tx.s, normalize_v(tx.v).expect("invalid v"));

    match tx_type {
        0x00 => {
            let tx = TxLegacy {
                chain_id: Some(tx.chain_id),
                nonce: tx.nonce,
                gas_price: tx.gas_price.unwrap(),
                gas_limit: tx.gas_limit,
                to: tx.to.into(),
                value: tx.value,
                input: tx.data.clone(),
            };

            tx.into_signed(sig).into()
        }
        0x01 => {
            let tx = TxEip2930 {
                chain_id: tx.chain_id,
                nonce: tx.nonce,
                gas_price: tx.gas_price.unwrap(),
                gas_limit: tx.gas_limit,
                to: tx.to.into(),
                value: tx.value,
                access_list: tx.access_list.clone().expect("missing access_list").into(),
                input: tx.data.clone(),
            };

            tx.into_signed(sig).into()
        }
        0x02 => {
            let tx = TxEip1559 {
                chain_id: tx.chain_id,
                nonce: tx.nonce,
                max_fee_per_gas: tx.max_fee_per_gas.expect("missing max_fee_per_gas"),
                max_priority_fee_per_gas: tx
                    .max_priority_fee_per_gas
                    .expect("missing max_priority_fee_per_gas"),
                gas_limit: tx.gas_limit,
                to: tx.to.into(),
                value: tx.value,
                access_list: tx.access_list.clone().expect("missing access_list").into(),
                input: tx.data.clone(),
            };

            tx.into_signed(sig).into()
        }
        0x03 => {
            let tx = TxEip4844 {
                chain_id: tx.chain_id,
                nonce: tx.nonce,
                max_fee_per_gas: tx.max_fee_per_gas.expect("missing max_fee_per_gas"),
                max_priority_fee_per_gas: tx
                    .max_priority_fee_per_gas
                    .expect("missing max_priority_fee_per_gas"),
                gas_limit: tx.gas_limit,
                to: tx.to.expect("missing to"),
                value: tx.value.into(),
                input: tx.data.clone(),
                access_list: tx.access_list.clone().expect("missing access_list").into(),
                blob_versioned_hashes: tx
                    .blob_versioned_hashes
                    .clone()
                    .expect("missing blob_versioned_hashes"),
                max_fee_per_blob_gas: tx
                    .max_fee_per_blob_gas
                    .expect("missing max_fee_per_blob_gas"),
            };
            tx.into_signed(sig).into()
        }
        0x04 => {
            let tx = TxEip7702 {
                chain_id: tx.chain_id,
                nonce: tx.nonce,
                gas_limit: tx.gas_limit,
                max_fee_per_gas: tx.max_fee_per_gas.expect("missing max_fee_per_gas"),
                max_priority_fee_per_gas: tx
                    .max_priority_fee_per_gas
                    .expect("missing max_priority_fee_per_gas"),
                to: tx.to.expect("missing to"),
                value: tx.value,
                access_list: tx.access_list.clone().expect("missing access_list").into(),
                authorization_list: tx
                    .authorization_list
                    .clone()
                    .expect("missing authorization_list"),
                input: tx.data.clone(),
            };
            tx.into_signed(sig).into()
        }
        _ => unimplemented!("unsupported tx type: {}", tx_type),
    }
}
