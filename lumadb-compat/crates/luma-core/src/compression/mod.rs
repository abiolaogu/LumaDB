pub mod gorilla;
pub mod log; // Added log compression
pub use gorilla::{GorillaEncoder, DeltaDeltaEncoder};
