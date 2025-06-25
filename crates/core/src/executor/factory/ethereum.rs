use reth_evm_ethereum::EthEvmConfig;
use sbv_primitives::chainspec::ChainSpec;
use sbv_primitives::types::reth::EthPrimitives;

pub type ExecutorProvider = EthEvmConfig<
    ChainSpec,
    EthPrimitives,
    ScrollRethReceiptBuilder,
    crate::executor::factory::scroll::evm::ScrollEvmFactory,
>;
