//! Roblox property types and serialization
//!
//! This module defines all Roblox property types and their JSON representations.
//! The goal is to capture every possible property value with full fidelity.

mod harness;
mod instance;
mod project;
mod properties;
mod wally;

pub use harness::*;
pub use instance::*;
pub use project::*;
pub use properties::*;
pub use wally::*;
