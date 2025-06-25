use alloy_evm::{Database, EvmEnv, EvmFactory, precompiles::PrecompilesMap};
use core::fmt::Debug;
use reth_scroll_evm::{ScrollEvmConfig, ScrollRethReceiptBuilder};
use revm::{
    Context, Inspector,
    context::{TxEnv, result::HaltReason},
    context_interface::result::EVMError,
    inspector::NoOpInspector,
};
use revm_scroll::builder::EuclidEipActivations;
use revm_scroll::{
    ScrollSpecId,
    builder::{DefaultScrollContext, MaybeWithEip7702, ScrollBuilder, ScrollContext},
};
use sbv_precompile::PrecompileProvider;
use sbv_primitives::chainspec::ChainSpec;
use sbv_primitives::types::reth::EthPrimitives;
use scroll_alloy_evm::{ScrollEvm, ScrollTransactionIntoTxEnv};

pub type ExecutorProvider =
    ScrollEvmConfig<ChainSpec, EthPrimitives, ScrollRethReceiptBuilder, ScrollEvmFactory>;

/// Factory producing [`ScrollEvm`]s.
#[derive(Debug, Default, Clone, Copy)]
#[non_exhaustive]
pub struct ScrollEvmFactory;

impl EvmFactory for ScrollEvmFactory {
    type Evm<DB: Database, I: Inspector<ScrollContext<DB>>> = ScrollEvm<DB, I, Self::Precompiles>;
    type Context<DB: Database> = ScrollContext<DB>;
    type Tx = ScrollTransactionIntoTxEnv<TxEnv>;
    type Error<DBError: core::error::Error + Send + Sync + 'static> = EVMError<DBError>;
    type HaltReason = HaltReason;
    type Spec = ScrollSpecId;
    type Precompiles = PrecompilesMap;

    fn create_evm<DB: Database>(
        &self,
        db: DB,
        input: EvmEnv<ScrollSpecId>,
    ) -> Self::Evm<DB, NoOpInspector> {
        let spec_id = input.cfg_env.spec;
        ScrollEvm {
            inner: Context::scroll()
                .with_db(db)
                .with_block(input.block_env)
                .with_cfg(input.cfg_env)
                .maybe_with_eip_7702()
                .build_scroll_with_inspector(NoOpInspector {})
                .with_precompiles(PrecompileProvider::new_with_spec(spec_id)),
            inspect: false,
        }
    }

    fn create_evm_with_inspector<DB: Database, I: Inspector<Self::Context<DB>>>(
        &self,
        db: DB,
        input: EvmEnv<ScrollSpecId>,
        inspector: I,
    ) -> Self::Evm<DB, I> {
        let spec_id = input.cfg_env.spec;
        ScrollEvm {
            inner: Context::scroll()
                .with_db(db)
                .with_block(input.block_env)
                .with_cfg(input.cfg_env)
                .maybe_with_eip_7702()
                .build_scroll_with_inspector(inspector)
                .with_precompiles(PrecompileProvider::new_with_spec(spec_id)),
            inspect: true,
        }
    }
}
