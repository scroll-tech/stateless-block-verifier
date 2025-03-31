//! reth t8n executor
use reth_evm::execute::{BlockExecutorProvider, Executor};
use reth_evm_ethereum::execute::EthExecutorProvider as ExecutorProvider;
use sbv_primitives::{ChainId, types::revm::db::CacheDB};
use t8n_types::{TransitionToolInput, TransitionToolOutput};

mod block;
mod chain_spec;
mod output;
mod state;

/// Execute a t8n input
pub fn execute_t8n<S: AsRef<str>>(
    fork_name: S,
    chain_id: ChainId,
    _reward: u64,
    input: TransitionToolInput,
) -> TransitionToolOutput {
    let chain_spec = chain_spec::build_chain_spec(chain_id, fork_name.as_ref());
    let provider = ExecutorProvider::ethereum(chain_spec);
    let db = state::AllocDb::new(&input);
    let block = block::build_block(&input);
    let output = provider
        .executor(CacheDB::new(db))
        .execute(&block)
        .expect("execution failed");
    output::make_output(input, block, output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test() {
        let input = serde_json::from_reader(std::fs::File::open("/Users/hhq/workspace/t8n-types/tests/0a1c501c99ac0e76b462e814d995dfc7e705a60ee89f253dc93b7854e46c24a0.json").unwrap()).unwrap();
        let output = execute_t8n("Paris", 1, 0, input);
        println!("{}", serde_json::to_string_pretty(&output).unwrap());
    }
}
