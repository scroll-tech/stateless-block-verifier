use crate::BlockWitness;
use crate::{B256, Bytes, ext::BlockWitnessExt, keccak256};
use sbv_helpers::cycle_track;
use sbv_kv::KeyValueStore;

impl BlockWitnessExt for BlockWitness {
    fn import_codes<CodeDb: KeyValueStore<B256, Bytes>>(&self, mut code_db: CodeDb) {
        for code in self.codes.iter() {
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
        let block_number = self.header.number;
        for (i, hash) in self.block_hashes.iter().copied().enumerate() {
            let block_number = block_number
                .checked_sub(i as u64 + 1)
                .expect("block number underflow");
            block_hashes.insert(block_number, hash)
        }
    }
}

impl BlockWitnessExt for [BlockWitness] {
    fn import_codes<CodeDb: KeyValueStore<B256, Bytes>>(&self, mut code_db: CodeDb) {
        for code in self.iter().flat_map(|w| w.codes.iter()) {
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
            let block_number = witness.header.number;
            for (i, hash) in witness.block_hashes.iter().copied().enumerate() {
                let block_number = block_number
                    .checked_sub(i as u64 + 1)
                    .expect("block number underflow");
                block_hashes.insert(block_number, hash)
            }
        }
    }
}
