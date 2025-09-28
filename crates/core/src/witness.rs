use auto_impl::auto_impl;
use itertools::Itertools;
use reth_primitives_traits::crypto::{InvalidSignatureS, RecoveryError, SECP256K1N_HALF};
use reth_primitives_traits::serde_bincode_compat::BincodeReprFor;
use sbv_kv::KeyValueStore;
use sbv_primitives::types::revm;
use sbv_primitives::{
    Address, B256, Bytes, ChainId, SignatureError, U256, keccak256,
    types::{
        Header,
        consensus::{SignerRecoverable, TxEnvelope},
        eips::eip4895::Withdrawals,
        reth::primitives::{Block, BlockBody, RecoveredBlock, SealedBlock},
    },
};

/// Witness for a block.
#[serde_with::serde_as]
#[derive(Debug, Clone, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct BlockWitness {
    /// Chain id
    pub chain_id: ChainId,
    /// Block header representation.
    #[serde_as(as = "BincodeReprFor<'_, Header>")]
    pub header: Header,
    /// State trie root before the block.
    pub prev_state_root: B256,
    /// Transactions in the block.
    #[serde_as(as = "Vec<BincodeReprFor<'_, TxEnvelope>>")]
    pub transactions: Vec<TxEnvelope>,
    /// Withdrawals in the block.
    pub withdrawals: Option<Withdrawals>,
    /// Last 256 Ancestor block hashes.
    #[cfg(not(feature = "scroll"))]
    pub block_hashes: Vec<B256>,
    /// Rlp encoded state trie nodes.
    #[serde(default)]
    pub states: Vec<Bytes>,
    /// Code bytecodes
    pub codes: Vec<Bytes>,
}

impl BlockWitness {
    /// Calculates compression ratios for all transactions in the block witness.
    ///
    /// # Panics
    ///
    /// Panics if called without the "scroll-compress-ratio" feature enabled, as this
    /// functionality is not intended to be used in guest environments.
    pub fn compression_ratios(&self) -> Vec<U256> {
        #[cfg(feature = "scroll-compress-ratio")]
        {
            use sbv_primitives::types::consensus::Transaction;

            self.transactions
                .iter()
                .map(|tx| sbv_primitives::types::evm::compute_compression_ratio(&tx.input()))
                .collect()
        }
        #[cfg(not(feature = "scroll-compress-ratio"))]
        {
            unimplemented!("you should not build ChunkWitness in guest?");
        }
    }

    /// Converts the `BlockWitness` into a legacy `BlockWitness`.
    pub fn into_legacy(self) -> sbv_primitives::legacy_types::BlockWitness {
        sbv_primitives::legacy_types::BlockWitness {
            chain_id: self.chain_id,
            header: self.header.into(),
            pre_state_root: self.prev_state_root,
            transaction: self.transactions.into_iter().map(Into::into).collect(),
            withdrawals: self
                .withdrawals
                .map(|w| w.into_iter().map(Into::into).collect()),
            #[cfg(not(feature = "scroll"))]
            block_hashes: self.block_hashes,
            states: self.states,
            codes: self.codes,
        }
    }

    /// Build execution context from the witness.
    pub fn build_reth_block(&self) -> Result<RecoveredBlock<Block>, SignatureError> {
        let crypto = revm::precompile::crypto();

        let senders = self
            .transactions
            .iter()
            .map(|tx| {
                let (signature, signature_hash) = match tx {
                    TxEnvelope::Legacy(tx) => (tx.signature(), tx.signature_hash()),
                    TxEnvelope::Eip2930(tx) => (tx.signature(), tx.signature_hash()),
                    TxEnvelope::Eip1559(tx) => (tx.signature(), tx.signature_hash()),
                    TxEnvelope::Eip7702(tx) => (tx.signature(), tx.signature_hash()),
                    #[cfg(feature = "scroll")]
                    TxEnvelope::L1Message(tx) => return Ok(tx.sender),
                };

                if signature.s() > SECP256K1N_HALF {
                    return Err(RecoveryError::from_source(InvalidSignatureS));
                }

                let mut sig = [0u8; 64];
                sig[0..32].copy_from_slice(&signature.r().to_be_bytes::<32>());
                sig[32..64].copy_from_slice(&signature.s().to_be_bytes::<32>());

                let result = crypto
                    .secp256k1_ecrecover(&sig, signature.v() as u8, &signature_hash.0)
                    .map_err(RecoveryError::from_source)?;
                let signer = Address::from_slice(&result[12..]);

                debug_assert_eq!(signer, tx.recover_signer()?, "should match");

                Ok(signer.into())
            })
            .collect::<Result<Vec<_>, _>>()
            .expect("Failed to recover signer");

        let body = BlockBody {
            transactions: self.transactions.clone(),
            ommers: vec![],
            withdrawals: self.withdrawals.clone(),
        };
        let block = RecoveredBlock::new_sealed(
            SealedBlock::seal_slow(Block {
                header: self.header.clone(),
                body,
            }),
            senders,
        );

        Ok(block)
    }
}

impl From<sbv_primitives::legacy_types::BlockWitness> for BlockWitness {
    fn from(legacy: sbv_primitives::legacy_types::BlockWitness) -> Self {
        Self {
            chain_id: legacy.chain_id,
            header: legacy.header.into(),
            prev_state_root: legacy.pre_state_root,
            transactions: legacy
                .transaction
                .into_iter()
                .map(|t| t.try_into().unwrap())
                .collect(),
            withdrawals: legacy
                .withdrawals
                .map(|w| Withdrawals::new(w.into_iter().map(Into::into).collect())),
            #[cfg(not(feature = "scroll"))]
            block_hashes: legacy.block_hashes,
            states: legacy.states,
            codes: legacy.codes,
        }
    }
}

/// BlockWitnessExt trait
#[auto_impl(&, &mut, Box, Rc, Arc)]
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
        for (i, hash) in self.block_hashes.iter().enumerate() {
            let block_number = block_number
                .checked_sub(i as u64 + 1)
                .expect("block number underflow");
            block_hashes.insert(block_number, *hash)
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
            for (i, hash) in witness.block_hashes.iter().enumerate() {
                let block_number = block_number
                    .checked_sub(i as u64 + 1)
                    .expect("block number underflow");
                block_hashes.insert(block_number, *hash)
            }
        }
    }
}

/// BlockWitnessCodeExt trait
#[auto_impl(&, &mut, Box, Rc, Arc)]
pub trait BlockWitnessChunkExt {
    /// Get the chain id.
    fn chain_id(&self) -> ChainId;
    /// Get the previous state root.
    fn prev_state_root(&self) -> B256;
    /// Check if all witnesses have the same chain id.
    fn has_same_chain_id(&self) -> bool;
    /// Check if all witnesses have a sequence block number.
    fn has_seq_block_number(&self) -> bool;
    /// Check if all witnesses have a sequence state root.
    fn has_seq_state_root(&self) -> bool;
}

impl BlockWitnessChunkExt for [BlockWitness] {
    #[inline(always)]
    fn chain_id(&self) -> ChainId {
        debug_assert!(self.has_same_chain_id(), "chain id mismatch");
        self.first().expect("empty witnesses").chain_id
    }

    #[inline(always)]
    fn prev_state_root(&self) -> B256 {
        self.first().expect("empty witnesses").prev_state_root
    }

    #[inline(always)]
    fn has_same_chain_id(&self) -> bool {
        self.iter()
            .tuple_windows()
            .all(|(a, b)| a.chain_id == b.chain_id)
    }

    #[inline(always)]
    fn has_seq_block_number(&self) -> bool {
        self.iter()
            .tuple_windows()
            .all(|(a, b)| a.header.number + 1 == b.header.number)
    }

    #[inline(always)]
    fn has_seq_state_root(&self) -> bool {
        self.iter()
            .tuple_windows()
            .all(|(a, b)| a.header.state_root == b.prev_state_root)
    }
}
