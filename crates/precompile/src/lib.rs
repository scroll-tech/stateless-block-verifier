#[cfg(not(feature = "scroll"))]
mod ethereum;
#[cfg(feature = "openvm")]
mod openvm;
#[cfg(feature = "scroll")]
mod scroll;

pub struct PrecompileProvider;
