//! OS-agnostic business logic. Depends on `platform` only through abstractions,
//! never on Win32 directly, so it stays portable and unit-testable.

pub mod backup;
pub mod classify;
pub mod clock;
pub mod db;
pub mod error;
pub mod expansion;
pub mod import;
pub mod models;
pub mod region;
pub mod repo;
pub mod seed;
pub mod tokens;
pub mod transform;
