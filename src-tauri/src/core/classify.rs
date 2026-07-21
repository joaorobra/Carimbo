//! Content-type classification for a text clip.
//!
//! Copying a URL, an email, a hex color, or a file path each affords a different
//! action (open in browser, compose mail, preview the color, reveal in Explorer).
//! We classify once at capture time and store the result on the row so the UI can
//! show a badge and the right action without re-scanning on every render.
//!
//! Classification is intentionally conservative: when in doubt it returns
//! [`ContentType::Text`]. A false "plain text" is harmless (you just don't get a
//! shortcut action); a false "URL" that opens the browser on junk is not.

use serde::{Deserialize, Serialize};

/// How a clip's content should be rendered and acted on. Mirrors the
/// `content_type` column; serialized camelCase for the frontend.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ContentType {
    Text,
    Url,
    Email,
    /// A CSS hex color like `#1a2b3c` or `#fff`.
    Color,
    /// A single filesystem path (Windows `C:\...` or UNC `\\server\...`).
    Path,
    /// One or more newline-separated file paths from an Explorer copy (CF_HDROP).
    Files,
    /// Bitmap image clip (mirrors `ClipKind::Image`). Set by the caller, never
    /// inferred from text.
    Image,
}

impl ContentType {
    pub fn as_str(self) -> &'static str {
        match self {
            ContentType::Text => "text",
            ContentType::Url => "url",
            ContentType::Email => "email",
            ContentType::Color => "color",
            ContentType::Path => "path",
            ContentType::Files => "files",
            ContentType::Image => "image",
        }
    }
    pub fn from_str(s: &str) -> Self {
        match s {
            "url" => ContentType::Url,
            "email" => ContentType::Email,
            "color" => ContentType::Color,
            "path" => ContentType::Path,
            "files" => ContentType::Files,
            "image" => ContentType::Image,
            _ => ContentType::Text,
        }
    }
}

/// Classify a text clip's content. Whitespace at the edges is ignored so a
/// copied URL with a trailing newline still classifies as a URL.
pub fn classify_text(content: &str) -> ContentType {
    let s = content.trim();
    if s.is_empty() {
        return ContentType::Text;
    }

    // Multi-line: only interesting if every line is a path (a file-drop copy).
    if s.contains('\n') {
        if s.lines().map(str::trim).filter(|l| !l.is_empty()).all(is_path) {
            return ContentType::Files;
        }
        return ContentType::Text;
    }

    if is_url(s) {
        return ContentType::Url;
    }
    if is_email(s) {
        return ContentType::Email;
    }
    if is_hex_color(s) {
        return ContentType::Color;
    }
    if is_path(s) {
        return ContentType::Path;
    }
    ContentType::Text
}

fn is_url(s: &str) -> bool {
    // Scheme we're willing to open. No spaces, and something after the scheme.
    if s.contains(char::is_whitespace) {
        return false;
    }
    let lower = s.to_ascii_lowercase();
    for scheme in ["https://", "http://", "ftp://", "mailto:"] {
        if lower.starts_with(scheme) && s.len() > scheme.len() {
            return true;
        }
    }
    false
}

fn is_email(s: &str) -> bool {
    // One '@', non-empty local part, and a dotted domain with no whitespace.
    if s.contains(char::is_whitespace) {
        return false;
    }
    let mut parts = s.splitn(2, '@');
    let (local, domain) = match (parts.next(), parts.next()) {
        (Some(l), Some(d)) => (l, d),
        _ => return false,
    };
    if local.is_empty() || domain.is_empty() || domain.contains('@') {
        return false;
    }
    // Domain must contain a dot with labels on both sides.
    match domain.rsplit_once('.') {
        Some((host, tld)) => !host.is_empty() && tld.len() >= 2 && tld.chars().all(|c| c.is_ascii_alphabetic()),
        None => false,
    }
}

fn is_hex_color(s: &str) -> bool {
    let Some(hex) = s.strip_prefix('#') else {
        return false;
    };
    matches!(hex.len(), 3 | 4 | 6 | 8) && hex.bytes().all(|b| b.is_ascii_hexdigit())
}

/// A single Windows path: drive-letter (`C:\...`) or UNC (`\\server\share`).
/// Deliberately Windows-shaped since that's the only platform we capture on.
fn is_path(s: &str) -> bool {
    let bytes = s.as_bytes();
    // UNC: \\host\...
    if s.starts_with("\\\\") && s.len() > 3 {
        return true;
    }
    // Drive path: X:\ or X:/  (X an ASCII letter)
    if bytes.len() >= 3
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && (bytes[2] == b'\\' || bytes[2] == b'/')
    {
        return true;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn urls() {
        assert_eq!(classify_text("https://example.com/x"), ContentType::Url);
        assert_eq!(classify_text("  http://a.b\n"), ContentType::Url);
        assert_eq!(classify_text("https://"), ContentType::Text); // nothing after scheme
        assert_eq!(classify_text("not a url"), ContentType::Text);
    }

    #[test]
    fn emails() {
        assert_eq!(classify_text("rubem@arvore.com.br"), ContentType::Email);
        assert_eq!(classify_text("a@b.co"), ContentType::Email);
        assert_eq!(classify_text("@nope.com"), ContentType::Text);
        assert_eq!(classify_text("a@b"), ContentType::Text); // no TLD
        assert_eq!(classify_text("two @ words.com"), ContentType::Text);
    }

    #[test]
    fn colors() {
        assert_eq!(classify_text("#1a2b3c"), ContentType::Color);
        assert_eq!(classify_text("#FFF"), ContentType::Color);
        assert_eq!(classify_text("#12345678"), ContentType::Color);
        assert_eq!(classify_text("#12345"), ContentType::Text); // wrong length
        assert_eq!(classify_text("#gggggg"), ContentType::Text); // non-hex
    }

    #[test]
    fn paths() {
        assert_eq!(classify_text("C:\\Users\\me\\file.txt"), ContentType::Path);
        assert_eq!(classify_text("D:/data/x"), ContentType::Path);
        assert_eq!(classify_text("\\\\server\\share\\f"), ContentType::Path);
        assert_eq!(classify_text("just text"), ContentType::Text);
    }

    #[test]
    fn multi_line_files() {
        let drop = "C:\\a\\one.png\r\nC:\\a\\two.png";
        assert_eq!(classify_text(drop), ContentType::Files);
        // A path plus a non-path line is just text.
        assert_eq!(classify_text("C:\\a\\one.png\nhello"), ContentType::Text);
    }

    #[test]
    fn plain_text() {
        assert_eq!(classify_text("hello world"), ContentType::Text);
        assert_eq!(classify_text(""), ContentType::Text);
    }
}
