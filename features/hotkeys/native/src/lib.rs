#![deny(unsafe_code)]

//! Platform-neutral Hotkeys domain, configuration compiler, and feature-owned storage.
//! Database calls are configuration-plane work and must never run on an input callback.

pub mod application;
pub mod domain;
pub mod feature;
pub mod persistence;

pub use feature::{FEATURE_ID, FEATURE_SCHEMA_VERSION, descriptor};
