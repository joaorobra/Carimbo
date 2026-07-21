//! Text transforms applied to a clipboard entry before copy/paste.
//!
//! These turn the clipboard from passive storage into a small utility: "paste as
//! plain text / UPPERCASE / a slug / decoded base64", etc. All transforms are
//! pure `&str -> String` so they are trivially unit-testable and OS-agnostic;
//! the command layer just picks one by [`TransformKind`] and feeds the result to
//! the existing paste/copy path.

use serde::{Deserialize, Serialize};

/// A named text transform the UI can offer on a text clip. Serialized in
/// camelCase so the frontend can pass e.g. `"upperCase"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TransformKind {
    /// Identity — used for "paste as plain text": the content is already a plain
    /// string, so this is the no-op that the plain-text action maps to. Kept
    /// explicit so the action set is uniform.
    Plain,
    UpperCase,
    LowerCase,
    /// Capitalize the first letter of each whitespace-separated word.
    TitleCase,
    /// Trim leading/trailing whitespace on the whole string.
    Trim,
    /// Collapse all runs of whitespace (including newlines) to single spaces and
    /// trim — turns wrapped/multi-line text into one clean line.
    SingleLine,
    /// URL/anchor-friendly slug: lowercase, non-alphanumerics to hyphens,
    /// collapsed and edge-trimmed.
    Slug,
    Base64Encode,
    /// Decode base64. Falls back to the original text if it isn't valid base64,
    /// so the action never silently produces garbage or an empty paste.
    Base64Decode,
}

impl TransformKind {
    /// Apply the transform to `input`.
    pub fn apply(self, input: &str) -> String {
        match self {
            TransformKind::Plain => input.to_string(),
            TransformKind::UpperCase => input.to_uppercase(),
            TransformKind::LowerCase => input.to_lowercase(),
            TransformKind::TitleCase => title_case(input),
            TransformKind::Trim => input.trim().to_string(),
            TransformKind::SingleLine => single_line(input),
            TransformKind::Slug => slug(input),
            TransformKind::Base64Encode => base64_encode(input.as_bytes()),
            TransformKind::Base64Decode => base64_decode(input)
                .and_then(|b| String::from_utf8(b).ok())
                .unwrap_or_else(|| input.to_string()),
        }
    }
}

fn title_case(s: &str) -> String {
    // Capitalize the first char of each word; lowercase the rest so "HELLO" ->
    // "Hello". Whitespace (any kind) is preserved verbatim.
    let mut out = String::with_capacity(s.len());
    let mut at_word_start = true;
    for ch in s.chars() {
        if ch.is_whitespace() {
            at_word_start = true;
            out.push(ch);
        } else if at_word_start {
            out.extend(ch.to_uppercase());
            at_word_start = false;
        } else {
            out.extend(ch.to_lowercase());
        }
    }
    out
}

fn single_line(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn slug(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut prev_hyphen = false;
    for ch in s.chars() {
        if ch.is_ascii_alphanumeric() {
            out.extend(ch.to_lowercase());
            prev_hyphen = false;
        } else if !prev_hyphen && !out.is_empty() {
            // Collapse any run of separators to a single hyphen.
            out.push('-');
            prev_hyphen = true;
        }
    }
    // Trim a trailing hyphen produced by trailing separators.
    if out.ends_with('-') {
        out.pop();
    }
    out
}

// --- base64 (standard alphabet, with padding) -------------------------------
// Implemented inline to avoid pulling a crate for a few dozen lines. Standard
// RFC 4648 alphabet; decode is lenient about surrounding whitespace/newlines
// (common when text was copied from an email or code block).

const B64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

fn base64_encode(bytes: &[u8]) -> String {
    let mut out = String::with_capacity((bytes.len() + 2) / 3 * 4);
    for chunk in bytes.chunks(3) {
        let b = [
            chunk[0],
            *chunk.get(1).unwrap_or(&0),
            *chunk.get(2).unwrap_or(&0),
        ];
        let n = ((b[0] as u32) << 16) | ((b[1] as u32) << 8) | (b[2] as u32);
        out.push(B64[((n >> 18) & 63) as usize] as char);
        out.push(B64[((n >> 12) & 63) as usize] as char);
        out.push(if chunk.len() > 1 {
            B64[((n >> 6) & 63) as usize] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            B64[(n & 63) as usize] as char
        } else {
            '='
        });
    }
    out
}

fn base64_decode(s: &str) -> Option<Vec<u8>> {
    fn val(c: u8) -> Option<u32> {
        match c {
            b'A'..=b'Z' => Some((c - b'A') as u32),
            b'a'..=b'z' => Some((c - b'a' + 26) as u32),
            b'0'..=b'9' => Some((c - b'0' + 52) as u32),
            b'+' => Some(62),
            b'/' => Some(63),
            _ => None,
        }
    }
    // Strip whitespace and padding; reject any other stray character so a plain
    // sentence isn't mistaken for base64.
    let cleaned: Vec<u8> = s
        .bytes()
        .filter(|b| !b.is_ascii_whitespace() && *b != b'=')
        .collect();
    if cleaned.is_empty() {
        return None;
    }
    let mut out = Vec::with_capacity(cleaned.len() / 4 * 3);
    let mut acc: u32 = 0;
    let mut bits = 0u32;
    for c in cleaned {
        let v = val(c)?; // any non-base64 char → not base64
        acc = (acc << 6) | v;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::TransformKind::*;

    #[test]
    fn case_transforms() {
        assert_eq!(UpperCase.apply("Olá mundo"), "OLÁ MUNDO");
        assert_eq!(LowerCase.apply("Olá MUNDO"), "olá mundo");
        assert_eq!(TitleCase.apply("hello WORLD foo"), "Hello World Foo");
    }

    #[test]
    fn whitespace_transforms() {
        assert_eq!(Trim.apply("  hi  "), "hi");
        assert_eq!(SingleLine.apply("a\n  b\t c  "), "a b c");
    }

    #[test]
    fn slug_transform() {
        assert_eq!(Slug.apply("Hello, World! 123"), "hello-world-123");
        assert_eq!(Slug.apply("  --Trim-- "), "trim");
        assert_eq!(Slug.apply("já vou"), "j-vou"); // non-ascii dropped
    }

    #[test]
    fn base64_roundtrip() {
        let s = "Carimbo ✓";
        let enc = Base64Encode.apply(s);
        assert_eq!(Base64Decode.apply(&enc), s);
    }

    #[test]
    fn base64_known_vector() {
        assert_eq!(Base64Encode.apply("Man"), "TWFu");
        assert_eq!(Base64Encode.apply("Ma"), "TWE=");
        assert_eq!(Base64Encode.apply("M"), "TQ==");
    }

    #[test]
    fn base64_decode_falls_back_on_non_base64() {
        // A plain sentence isn't base64 — decode returns the input unchanged.
        let plain = "This is not base64!";
        assert_eq!(Base64Decode.apply(plain), plain);
    }

    #[test]
    fn plain_is_identity() {
        assert_eq!(Plain.apply("as-is\ntext"), "as-is\ntext");
    }
}
