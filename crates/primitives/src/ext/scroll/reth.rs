use crate::{
    B256,
    ext::BlockChunkExt,
    reth::primitives::{Block, RecoveredBlock, TransactionSigned},
};

impl<'a, I: IntoIterator<Item = &'a TransactionSigned>> crate::ext::scroll::TxBytesHashExt for I
where
    I: IntoIterator<Item = &'a TransactionSigned>,
{
    fn tx_bytes_hash(self) -> (usize, B256) {
        let mut rlp_buffer = Vec::new();
        self.tx_bytes_hash_in(&mut rlp_buffer)
    }

    fn tx_bytes_hash_in(self, rlp_buffer: &mut Vec<u8>) -> (usize, B256) {
        use crate::eips::Encodable2718;
        use tiny_keccak::{Hasher, Keccak};

        let mut tx_bytes_hasher = Keccak::v256();
        let mut len = 0;

        // Ignore L1 msg txs.
        for tx in self.into_iter().filter(|&tx| !tx.is_l1_message()) {
            tx.encode_2718(rlp_buffer);
            len += rlp_buffer.len();
            tx_bytes_hasher.update(rlp_buffer);
            rlp_buffer.clear();
        }

        let mut tx_bytes_hash = B256::ZERO;
        tx_bytes_hasher.finalize(&mut tx_bytes_hash.0);
        (len, tx_bytes_hash)
    }
}

impl BlockChunkExt for RecoveredBlock<Block> {
    #[inline]
    fn legacy_hash_da_header(&self, hasher: &mut impl tiny_keccak::Hasher) {
        use crate::U256;

        hasher.update(&self.number.to_be_bytes());
        hasher.update(&self.timestamp.to_be_bytes());
        hasher.update(
            &U256::from_limbs([self.base_fee_per_gas.unwrap_or_default(), 0, 0, 0])
                .to_be_bytes::<{ U256::BYTES }>(),
        );
        hasher.update(&self.gas_limit.to_be_bytes());
        // FIXME: l1 tx could be skipped, the actual tx count needs to be calculated
        hasher.update(&(self.body().transactions.len() as u16).to_be_bytes());
    }

    #[inline]
    fn legacy_hash_l1_msg(&self, hasher: &mut impl tiny_keccak::Hasher) {
        use reth_primitives_traits::SignedTransaction;
        for tx in self
            .body()
            .transactions
            .iter()
            .filter(|tx| tx.is_l1_message())
        {
            hasher.update(tx.tx_hash().as_slice())
        }
    }

    #[inline]
    fn hash_msg_queue(&self, initial_queue_hash: &B256) -> B256 {
        use reth_primitives_traits::SignedTransaction;
        use tiny_keccak::Hasher;

        let mut rolling_hash = *initial_queue_hash;
        for tx in self
            .body()
            .transactions
            .iter()
            .filter(|tx| tx.is_l1_message())
        {
            let mut hasher = tiny_keccak::Keccak::v256();
            hasher.update(rolling_hash.as_slice());
            hasher.update(tx.tx_hash().as_slice());

            hasher.finalize(rolling_hash.as_mut_slice());

            // clear last 32 bits, i.e. 4 bytes.
            // https://github.com/scroll-tech/da-codec/blob/26dc8d575244560611548fada6a3a2745c60fe83/encoding/da.go#L817-L825
            // see also https://github.com/scroll-tech/da-codec/pull/42
            rolling_hash.0[28] = 0;
            rolling_hash.0[29] = 0;
            rolling_hash.0[30] = 0;
            rolling_hash.0[31] = 0;
        }

        rolling_hash
    }

    #[inline]
    fn num_l1_msgs(&self) -> usize {
        self.body()
            .transactions
            .iter()
            .filter(|tx| tx.is_l1_message())
            .count()
    }
}
