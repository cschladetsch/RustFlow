pub mod error;
pub mod kernel;
pub mod runtime;
pub mod traits;
pub mod types;

pub mod flow {
    pub mod barrier;
    pub mod channel;
    pub mod coroutine;
    pub mod future;
    pub mod group;
    pub mod node;
    pub mod sequence;
    pub mod timer;
}

pub mod factory;
pub mod logger;
pub mod threading;

pub use error::*;
pub use factory::*;
pub use kernel::*;
pub use logger::*;
pub use runtime::*;
pub use traits::*;
pub use types::*;

pub fn init_tracing() {
    tracing_subscriber::fmt::init();
}

pub type Result<T> = std::result::Result<T, FlowError>;