//! The interactive terminal UI: an entry-point panel listing every locally
//! tracked stack (like LazyGit's branch panel, but for stacks), drilling
//! into a per-stack layer view.

mod app;
mod layer_resource;
mod stack_layers;
mod stack_list;

pub use app::run;
