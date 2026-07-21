//! Error type for the core (data + logic) layer.

use serde::Serialize;

pub type CoreResult<T> = Result<T, CoreError>;

#[derive(Debug, thiserror::Error)]
pub enum CoreError {
    #[error("database error: {0}")]
    Sqlite(#[from] rusqlite::Error),

    #[error("migration error: {0}")]
    Migration(String),

    #[error("not found")]
    NotFound,

    #[error("a snippet with trigger {0:?} already exists")]
    DuplicateTrigger(String),

    #[error("invalid input: {0}")]
    Invalid(String),

    #[error("{0}")]
    Other(String),
}

impl CoreError {
    /// Stable machine-readable tag so the frontend can branch on the error kind
    /// (e.g. show a friendly duplicate-trigger message) without parsing strings.
    fn kind(&self) -> &'static str {
        match self {
            CoreError::NotFound => "not_found",
            CoreError::DuplicateTrigger(_) => "duplicate_trigger",
            CoreError::Invalid(_) => "invalid",
            CoreError::Migration(_) => "migration",
            CoreError::Sqlite(_) => "sqlite",
            CoreError::Other(_) => "other",
        }
    }
}

// Tauri commands surface errors to JS as `{ kind, message }`.
impl Serialize for CoreError {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let mut st = s.serialize_struct("CommandError", 2)?;
        st.serialize_field("kind", self.kind())?;
        st.serialize_field("message", &self.to_string())?;
        st.end()
    }
}
