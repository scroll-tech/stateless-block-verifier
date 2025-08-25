use crate::{EvmDatabase, VerificationError};
use sbv_kv::KeyValueStoreGet;
use sbv_precompile::PrecompileProvider;
use sbv_primitives::{
    B256, Bytes,
    chainspec::ChainSpec,
    types::{
        evm::precompiles::PrecompilesMap,
        reth::{
            evm::{
                ConfigureEvm, Database, EthEvm, EthEvmConfig, EvmEnv, EvmFactory,
                eth::EthEvmContext,
                execute::Executor,
                revm::{
                    Context, Inspector, MainBuilder, MainContext,
                    context::{
                        BlockEnv, CfgEnv, TxEnv,
                        result::{EVMError, HaltReason},
                    },
                    inspector::NoOpInspector,
                },
            },
            execution_types::BlockExecutionOutput,
            primitives::{Block, Receipt, RecoveredBlock},
        },
        revm::{SpecId, database::CacheDB, precompile::PrecompileSpecId},
    },
};
use std::sync::Arc;

/// Ethereum-related EVM configuration with [`SbvEthEvmFactory`] as the factory.
pub type EvmConfig = EthEvmConfig<ChainSpec, SbvEthEvmFactory>;

/// EVM executor that handles the block.
#[derive(Debug)]
pub struct EvmExecutor<'a, CodeDb, NodesProvider, BlockHashProvider> {
    chain_spec: Arc<ChainSpec>,
    db: &'a EvmDatabase<CodeDb, NodesProvider, BlockHashProvider>,
    block: &'a RecoveredBlock<Block>,
}

impl<'a, CodeDb, NodesProvider, BlockHashProvider>
    EvmExecutor<'a, CodeDb, NodesProvider, BlockHashProvider>
{
    /// Create a new EVM executor
    pub fn new(
        chain_spec: Arc<ChainSpec>,
        db: &'a EvmDatabase<CodeDb, NodesProvider, BlockHashProvider>,
        block: &'a RecoveredBlock<Block>,
    ) -> Self {
        Self {
            chain_spec,
            db,
            block,
        }
    }
}

impl<
    CodeDb: KeyValueStoreGet<B256, Bytes>,
    NodesProvider: KeyValueStoreGet<B256, Bytes>,
    BlockHashProvider: KeyValueStoreGet<u64, B256>,
> EvmExecutor<'_, CodeDb, NodesProvider, BlockHashProvider>
{
    /// Handle the block with the given witness
    pub fn execute(self) -> Result<BlockExecutionOutput<Receipt>, VerificationError> {
        let provider = EvmConfig::new_with_evm_factory(self.chain_spec.clone(), SbvEthEvmFactory);

        let output = cycle_track!(
            provider.executor(CacheDB::new(self.db)).execute(self.block),
            "handle_block"
        )?;

        Ok(output)
    }
}

/// Factory producing [`EthEvm`].
#[derive(Debug, Default, Clone, Copy)]
#[non_exhaustive]
pub struct SbvEthEvmFactory;

impl EvmFactory for SbvEthEvmFactory {
    type Evm<DB: Database, I: Inspector<EthEvmContext<DB>>> = EthEvm<DB, I, Self::Precompiles>;
    type Context<DB: Database> = Context<BlockEnv, TxEnv, CfgEnv, DB>;
    type Tx = TxEnv;
    type Error<DBError: core::error::Error + Send + Sync + 'static> = EVMError<DBError>;
    type HaltReason = HaltReason;
    type Spec = SpecId;
    type Precompiles = PrecompilesMap;

    fn create_evm<DB: Database>(&self, db: DB, input: EvmEnv) -> Self::Evm<DB, NoOpInspector> {
        let spec_id = input.cfg_env.spec;
        EthEvm::new(
            Context::mainnet()
                .with_block(input.block_env)
                .with_cfg(input.cfg_env)
                .with_db(db)
                .build_mainnet_with_inspector(NoOpInspector {})
                .with_precompiles(PrecompileProvider::with_spec(
                    PrecompileSpecId::from_spec_id(spec_id),
                )),
            false,
        )
    }

    fn create_evm_with_inspector<DB: Database, I: Inspector<Self::Context<DB>>>(
        &self,
        db: DB,
        input: EvmEnv,
        inspector: I,
    ) -> Self::Evm<DB, I> {
        let spec_id = input.cfg_env.spec;
        EthEvm::new(
            Context::mainnet()
                .with_block(input.block_env)
                .with_cfg(input.cfg_env)
                .with_db(db)
                .build_mainnet_with_inspector(inspector)
                .with_precompiles(PrecompileProvider::with_spec(
                    PrecompileSpecId::from_spec_id(spec_id),
                )),
            true,
        )
    }
}
