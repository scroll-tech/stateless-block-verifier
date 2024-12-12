use alloy_chains::{Chain, NamedChain};
use reth_chainspec::{once_cell_set, BaseFeeParams, BaseFeeParamsKind, ChainSpec};
use reth_ethereum_forks::{hardfork, ChainHardforks, EthereumHardfork, ForkCondition, Hardfork};
use revm::primitives::{Account, AccountStatus, Bytecode, Bytes, EvmStorage, EvmStorageSlot};
use revm::DatabaseRef;
// use sbv_primitives::predeployed::l1_gas_price_oracle;
use sbv_primitives::{b256, Address, B256, U256};
use std::convert::Infallible;
use std::sync::{Arc, LazyLock};
use std::{
    fmt,
    fmt::{Display, Formatter},
    str::FromStr,
};

hardfork!(
    /// The name of an Ethereum Scroll hardfork.
    ScrollHardfork {
        /// Frontier: <https://blog.ethereum.org/2015/03/03/ethereum-launch-process>.
        Frontier,
        /// Homestead: <https://github.com/ethereum/execution-specs/blob/master/network-upgrades/mainnet-upgrades/homestead.md>.
        Homestead,
        /// The DAO fork: <https://github.com/ethereum/execution-specs/blob/master/network-upgrades/mainnet-upgrades/dao-fork.md>.
        Dao,
        /// Tangerine: <https://github.com/ethereum/execution-specs/blob/master/network-upgrades/mainnet-upgrades/tangerine-whistle.md>.
        Tangerine,
        /// Spurious Dragon: <https://github.com/ethereum/execution-specs/blob/master/network-upgrades/mainnet-upgrades/spurious-dragon.md>.
        SpuriousDragon,
        /// Byzantium: <https://github.com/ethereum/execution-specs/blob/master/network-upgrades/mainnet-upgrades/byzantium.md>.
        Byzantium,
        /// Constantinople: <https://github.com/ethereum/execution-specs/blob/master/network-upgrades/mainnet-upgrades/constantinople.md>.
        Constantinople,
        /// Petersburg: <https://github.com/ethereum/execution-specs/blob/master/network-upgrades/mainnet-upgrades/petersburg.md>.
        Petersburg,
        /// Istanbul: <https://github.com/ethereum/execution-specs/blob/master/network-upgrades/mainnet-upgrades/istanbul.md>.
        Istanbul,
        /// Muir Glacier: <https://github.com/ethereum/execution-specs/blob/master/network-upgrades/mainnet-upgrades/muir-glacier.md>.
        MuirGlacier,
        /// Berlin: <https://github.com/ethereum/execution-specs/blob/master/network-upgrades/mainnet-upgrades/berlin.md>.
        Berlin,
        /// London: <https://github.com/ethereum/execution-specs/blob/master/network-upgrades/mainnet-upgrades/london.md>.
        London,
        /// Arrow Glacier: <https://github.com/ethereum/execution-specs/blob/master/network-upgrades/mainnet-upgrades/arrow-glacier.md>.
        ArrowGlacier,
        /// Gray Glacier: <https://github.com/ethereum/execution-specs/blob/master/network-upgrades/mainnet-upgrades/gray-glacier.md>.
        GrayGlacier,
        /// Paris: <https://github.com/ethereum/execution-specs/blob/master/network-upgrades/mainnet-upgrades/paris.md>.
        Paris,
        /// Shanghai: <https://github.com/ethereum/execution-specs/blob/master/network-upgrades/mainnet-upgrades/shanghai.md>.
        Shanghai,
        /// Initial hardfork for Scroll.
        PreBernoulli,
        /// Bernoulli update introduces:
        ///   - Enable `SHA-256` precompile.
        ///   - Use `EIP-4844` blobs for Data Availability (not part of layer2).
        Bernoulli,
        /// Curie update introduces:
        ///   - Support `EIP-1559` transactions.
        ///   - Support the `BASEFEE`, `MCOPY`, `TLOAD`, `TSTORE` opcodes.
        ///
        /// Although the Curie update include new opcodes in Cancun, the most important change
        /// `EIP-4844` is not included. So we sort it before Cancun.
        Curie,
        /// Euclid update introduces:
        ///   - Support `p256_verify` precompile.
        Euclid,
        /// Cancun.
        Cancun,
        /// Prague: <https://github.com/ethereum/execution-specs/blob/master/network-upgrades/mainnet-upgrades/prague.md>
        Prague,
        /// Osaka: <https://eips.ethereum.org/EIPS/eip-7607>
        Osaka,
    }
);

/// Scroll mainnet chain id
pub const SCROLL_MAINNET_CHAIN_ID: Chain = Chain::from_named(NamedChain::Scroll);
/// Scroll sepolia testnet chain id
pub const SCROLL_SEPOLIA_CHAIN_ID: Chain = Chain::from_named(NamedChain::ScrollSepolia);
// /// Scroll devnet chain id
// pub const SCROLL_DEVNET_CHAIN_ID: Chain = Chain::from_id(222222);

const SCROLL_MAINNET_GENESIS_HASH: B256 =
    b256!("bbc05efd412b7cd47a2ed0e5ddfcf87af251e414ea4c801d78b6784513180a80");
const SCROLL_SEPOLIA_GENESIS_HASH: B256 =
    b256!("aa62d1a8b2bffa9e5d2368b63aae0d98d54928bd713125e3fd9e5c896c68592c");

// FIXME: is that true?
const SCROLL_MAINNET_MAX_GAS_LIMIT: u64 = 10_000_000;
// FIXME: is that true?
const SCROLL_SEPOLIA_MAX_GAS_LIMIT: u64 = 8_000_000;

/// The scroll mainnet spec
pub static SCROLL_MAINNET: LazyLock<Arc<ChainSpec>> = LazyLock::new(|| {
    let mut spec = ChainSpec {
        chain: SCROLL_MAINNET_CHAIN_ID,
        genesis: serde_json::from_str(include_str!("../data/genesis/genesis.mainnet.json"))
            .expect("Can't deserialize scroll Mainnet genesis json"),
        genesis_hash: once_cell_set(SCROLL_MAINNET_GENESIS_HASH),
        genesis_header: Default::default(),
        // <https://scrollscan.com/block/0>
        paris_block_and_final_difficulty: Some((0, U256::from(1))),
        hardforks: ChainHardforks::new(
            ScrollHardfork::scroll_mainnet()
                .into_iter()
                .map(|(fork, cond)| (Box::new(fork) as Box<dyn Hardfork>, cond))
                .collect(),
        ),
        deposit_contract: None,
        // FIXME: is that true?
        base_fee_params: BaseFeeParamsKind::Constant(BaseFeeParams::ethereum()),
        max_gas_limit: SCROLL_MAINNET_MAX_GAS_LIMIT,
        ..Default::default()
    };
    spec.genesis.config.dao_fork_support = true;
    spec.into()
});

/// The scroll mainnet spec
pub static SCROLL_SEPOLIA: LazyLock<Arc<ChainSpec>> = LazyLock::new(|| {
    let mut spec = ChainSpec {
        chain: SCROLL_MAINNET_CHAIN_ID,
        genesis: serde_json::from_str(include_str!("../data/genesis/genesis.sepolia.json"))
            .expect("Can't deserialize scroll Mainnet genesis json"),
        genesis_hash: once_cell_set(SCROLL_SEPOLIA_GENESIS_HASH),
        genesis_header: Default::default(),
        // <https://sepolia.scrollscan.com/block/0>
        paris_block_and_final_difficulty: Some((0, U256::from(1))),
        hardforks: ChainHardforks::new(
            ScrollHardfork::sepolia_testnet()
                .into_iter()
                .map(|(fork, cond)| (Box::new(fork) as Box<dyn Hardfork>, cond))
                .collect(),
        ),
        deposit_contract: None,
        // FIXME: is that true?
        base_fee_params: BaseFeeParamsKind::Constant(BaseFeeParams::ethereum()),
        max_gas_limit: SCROLL_SEPOLIA_MAX_GAS_LIMIT,
        ..Default::default()
    };
    spec.genesis.config.dao_fork_support = true;
    spec.into()
});

impl ScrollHardfork {
    /// Retrieves the activation block for the specified hardfork on the given chain.
    pub fn activation_block(&self, chain: Chain) -> Option<u64> {
        if chain == SCROLL_MAINNET_CHAIN_ID {
            return self.scroll_mainnet_activation_block();
        }
        if chain == SCROLL_SEPOLIA_CHAIN_ID {
            return self.sepolia_testnet_activation_block();
        }
        // if chain == SCROLL_DEVNET_CHAIN_ID {
        //     return self.devnet_activation_block();
        // }

        None
    }

    /// Retrieves the activation block for the specified hardfork on the scroll mainnet.
    pub const fn scroll_mainnet_activation_block(&self) -> Option<u64> {
        match self {
            Self::Dao
            | Self::Tangerine
            | Self::SpuriousDragon
            | Self::Byzantium
            | Self::Constantinople
            | Self::Petersburg
            | Self::Istanbul
            | Self::MuirGlacier
            | Self::Berlin
            | Self::London
            | Self::ArrowGlacier
            | Self::GrayGlacier
            | Self::Paris
            | Self::Shanghai
            | Self::PreBernoulli => Some(0),
            Self::Bernoulli => Some(5220340),
            Self::Curie => Some(7096836),
            _ => None,
        }
    }

    /// Retrieves the activation block for the specified hardfork on the scroll sepolia testnet.
    pub const fn sepolia_testnet_activation_block(&self) -> Option<u64> {
        match self {
            Self::Dao
            | Self::Tangerine
            | Self::SpuriousDragon
            | Self::Byzantium
            | Self::Constantinople
            | Self::Petersburg
            | Self::Istanbul
            | Self::MuirGlacier
            | Self::Berlin
            | Self::London
            | Self::ArrowGlacier
            | Self::GrayGlacier
            | Self::Paris
            | Self::Shanghai
            | Self::PreBernoulli => Some(0),
            Self::Bernoulli => Some(3747132),
            Self::Curie => Some(4740239),
            _ => None,
        }
    }

    // /// Retrieves the activation block for the specified hardfork on the scroll devnet.
    // pub const fn devnet_activation_block(&self) -> Option<u64> {
    //     match self {
    //         Self::Dao
    //         | Self::Tangerine
    //         | Self::SpuriousDragon
    //         | Self::Byzantium
    //         | Self::Constantinople
    //         | Self::Petersburg
    //         | Self::Istanbul
    //         | Self::MuirGlacier
    //         | Self::Berlin
    //         | Self::London
    //         | Self::ArrowGlacier
    //         | Self::GrayGlacier
    //         | Self::Paris
    //         | Self::Shanghai
    //         | Self::PreBernoulli
    //         | Self::Bernoulli => Some(0),
    //         Self::Curie => Some(5),
    //         _ => None,
    //     }
    // }

    /// Retrieves the activation timestamp for the specified hardfork on the given chain.
    pub fn activation_timestamp(&self, _chain: Chain) -> Option<u64> {
        None
    }

    /// Ethereum scroll_mainnet list of hardforks.
    pub const fn scroll_mainnet() -> [(ScrollHardfork, ForkCondition); 17] {
        [
            (Self::Frontier, ForkCondition::Block(0)),
            (Self::Homestead, ForkCondition::Block(0)),
            (Self::Dao, ForkCondition::Block(0)),
            (Self::Tangerine, ForkCondition::Block(0)),
            (Self::SpuriousDragon, ForkCondition::Block(0)),
            (Self::Byzantium, ForkCondition::Block(0)),
            (Self::Constantinople, ForkCondition::Block(0)),
            (Self::Petersburg, ForkCondition::Block(0)),
            (Self::Istanbul, ForkCondition::Block(0)),
            (Self::MuirGlacier, ForkCondition::Block(0)),
            (Self::Berlin, ForkCondition::Block(0)),
            (Self::London, ForkCondition::Block(0)),
            (
                Self::Paris,
                ForkCondition::TTD {
                    fork_block: Some(0),
                    total_difficulty: U256::ZERO,
                },
            ),
            (Self::Shanghai, ForkCondition::Block(0)),
            (Self::PreBernoulli, ForkCondition::Block(0)),
            (Self::Bernoulli, ForkCondition::Block(5220340)),
            (Self::Curie, ForkCondition::Block(7096836)),
        ]
    }

    /// Ethereum scroll sepolia list of hardforks.
    pub const fn sepolia_testnet() -> [(ScrollHardfork, ForkCondition); 17] {
        [
            (Self::Frontier, ForkCondition::Block(0)),
            (Self::Homestead, ForkCondition::Block(0)),
            (Self::Dao, ForkCondition::Block(0)),
            (Self::Tangerine, ForkCondition::Block(0)),
            (Self::SpuriousDragon, ForkCondition::Block(0)),
            (Self::Byzantium, ForkCondition::Block(0)),
            (Self::Constantinople, ForkCondition::Block(0)),
            (Self::Petersburg, ForkCondition::Block(0)),
            (Self::Istanbul, ForkCondition::Block(0)),
            (Self::MuirGlacier, ForkCondition::Block(0)),
            (Self::Berlin, ForkCondition::Block(0)),
            (Self::London, ForkCondition::Block(0)),
            (
                Self::Paris,
                ForkCondition::TTD {
                    fork_block: Some(0),
                    total_difficulty: U256::ZERO,
                },
            ),
            (Self::Shanghai, ForkCondition::Block(0)),
            (Self::PreBernoulli, ForkCondition::Block(0)),
            (Self::Bernoulli, ForkCondition::Block(3747132)),
            (Self::Curie, ForkCondition::Block(4740239)),
        ]
    }

    // /// Ethereum scroll devnet list of hardforks.
    // pub const fn devnet() -> [(ScrollHardfork, ForkCondition); 17] {
    //     [
    //         (Self::Frontier, ForkCondition::Block(0)),
    //         (Self::Homestead, ForkCondition::Block(0)),
    //         (Self::Dao, ForkCondition::Block(0)),
    //         (Self::Tangerine, ForkCondition::Block(0)),
    //         (Self::SpuriousDragon, ForkCondition::Block(0)),
    //         (Self::Byzantium, ForkCondition::Block(0)),
    //         (Self::Constantinople, ForkCondition::Block(0)),
    //         (Self::Petersburg, ForkCondition::Block(0)),
    //         (Self::Istanbul, ForkCondition::Block(0)),
    //         (Self::MuirGlacier, ForkCondition::Block(0)),
    //         (Self::Berlin, ForkCondition::Block(0)),
    //         (Self::London, ForkCondition::Block(0)),
    //         (
    //             Self::Paris,
    //             ForkCondition::TTD {
    //                 fork_block: Some(0),
    //                 total_difficulty: U256::ZERO,
    //             },
    //         ),
    //         (Self::Shanghai, ForkCondition::Block(0)),
    //         (Self::PreBernoulli, ForkCondition::Block(0)),
    //         (Self::Bernoulli, ForkCondition::Block(0)),
    //         (Self::Curie, ForkCondition::Block(5)),
    //     ]
    // }
}

// FIXME: curie block
// fn curie_migrate(
//     db: &dyn DatabaseRef<Error = Infallible>,
// ) -> revm::primitives::HashMap<Address, Account> {
//     let l1_gas_price_oracle_addr = Address::from(l1_gas_price_oracle::ADDRESS.0);
//     let mut l1_gas_price_oracle_info = db
//         .basic_ref(l1_gas_price_oracle_addr)
//         .unwrap()
//         .unwrap_or_default();
//     // Set the new code
//     let code = Bytecode::new_raw(Bytes::from_static(l1_gas_price_oracle::V2_BYTECODE));
//     l1_gas_price_oracle_info.code_size = code.len();
//     l1_gas_price_oracle_info.code_hash = code.hash_slow();
//     l1_gas_price_oracle_info.poseidon_code_hash = code.poseidon_hash_slow();
//     l1_gas_price_oracle_info.code = Some(code);
//
//     let l1_gas_price_oracle_acc = Account {
//         info: l1_gas_price_oracle_info,
//         storage: EvmStorage::from_iter([
//             (
//                 l1_gas_price_oracle::IS_CURIE_SLOT,
//                 EvmStorageSlot::new(U256::from(1)),
//             ),
//             (
//                 l1_gas_price_oracle::L1_BLOB_BASEFEE_SLOT,
//                 EvmStorageSlot::new(U256::from(1)),
//             ),
//             (
//                 l1_gas_price_oracle::COMMIT_SCALAR_SLOT,
//                 EvmStorageSlot::new(l1_gas_price_oracle::INITIAL_COMMIT_SCALAR),
//             ),
//             (
//                 l1_gas_price_oracle::BLOB_SCALAR_SLOT,
//                 EvmStorageSlot::new(l1_gas_price_oracle::INITIAL_BLOB_SCALAR),
//             ),
//         ]),
//         status: AccountStatus::Touched,
//     };
//
//     revm::primitives::HashMap::from_iter([(l1_gas_price_oracle_addr, l1_gas_price_oracle_acc)])
// }
