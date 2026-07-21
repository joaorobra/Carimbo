//! Time and id helpers shared by the repositories.

use crate::core::models::Timestamp;

/// Current unix time in milliseconds UTC. Uses SystemTime so it works without a
/// timezone database; wall-clock jumps are acceptable for our updated_at/LWW use.
pub fn now_ms() -> Timestamp {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as Timestamp)
        .unwrap_or(0)
}

/// A fresh UUID v4 string, used for all primary keys.
pub fn new_id() -> String {
    uuid::Uuid::new_v4().to_string()
}
