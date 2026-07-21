//! Domain models. These serialize directly to the frontend (camelCase) and map
//! to rows in the schema. Timestamps are unix milliseconds UTC.

use serde::{Deserialize, Serialize};

use crate::core::classify::ContentType;

pub type Timestamp = i64;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum FolderKind {
    Snippet,
    Clipboard,
}

impl FolderKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            FolderKind::Snippet => "snippet",
            FolderKind::Clipboard => "clipboard",
        }
    }
    pub fn from_str(s: &str) -> Self {
        match s {
            "clipboard" => FolderKind::Clipboard,
            _ => FolderKind::Snippet,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Folder {
    pub id: String,
    pub name: String,
    pub kind: FolderKind,
    pub parent_id: Option<String>,
    pub sort_order: i64,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Snippet {
    pub id: String,
    pub folder_id: Option<String>,
    pub name: String,
    pub trigger: Option<String>,
    pub body: String,
    /// Optional rich-text (HTML) form of the body. `None` -> plain snippet.
    /// When present, insertion offers formatted output to rich targets while
    /// `body` is the plain-text fallback. Defaulted for backward-compatible
    /// deserialization of older backups that predate the field.
    #[serde(default)]
    pub body_html: Option<String>,
    pub is_favorite: bool,
    pub use_count: i64,
    pub last_used_at: Option<Timestamp>,
    pub sort_order: i64,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

/// Fields a client sends to create a snippet. The server assigns id/timestamps.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewSnippet {
    pub name: String,
    pub trigger: Option<String>,
    pub body: String,
    #[serde(default)]
    pub body_html: Option<String>,
    pub folder_id: Option<String>,
    #[serde(default)]
    pub is_favorite: bool,
}

/// Fields a client sends to update a snippet. `id` identifies the row; other
/// fields fully replace the stored values (no partial-merge ambiguity).
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSnippet {
    pub id: String,
    pub name: String,
    pub trigger: Option<String>,
    pub body: String,
    #[serde(default)]
    pub body_html: Option<String>,
    pub folder_id: Option<String>,
    pub is_favorite: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ClipKind {
    Text,
    Image,
}

impl ClipKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            ClipKind::Text => "text",
            ClipKind::Image => "image",
        }
    }
    pub fn from_str(s: &str) -> Self {
        match s {
            "image" => ClipKind::Image,
            _ => ClipKind::Text,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClipEntry {
    pub id: String,
    pub kind: ClipKind,
    /// Refined content classification (url/email/color/path/files/…) driving the
    /// row's badge and type-specific action. `image` for image clips.
    pub content_type: ContentType,
    /// Text content (None for images).
    pub content: Option<String>,
    /// Absolute path to the PNG on disk (None for text).
    pub image_path: Option<String>,
    pub preview: String,
    pub is_pinned: bool,
    pub folder_id: Option<String>,
    /// Best-effort foreground process name at capture time (e.g. `chrome.exe`).
    pub source_app: Option<String>,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

/// A newly captured clipboard item, pre-persistence.
#[derive(Debug, Clone)]
pub struct NewClip {
    pub kind: ClipKind,
    pub content_type: ContentType,
    pub content: Option<String>,
    pub image_path: Option<String>,
    pub preview: String,
    pub content_hash: String,
    pub source_app: Option<String>,
}
