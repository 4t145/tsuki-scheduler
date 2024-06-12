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
#[cfg(feature = "promise")]
mod promise;
#[cfg(feature = "promise")]
pub use promise::*;

mod local;
pub use local::*;

pub trait Runtime {
    type Handle;
}
