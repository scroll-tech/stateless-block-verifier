use crate::{keccak256, types::ExecutionWitness, BlockHeader, BlockWitness, Bytes, B256};
use alloy_eips::BlockNumberOrTag;
use alloy_provider::network::Ethereum;
use alloy_provider::{Network, Provider};
use alloy_transport::{BoxTransport, Transport, TransportResult};
use sbv_helpers::cycle_track;
use sbv_kv::KeyValueStore;

/// BlockWitnessCodeExt trait
#[cfg(feature = "scroll")]
pub trait BlockWitnessChunkExt {
    /// Check if all witnesses have the same chain id.
    fn has_same_chain_id(&self) -> bool;
    /// Check if all witnesses have a sequence block number.
    fn has_seq_block_number(&self) -> bool;
}

#[cfg(feature = "scroll")]
impl<T: BlockWitness> BlockWitnessChunkExt for [T] {
    #[inline(always)]
    fn has_same_chain_id(&self) -> bool {
        self.windows(2).all(|w| w[0].chain_id() == w[1].chain_id())
    }
    #[inline(always)]
    fn has_seq_block_number(&self) -> bool {
        self.windows(2)
            .all(|w| w[0].header().number() + 1 == w[1].header().number())
    }
}

/// BlockWitnessExt trait
pub trait BlockWitnessExt {
    /// Import codes into code db
    fn import_codes<CodeDb: KeyValueStore<B256, Bytes>>(&self, code_db: CodeDb);
    /// Import block hashes into block hash provider
    #[cfg(not(feature = "scroll"))]
    fn import_block_hashes<BlockHashProvider: KeyValueStore<u64, B256>>(
        &self,
        block_hashes: BlockHashProvider,
    );
}

impl<T: BlockWitness> BlockWitnessExt for T {
    fn import_codes<CodeDb: KeyValueStore<B256, Bytes>>(&self, mut code_db: CodeDb) {
        for code in self.codes_iter() {
            let code = code.as_ref();
            let code_hash = cycle_track!(keccak256(code), "keccak256");
            code_db.or_insert_with(code_hash, || Bytes::copy_from_slice(code))
        }
    }

    #[cfg(not(feature = "scroll"))]
    fn import_block_hashes<BlockHashProvider: KeyValueStore<u64, B256>>(
        &self,
        mut block_hashes: BlockHashProvider,
    ) {
        let block_number = self.header().number();
        for (i, hash) in self.block_hashes_iter().enumerate() {
            let block_number = block_number
                .checked_sub(i as u64 + 1)
                .expect("block number underflow");
            block_hashes.insert(block_number, hash)
        }
    }
}

impl<T: BlockWitness> BlockWitnessExt for [T] {
    fn import_codes<CodeDb: KeyValueStore<B256, Bytes>>(&self, mut code_db: CodeDb) {
        for code in self.iter().flat_map(|w| w.codes_iter()) {
            let code = code.as_ref();
            let code_hash = cycle_track!(keccak256(code), "keccak256");
            code_db.or_insert_with(code_hash, || Bytes::copy_from_slice(code))
        }
    }

    #[cfg(not(feature = "scroll"))]
    fn import_block_hashes<BlockHashProvider: KeyValueStore<u64, B256>>(
        &self,
        mut block_hashes: BlockHashProvider,
    ) {
        for witness in self.iter() {
            let block_number = witness.header().number();
            for (i, hash) in witness.block_hashes_iter().enumerate() {
                let block_number = block_number
                    .checked_sub(i as u64 + 1)
                    .expect("block number underflow");
                block_hashes.insert(block_number, hash)
            }
        }
    }
}

/// Helper trait for hashing transaction bytes.
#[cfg(feature = "scroll")]
pub trait TxBytesHashExt {
    /// Hash the transaction bytes.
    fn tx_bytes_hash(self) -> B256;

    /// Hash the transaction bytes.
    fn tx_bytes_hash_in(self, rlp_buffer: &mut Vec<u8>) -> B256;
}

#[cfg(feature = "scroll")]
impl<'a, I: IntoIterator<Item = &'a Tx>, Tx: alloy_eips::eip2718::Encodable2718 + 'a> TxBytesHashExt
    for I
{
    fn tx_bytes_hash(self) -> B256 {
        let mut rlp_buffer = Vec::new();
        self.tx_bytes_hash_in(&mut rlp_buffer)
    }

    fn tx_bytes_hash_in(self, rlp_buffer: &mut Vec<u8>) -> B256 {
        use tiny_keccak::{Hasher, Keccak};
        let mut tx_bytes_hasher = Keccak::v256();
        for tx in self.into_iter() {
            tx.encode_2718(rlp_buffer);
            tx_bytes_hasher.update(&rlp_buffer);
            rlp_buffer.clear();
        }
        let mut tx_bytes_hash = B256::ZERO;
        tx_bytes_hasher.finalize(&mut tx_bytes_hash.0);
        tx_bytes_hash
    }
}

/// Extension trait for [`Provider`](Provider).
#[async_trait::async_trait]
pub trait ProviderExt<T: Transport + Clone = BoxTransport, N: Network = Ethereum>:
    Provider<T, N>
{
    /// Get the execution witness for a block.
    async fn debug_execution_witness(
        &self,
        number: BlockNumberOrTag,
    ) -> TransportResult<ExecutionWitness> {
        self.client()
            .request::<_, ExecutionWitness>("debug_executionWitness", (number,))
            .await
    }

    /// Get the disk root for a block.
    #[cfg(feature = "scroll")]
    async fn scroll_disk_root(
        &self,
        number: BlockNumberOrTag,
    ) -> TransportResult<crate::types::DiskRoot> {
        self.client()
            .request::<_, crate::types::DiskRoot>("scroll_diskRoot", (number,))
            .await
    }
}

impl<P: Provider<T, N>, T: Transport + Clone, N: Network> ProviderExt<T, N> for P {}
