use reth_evm::{env::EvmEnv, ConfigureEvmEnv, NextBlockEnvAttributes};
use reth_evm_ethereum::EthEvmConfig;
use reth_primitives::transaction::FillTxEnv;
use revm::primitives::{CfgEnvWithHandlerCfg, Env, TxEnv};
use sbv_chainspec::{revm_spec, ChainSpec, Head};
use sbv_primitives::{types::TypedTransaction, Address, Bytes, Header, U256};
use std::convert::Infallible;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct EvmConfig {
    eth: EthEvmConfig,
}

impl EvmConfig {
    /// Creates a new Ethereum EVM configuration with the given chain spec.
    pub const fn new(chain_spec: Arc<ChainSpec>) -> Self {
        Self {
            eth: EthEvmConfig::new(chain_spec),
        }
    }

    /// Returns the chain spec associated with this configuration.
    pub const fn chain_spec(&self) -> &Arc<ChainSpec> {
        self.eth.chain_spec()
    }
}

impl ConfigureEvmEnv for EvmConfig {
    type Header = Header;
    type Transaction = TypedTransaction;
    type Error = Infallible;

    fn fill_tx_env(&self, tx_env: &mut TxEnv, transaction: &Self::Transaction, sender: Address) {
        transaction.fill_tx_env(tx_env, sender);
    }

    fn fill_tx_env_system_contract_call(
        &self,
        env: &mut Env,
        caller: Address,
        contract: Address,
        data: Bytes,
    ) {
        self.eth
            .fill_tx_env_system_contract_call(env, caller, contract, data);
    }

    fn fill_cfg_env(
        &self,
        cfg_env: &mut CfgEnvWithHandlerCfg,
        header: &Self::Header,
        total_difficulty: U256,
    ) {
        let spec_id = revm_spec(
            self.chain_spec(),
            &Head {
                number: header.number,
                timestamp: header.timestamp,
                difficulty: header.difficulty,
                total_difficulty,
                hash: Default::default(),
            },
        );

        cfg_env.chain_id = self.chain_spec().chain().id();
        cfg_env.handler_cfg.spec_id = spec_id;
    }

    fn next_cfg_and_block_env(
        &self,
        parent: &Self::Header,
        attributes: NextBlockEnvAttributes,
    ) -> Result<EvmEnv, Self::Error> {
        self.eth.next_cfg_and_block_env(parent, attributes)
    }
}
