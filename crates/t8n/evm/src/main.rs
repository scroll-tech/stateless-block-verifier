//! This is the main entry point for the t8n executor.
use clap::Parser;
use sbv_t8n::execute_t8n;
use std::io::stdin;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Args {
    #[clap(long = "input.alloc")]
    _input_alloc: String,
    #[clap(long = "input.txs")]
    _input_txs: String,
    #[clap(long = "input.env")]
    _input_env: String,
    #[clap(long = "output.result")]
    _output_result: String,
    #[clap(long = "output.alloc")]
    _output_alloc: String,
    #[clap(long = "output.body")]
    _output_body: String,
    #[clap(long = "state.fork")]
    state_fork: String,
    #[clap(long = "state.chainid")]
    state_chainid: u64,
    #[clap(long = "state.reward")]
    state_reward: u64,
}

fn main() {
    let args = Args::parse();
    let mut input = String::new();
    stdin().read_line(&mut input).expect("Failed to read input");
    let input = serde_json::from_str(&input).expect("Failed to parse input");
    let output = execute_t8n(
        args.state_fork,
        args.state_chainid,
        args.state_reward,
        input,
    );
    println!("{}", serde_json::to_string_pretty(&output).unwrap());
}
