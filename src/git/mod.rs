//! Git operations shelled out to the local `git` binary.

mod branch;
mod log;
mod rebase;
mod status;

pub use branch::branch;
pub use log::log;
pub use rebase::rebase;
pub use status::status;
