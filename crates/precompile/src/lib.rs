//! sbv precompiles provider

/// A precompile provider that patches the precompiles when runs on zkVM with OpenVM enabled.
#[derive(Debug, Default, Copy, Clone)]
pub struct PrecompileProvider;

#[cfg(feature = "scroll")]
mod scroll {}

#[cfg(not(feature = "scroll"))]
mod ethereum {
    compile_error!("unimplemented");
}
