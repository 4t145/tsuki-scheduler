#[cfg(feature = "async-std")]
mod async_std;
#[cfg(feature = "async-std")]
pub use async_std::*;
#[cfg(feature = "tokio")]
mod tokio;
#[cfg(feature = "tokio")]
pub use tokio::*;
#[cfg(feature = "thread")]
mod thread;
#[cfg(feature = "thread")]
pub use thread::*;
#[cfg(feature = "wasm")]
mod wasm;
#[cfg(feature = "wasm")]
pub use wasm::*;

pub trait Runtime {
    type Handle;
}
