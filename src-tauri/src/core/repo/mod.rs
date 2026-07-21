//! Repository layer — the single seam through which all data access flows. A
//! future SyncEngine (phase 2) observes the same repos, so keeping every read
//! and write here (never raw SQL in commands) is what makes sync a drop-in later.

pub mod clip_repo;
pub mod folder_repo;
pub mod settings_repo;
pub mod snippet_repo;
