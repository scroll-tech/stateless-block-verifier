use crate::utils::ext::TxRevmExt;
use eth_types::l2_types::TransactionTrace;
use eth_types::{Transaction, H256};
use revm::primitives::{AccessListItem, TransactTo, B256, U256};

mod archived_block_trace_v2;
mod blanket;
mod block_trace;
mod block_trace_v2;

impl TxRevmExt for TransactionTrace {
    #[inline]
    fn raw_type(&self) -> u8 {
        self.type_
    }
    #[inline]
    fn tx_hash(&self) -> B256 {
        B256::new(self.tx_hash.0)
    }
    #[inline]
    fn caller(&self) -> revm::precompile::Address {
        self.from.0.into()
    }
    #[inline]
    fn gas_limit(&self) -> u64 {
        self.gas
    }
    #[inline]
    fn gas_price(&self) -> U256 {
        U256::from_limbs(self.gas_price.0)
    }
    #[inline]
    fn transact_to(&self) -> TransactTo {
        match self.to {
            Some(to) => TransactTo::Call(to.0.into()),
            None => TransactTo::Create,
        }
    }
    #[inline]
    fn value(&self) -> U256 {
        U256::from_limbs(self.value.0)
    }
    #[inline]
    fn data(&self) -> revm::precompile::Bytes {
        revm::precompile::Bytes::copy_from_slice(self.data.as_ref())
    }
    #[inline]
    fn nonce(&self) -> u64 {
        self.nonce
    }
    #[inline]
    fn chain_id(&self) -> u64 {
        self.chain_id.as_u64()
    }
    #[inline]
    fn access_list(&self) -> Vec<AccessListItem> {
        self.access_list
            .as_ref()
            .map(|v| {
                v.iter()
                    .map(|e| AccessListItem {
                        address: e.address.0.into(),
                        storage_keys: e
                            .storage_keys
                            .iter()
                            .map(|s| s.to_fixed_bytes().into())
                            .collect(),
                    })
                    .collect()
            })
            .unwrap_or_default()
    }
    #[inline]
    fn gas_priority_fee(&self) -> Option<U256> {
        self.gas_tip_cap.map(|g| U256::from_limbs(g.0))
    }
    #[inline]
    fn to_eth_tx(
        &self,
        block_hash: B256,
        block_number: u64,
        transaction_index: usize,
        base_fee_per_gas: Option<U256>,
    ) -> Transaction {
        self.to_eth_tx(
            Some(H256::from(block_hash.0)),
            Some(block_number.into()),
            Some((transaction_index as u64).into()),
            base_fee_per_gas.map(|b| eth_types::U256(*b.as_limbs())),
        )
    }
}
