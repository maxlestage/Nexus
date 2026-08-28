//! Cœur logique de la manette une main "Nexus One".
//!
//! Ce crate est `no_std`-compatible : il tourne à l'identique dans le
//! firmware ESP32, dans l'application iPhone et dans les tests sur PC.
//! Tout ce qui est testable sans matériel vit ici.

#![cfg_attr(not(feature = "std"), no_std)]

pub mod buttons;
pub mod config;
pub mod engine;
pub mod macros_engine;
pub mod procon;
pub mod protocol;
pub mod report;
pub mod stats;
pub mod turbo;

pub use buttons::{PhysicalInput, SwitchButton, NUM_PHYSICAL};
pub use config::{Config, LedConfig, LedMode, StickTarget};
pub use engine::{Engine, EngineOutput, InputFrame};
