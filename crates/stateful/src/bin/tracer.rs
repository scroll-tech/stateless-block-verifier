//! trace dumper
use clap::Parser;
use sbv::core::{EvmExecutorBuilder, HardforkConfig};
use sbv::primitives::types::{BlockTrace, BytecodeTrace, StorageTrace};
use stateful_block_verifier::Metadata;
use std::path::PathBuf;
use zktrie_ng::db::kv::middleware::RecorderMiddleware;
use zktrie_ng::db::NodeDb;
use zktrie_ng::hash::keccak::Keccak;
use zktrie_ng::hash::poseidon::Poseidon;
use zktrie_ng::hash::HashSchemeKind;

#[derive(Parser)]
struct Cli {
    block_number: u64,
    output: Option<PathBuf>,

    /// Path to the sled database
    #[arg(short, long)]
    db: PathBuf,
    /// Chain ID
    #[arg(short, long)]
    chain_id: u64,
    /// Hash scheme
    #[arg(long, value_enum, default_value_t = HashSchemeKind::Poseidon)]
    hash_scheme: HashSchemeKind,
}

fn main() -> anyhow::Result<()> {
    let Cli {
        block_number,
        output,
        db,
        chain_id,
        hash_scheme,
    } = Cli::parse();

    let db = sled::open(db)?;
    let metadata = Metadata::open(&db, chain_id)?;

    if metadata.latest_block_number() < block_number {
        eprintln!("Block {} has not been imported yet", block_number);
        std::process::exit(1);
    }

    let block_db = metadata.open_block_db(&db)?;
    let block = block_db.get_block(block_number)?.unwrap();

    let hardfork_config = HardforkConfig::default_from_chain_id(chain_id);

    let history_db = match hash_scheme {
        HashSchemeKind::Poseidon => metadata.open_history_db(&db, HashSchemeKind::Poseidon)?,
        HashSchemeKind::Keccak => metadata.open_history_db(&db, HashSchemeKind::Keccak)?,
    };

    let mut code_db = metadata.open_code_db(&db)?;
    let zktrie_db = match hash_scheme {
        HashSchemeKind::Poseidon => metadata
            .open_zktrie_db(&db, HashSchemeKind::Poseidon)?
            .into_inner(),
        HashSchemeKind::Keccak => metadata
            .open_zktrie_db(&db, HashSchemeKind::Keccak)?
            .into_inner(),
    };
    let mut zktrie_db = NodeDb::new(RecorderMiddleware::new(zktrie_db));

    let storage_root_before = history_db
        .get_block_storage_root(block_number - 1)?
        .expect("prev block storage root not found");

    let builder = EvmExecutorBuilder::new(&mut code_db, &mut zktrie_db)
        .chain_id(chain_id)
        .hardfork_config(hardfork_config);

    let (codes, post_root) = match hash_scheme {
        HashSchemeKind::Poseidon => {
            let mut evm = builder.hash_scheme(Poseidon).build(storage_root_before)?;
            evm.handle_block(&block)?;
            (evm.db().contracts.clone(), evm.commit_changes()?)
        }
        HashSchemeKind::Keccak => {
            let mut evm = builder.hash_scheme(Keccak).build(storage_root_before)?;
            evm.handle_block(&block)?;
            (evm.db().contracts.clone(), evm.commit_changes()?)
        }
    };

    let storage_root_after = history_db.get_block_storage_root(block_number)?.unwrap();
    assert_eq!(storage_root_after, post_root);

    let trace = BlockTrace::new_from_alloy(
        chain_id,
        codes
            .into_values()
            .map(|code| BytecodeTrace {
                code: code.original_bytes(),
            })
            .collect(),
        StorageTrace {
            root_before: storage_root_before,
            root_after: storage_root_after,
            flatten_proofs: zktrie_db
                .into_inner()
                .take_read_items()
                .into_iter()
                .map(|(_, v)| v.into())
                .collect(),
        },
        &block,
    );

    let output = output.unwrap_or_else(|| PathBuf::from(format!("block-{}.json", block_number)));
    let output = std::fs::File::create(output)?;
    serde_json::to_writer_pretty(output, &trace)?;

    Ok(())
}
