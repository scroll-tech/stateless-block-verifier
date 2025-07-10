//! BN128 precompiles added in [`EIP-1962`](https://eips.ethereum.org/EIPS/eip-1962)
use sbv_primitives::types::revm::precompile::{PrecompileWithAddress, bn128};

#[cfg(not(feature = "openvm-bn128"))]
pub use bn128::{run_add, run_mul, run_pair};
#[cfg(feature = "openvm-bn128")]
pub use imps::{run_add, run_mul, run_pair};

#[cfg(feature = "openvm-bn128")]
mod openvm;

/// Bn128 add precompile
pub mod add {
    use super::*;

    pub use bn128::add::{ADDRESS, BYZANTIUM_ADD_GAS_COST, ISTANBUL_ADD_GAS_COST};

    /// Bn128 add precompile with ISTANBUL gas rules
    pub const ISTANBUL: PrecompileWithAddress =
        PrecompileWithAddress(ADDRESS, |input, gas_limit| {
            run_add(input, ISTANBUL_ADD_GAS_COST, gas_limit)
        });

    /// Bn128 add precompile with BYZANTIUM gas rules
    pub const BYZANTIUM: PrecompileWithAddress =
        PrecompileWithAddress(ADDRESS, |input, gas_limit| {
            run_add(input, BYZANTIUM_ADD_GAS_COST, gas_limit)
        });
}

/// Bn128 mul precompile
pub mod mul {
    use super::*;

    pub use bn128::mul::{ADDRESS, BYZANTIUM_MUL_GAS_COST, ISTANBUL_MUL_GAS_COST};

    /// Bn128 mul precompile with ISTANBUL gas rules
    pub const ISTANBUL: PrecompileWithAddress =
        PrecompileWithAddress(ADDRESS, |input, gas_limit| {
            run_mul(input, ISTANBUL_MUL_GAS_COST, gas_limit)
        });

    /// Bn128 mul precompile with BYZANTIUM gas rules
    pub const BYZANTIUM: PrecompileWithAddress =
        PrecompileWithAddress(ADDRESS, |input, gas_limit| {
            run_mul(input, BYZANTIUM_MUL_GAS_COST, gas_limit)
        });
}

/// Bn128 pair precompile
pub mod pair {
    use super::*;

    pub use bn128::pair::{
        ADDRESS, BYZANTIUM_PAIR_BASE, BYZANTIUM_PAIR_PER_POINT, ISTANBUL_PAIR_BASE,
        ISTANBUL_PAIR_PER_POINT,
    };

    /// Bn128 pair precompile with ISTANBUL gas rules
    pub const ISTANBUL: PrecompileWithAddress =
        PrecompileWithAddress(ADDRESS, |input, gas_limit| {
            run_pair(
                input,
                ISTANBUL_PAIR_PER_POINT,
                ISTANBUL_PAIR_BASE,
                gas_limit,
            )
        });

    /// Bn128 pair precompile with BYZANTIUM gas rules
    pub const BYZANTIUM: PrecompileWithAddress =
        PrecompileWithAddress(ADDRESS, |input, gas_limit| {
            run_pair(
                input,
                BYZANTIUM_PAIR_PER_POINT,
                BYZANTIUM_PAIR_BASE,
                gas_limit,
            )
        });

    #[cfg(feature = "scroll")]
    #[cfg_attr(docsrs, doc(cfg(feature = "scroll")))]
    pub use scroll::*;
    #[cfg(feature = "scroll")]
    mod scroll {
        use super::*;
        use bn128::PAIR_ELEMENT_LEN;
        use sbv_primitives::types::revm::precompile::{PrecompileError, PrecompileResult};

        /// The number of pairing inputs per pairing operation. If the inputs provided to the precompile
        /// call are < 4, we append (G1::infinity, G2::generator) until we have the required no. of
        /// inputs.
        const BERNOULLI_LEN_LIMIT: usize = 4;

        /// The Bn128 pair precompile with BERNOULLI input rules.
        pub const BERNOULLI: PrecompileWithAddress = PrecompileWithAddress(ADDRESS, bernoulli_run);

        /// The bernoulli Bn128 pair precompile implementation.
        ///
        /// # Errors
        /// - `PrecompileError::Other("BN128PairingInputOverflow: input overflow".into())` if the input
        ///   length is greater than 768 bytes.
        fn bernoulli_run(input: &[u8], gas_limit: u64) -> PrecompileResult {
            if input.len() > BERNOULLI_LEN_LIMIT * PAIR_ELEMENT_LEN {
                return Err(PrecompileError::Other(
                    "BN128PairingInputOverflow: input overflow".into(),
                ));
            }
            run_pair(
                input,
                ISTANBUL_PAIR_PER_POINT,
                ISTANBUL_PAIR_BASE,
                gas_limit,
            )
        }

        /// The Bn128 pair precompile in FEYNMAN hardfork.
        pub const FEYNMAN: PrecompileWithAddress = ISTANBUL;
    }
}

#[cfg(feature = "openvm-bn128")]
mod imps {
    use super::*;
    use openvm::{
        encode_g1_point, g1_point_add, g1_point_mul, pairing_check, read_g1_point, read_g2_point,
        read_scalar,
    };
    use sbv_primitives::types::revm::precompile::{
        PrecompileError, PrecompileOutput, PrecompileResult,
        bn128::{ADD_INPUT_LEN, MUL_INPUT_LEN, PAIR_ELEMENT_LEN},
        utilities::{bool_to_bytes32, right_pad},
    };

    // Copied from https://github.com/bluealloy/revm/blob/v75/crates/precompile/src/bn128.rs Under MIT License

    /// FQ_LEN specifies the number of bytes needed to represent an
    /// Fq element. This is an element in the base field of BN254.
    ///
    /// Note: The base field is used to define G1 and G2 elements.
    const FQ_LEN: usize = 32;

    /// SCALAR_LEN specifies the number of bytes needed to represent an Fr element.
    /// This is an element in the scalar field of BN254.
    const SCALAR_LEN: usize = 32;

    /// FQ2_LEN specifies the number of bytes needed to represent an
    /// Fq^2 element.
    ///
    /// Note: This is the quadratic extension of Fq, and by definition
    /// means we need 2 Fq elements.
    const FQ2_LEN: usize = 2 * FQ_LEN;

    /// G1_LEN specifies the number of bytes needed to represent a G1 element.
    ///
    /// Note: A G1 element contains 2 Fq elements.
    const G1_LEN: usize = 2 * FQ_LEN;
    /// G2_LEN specifies the number of bytes needed to represent a G2 element.
    ///
    /// Note: A G2 element contains 2 Fq^2 elements.
    const G2_LEN: usize = 2 * FQ2_LEN;

    /// Run the Bn128 add precompile
    pub fn run_add(input: &[u8], gas_cost: u64, gas_limit: u64) -> PrecompileResult {
        if gas_cost > gas_limit {
            return Err(PrecompileError::OutOfGas);
        }

        let input = right_pad::<ADD_INPUT_LEN>(input);

        let p1 = read_g1_point(&input[..G1_LEN])?;
        let p2 = read_g1_point(&input[G1_LEN..])?;
        let result = g1_point_add(p1, p2);

        let output = encode_g1_point(result);

        Ok(PrecompileOutput::new(gas_cost, output.into()))
    }

    /// Run the Bn128 mul precompile
    pub fn run_mul(input: &[u8], gas_cost: u64, gas_limit: u64) -> PrecompileResult {
        if gas_cost > gas_limit {
            return Err(PrecompileError::OutOfGas);
        }

        let input = right_pad::<MUL_INPUT_LEN>(input);

        let p = read_g1_point(&input[..G1_LEN])?;

        let scalar = read_scalar(&input[G1_LEN..G1_LEN + SCALAR_LEN]);
        let result = g1_point_mul(p, scalar);

        let output = encode_g1_point(result);

        Ok(PrecompileOutput::new(gas_cost, output.into()))
    }

    /// Run the Bn128 pair precompile
    pub fn run_pair(
        input: &[u8],
        pair_per_point_cost: u64,
        pair_base_cost: u64,
        gas_limit: u64,
    ) -> PrecompileResult {
        let gas_used =
            (input.len() / PAIR_ELEMENT_LEN) as u64 * pair_per_point_cost + pair_base_cost;
        if gas_used > gas_limit {
            return Err(PrecompileError::OutOfGas);
        }

        if input.len() % PAIR_ELEMENT_LEN != 0 {
            return Err(PrecompileError::Bn128PairLength);
        }

        let elements = input.len() / PAIR_ELEMENT_LEN;

        let mut points = Vec::with_capacity(elements);

        for idx in 0..elements {
            // Offset to the start of the pairing element at index `idx` in the byte slice
            let start = idx * PAIR_ELEMENT_LEN;
            let g1_start = start;
            // Offset to the start of the G2 element in the pairing element
            // This is where G1 ends.
            let g2_start = start + G1_LEN;

            let encoded_g1_element = &input[g1_start..g2_start];
            let encoded_g2_element = &input[g2_start..g2_start + G2_LEN];

            // If either the G1 or G2 element is the encoded representation
            // of the point at infinity, then these two points are no-ops
            // in the pairing computation.
            //
            // Note: we do not skip the validation of these two elements even if
            // one of them is the point at infinity because we could have G1 be
            // the point at infinity and G2 be an invalid element or vice versa.
            // In that case, the precompile should error because one of the elements
            // was invalid.
            let g1_is_zero = encoded_g1_element.iter().all(|i| *i == 0);
            let g2_is_zero = encoded_g2_element.iter().all(|i| *i == 0);

            // Get G1 and G2 points from the input
            let a = read_g1_point(encoded_g1_element)?;
            let b = read_g2_point(encoded_g2_element)?;

            if !g1_is_zero && !g2_is_zero {
                points.push((a, b));
            }
        }

        let success = pairing_check(&points);

        Ok(PrecompileOutput::new(gas_used, bool_to_bytes32(success)))
    }
}
