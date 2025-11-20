//! Stateless Block Verifier

#[macro_use]
extern crate sbv;

use clap::Parser;

mod dump;
mod helpers;
mod run;

#[derive(Parser)]
#[command(version, about = "Stateless Block Verifier")]
enum Cli {
    #[command(about = "Run and verify witness")]
    Run(run::RunFileCommand),
    #[command(about = "Dump witness")]
    Dump(dump::DumpWitnessCommand),
}

fn main() -> eyre::Result<()> {
    // Install the tracing subscriber that will listen for events and filters. We try to use the
    // `RUST_LOG` environment variable and default to RUST_LOG=info if unset.
    #[cfg(feature = "dev")]
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    #[cfg(feature = "dev")]
    std::panic::set_hook(Box::new(|info| {
        if std::env::var_os("RUST_BACKTRACE").is_some() {
            dev_error!("panic happens: {info}");
            let bt = std::backtrace::Backtrace::force_capture();
            dev_error!("backtrace:\n{bt}");
        }
    }));

    match Cli::parse() {
        Cli::Run(cmd) => cmd.run(),
        Cli::Dump(cmd) => helpers::run_async(cmd.run()),
    }
}
