use sbv_primitives::{
    ChainId, U256,
    chainspec::{Chain, ChainHardforks, ChainSpec, DEV, EthereumHardfork, ForkCondition, Hardfork},
};
use std::{str::FromStr, sync::Arc};

// Actual possible `fork_name` called:
// - Frontier
// - Homestead
// - Byzantium
// - Constantinople
// - ConstantinopleFix (? what is this?)
// - Istanbul
// - Berlin
// - London
// - Merge
// - Shanghai
// - Cancun
pub(crate) fn build_chain_spec(chain_id: ChainId, fork_name: &str) -> Arc<ChainSpec> {
    const ETHEREUM_HARDFORKS: [EthereumHardfork; 19] = [
        EthereumHardfork::Frontier,
        EthereumHardfork::Homestead,
        EthereumHardfork::Dao,
        EthereumHardfork::Tangerine,
        EthereumHardfork::SpuriousDragon,
        EthereumHardfork::Byzantium,
        EthereumHardfork::Constantinople,
        EthereumHardfork::Petersburg,
        EthereumHardfork::Istanbul,
        EthereumHardfork::MuirGlacier,
        EthereumHardfork::Berlin,
        EthereumHardfork::London,
        EthereumHardfork::ArrowGlacier,
        EthereumHardfork::GrayGlacier,
        EthereumHardfork::Paris,
        EthereumHardfork::Shanghai,
        EthereumHardfork::Cancun,
        EthereumHardfork::Prague,
        EthereumHardfork::Osaka,
    ];

    let fork_name = match fork_name {
        "ConstantinopleFix" => "Constantinople", // FIXME: is this correct?
        "Merge" => "Paris",
        _ => fork_name,
    };

    let mut spec = (**DEV).clone();
    spec.chain = Chain::from_id(chain_id);
    let mut forks: Vec<(Box<dyn Hardfork>, ForkCondition)> = vec![];
    let fork = EthereumHardfork::from_str(fork_name).expect("Unknown fork name");
    for f in ETHEREUM_HARDFORKS {
        if f <= fork {
            if f < EthereumHardfork::Paris {
                forks.push((Box::new(f), ForkCondition::Block(0)));
            } else if f == EthereumHardfork::Paris {
                forks.push((Box::new(f), ForkCondition::TTD {
                    activation_block_number: 0,
                    fork_block: Some(0),
                    total_difficulty: U256::ZERO,
                }));
            } else {
                forks.push((Box::new(f), ForkCondition::Timestamp(0)));
            }
        } else {
            break;
        }
    }
    spec.hardforks = ChainHardforks::new(forks);
    Arc::new(spec)
}

#[test]
fn test_build_chain_spec() {
    let spec = build_chain_spec(1, "Shanghai");
    println!(
        "{:?}",
        spec.hardforks
            .fork(EthereumHardfork::Cancun)
            .active_at_timestamp(0)
    );
}
