//! Dynamic token expansion for snippet bodies.
//!
//! Two kinds of substitution, both resolved at insertion time:
//!
//! Automatic tokens (single brace) — resolved silently, no user input:
//!   {date}      -> local date. Order follows the user's Region: US (default)
//!                  is mm/dd/YYYY, Brazil is dd/mm/YYYY.
//!   {date+7d} / {date-3d} -> {date} shifted by a whole number of days (also
//!                  {w}eeks, {m}onths — approximated as 30 days — and {y}ears).
//!   {time}      -> local time, HH:MM
//!   {datetime}  -> "{date} {time}"
//!   {clipboard} -> the provided clipboard text (what was on the clipboard
//!                  before injection), or empty if none
//!   {uuid}      -> a fresh random UUID v4
//!   {cursor}    -> not text: marks where the caret should land after insertion.
//!                  Emits nothing; the injector moves the caret back to it. Only
//!                  the FIRST {cursor} is honoured; later ones expand to nothing.
//!
//! Form variables (double square bracket) — prompt the user before insertion,
//! like filling a small form:
//!   [[key]]         -> value the user typed for `key`
//!   [[key:Label]]   -> same, but "Label" is shown as the field's caption
//!
//! Unknown single-brace tokens are left verbatim so a literal "{foo}" survives.
//! Escaping: "{{" and "}}" emit literal braces. "[[" is only special when it
//! closes with "]]" on the same body; a lone "[[" is emitted verbatim.

use std::collections::HashMap;

use time::{format_description::FormatItem, macros::format_description, Duration, OffsetDateTime};
use uuid::Uuid;

use crate::core::region::Region;

/// Context supplied by the caller at insertion time.
#[derive(Default)]
pub struct TokenContext {
    /// Clipboard text to substitute for {clipboard}. None -> empty string.
    pub clipboard: Option<String>,
    /// Values the user filled in for `[[key]]` form variables, keyed by `key`.
    /// A key with no entry expands to an empty string.
    pub variables: HashMap<String, String>,
}

/// A form variable discovered in a snippet body. `key` identifies it (used as
/// the map key at expansion time); `label` is what the fill-in form shows —
/// it defaults to the key when the body doesn't specify one after a colon.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Variable {
    pub key: String,
    pub label: String,
}

// US month-first vs. Brazilian day-first date order. Time order is the same
// everywhere we currently support, so it isn't region-dependent.
const DATE_FMT_US: &[FormatItem<'_>] = format_description!("[month]/[day]/[year]");
const DATE_FMT_BR: &[FormatItem<'_>] = format_description!("[day]/[month]/[year]");
const TIME_FMT: &[FormatItem<'_>] = format_description!("[hour]:[minute]");

fn date_fmt(region: Region) -> &'static [FormatItem<'static>] {
    match region {
        Region::US => DATE_FMT_US,
        Region::BR => DATE_FMT_BR,
    }
}

/// The result of expanding a body: the final `text` plus an optional caret
/// position. `cursor_from_end` is the number of characters (Unicode scalar
/// values) between where `{cursor}` appeared and the end of the text — the
/// injector sends that many Left-arrow presses after pasting so the caret lands
/// at the marker. `None` means no `{cursor}` was present (leave the caret at the
/// end, the normal behaviour).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Expanded {
    pub text: String,
    pub cursor_from_end: Option<usize>,
}

/// Expand all tokens in `body`, returning just the text. Kept for the many
/// callers that don't care about the `{cursor}` position. See [`expand_full`].
pub fn expand(body: &str, ctx: &TokenContext, region: Region) -> String {
    expand_full(body, ctx, region).text
}

/// Expand all tokens in `body` using `ctx`, formatting `{date}`/`{datetime}` in
/// the user's `region` order (US month-first by default). Never fails: a
/// formatting problem or unknown token falls back to leaving text as-is.
/// Returns the text plus where (if anywhere) `{cursor}` asked the caret to land.
pub fn expand_full(body: &str, ctx: &TokenContext, region: Region) -> Expanded {
    // Resolve time once so {date}, {time}, {datetime} are consistent.
    let now = local_now();
    let date = now
        .format(date_fmt(region))
        .unwrap_or_else(|_| String::new());
    let time = now
        .format(TIME_FMT)
        .unwrap_or_else(|_| String::new());
    let datetime = format!("{date} {time}");
    let clipboard = ctx.clipboard.clone().unwrap_or_default();

    // Character offset (not byte offset) of the first {cursor} marker, so the
    // injector can count Left-arrow presses. Only the first marker wins.
    let mut cursor_at: Option<usize> = None;
    let mut char_count: usize = 0;

    let mut out = String::with_capacity(body.len());
    let bytes = body.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let rest = &body[i..];
        // Escaped braces.
        if rest.starts_with("{{") {
            out.push('{');
            char_count += 1;
            i += 2;
            continue;
        }
        if rest.starts_with("}}") {
            out.push('}');
            char_count += 1;
            i += 2;
            continue;
        }
        // Form variable: [[key]] or [[key:Label]] -> the user-provided value.
        if rest.starts_with("[[") {
            if let Some(end) = rest.find("]]") {
                // Contents between the brackets; split key from optional label.
                let inner = &rest[2..end];
                let key = inner.split(':').next().unwrap_or("").trim();
                if !key.is_empty() {
                    if let Some(v) = ctx.variables.get(key) {
                        out.push_str(v);
                        char_count += v.chars().count();
                    }
                    // Missing value -> empty string (field left blank).
                    i += end + 2;
                    continue;
                }
                // "[[:foo]]" with no key: fall through and emit verbatim.
            }
            // No closing "]]" or empty key: emit "[[" literally and move on.
            out.push_str("[[");
            char_count += 2;
            i += 2;
            continue;
        }
        if bytes[i] == b'{' {
            // Find the closing brace.
            if let Some(close) = rest.find('}') {
                let token = &rest[1..close];
                // {cursor}: emits nothing but records the caret position. Only
                // the first one is honoured; a second collapses to nothing.
                if token == "cursor" {
                    if cursor_at.is_none() {
                        cursor_at = Some(char_count);
                    }
                    i += close + 1;
                    continue;
                }
                let replacement = match token {
                    "date" => Some(date.clone()),
                    "time" => Some(time.clone()),
                    "datetime" => Some(datetime.clone()),
                    "clipboard" => Some(clipboard.clone()),
                    "uuid" => Some(Uuid::new_v4().to_string()),
                    // {date±Nunit} date arithmetic (e.g. {date+7d}, {date-1m}).
                    _ => shifted_date(token, now, region),
                };
                if let Some(r) = replacement {
                    char_count += r.chars().count();
                    out.push_str(&r);
                    i += close + 1;
                    continue;
                }
                // Unknown token: emit the literal "{token}" untouched.
                out.push_str(&rest[..=close]);
                char_count += rest[..=close].chars().count();
                i += close + 1;
                continue;
            }
            // No closing brace: emit the rest literally.
            out.push_str(rest);
            char_count += rest.chars().count();
            break;
        }
        // Ordinary character — push the full char (handles multibyte UTF-8).
        let ch = rest.chars().next().unwrap();
        out.push(ch);
        char_count += 1;
        i += ch.len_utf8();
    }

    // Convert the marker's from-start offset into a from-end distance the
    // injector can walk with Left-arrows.
    let cursor_from_end = cursor_at.map(|at| char_count.saturating_sub(at));
    Expanded {
        text: out,
        cursor_from_end,
    }
}

/// Parse a `date±N<unit>` token (e.g. `date+7d`, `date-2w`, `date+1m`,
/// `date+1y`) and format the shifted date in `region` order. Returns `None` if
/// `token` isn't a well-formed date-offset, so the caller treats it as unknown.
/// Months/years are approximated (30 / 365 days) — good enough for the "date a
/// week from now"-style use this serves, without pulling in calendar math.
fn shifted_date(token: &str, now: OffsetDateTime, region: Region) -> Option<String> {
    let rest = token.strip_prefix("date")?;
    let (sign, rest) = match rest.strip_prefix('+') {
        Some(r) => (1i64, r),
        None => (-1i64, rest.strip_prefix('-')?),
    };
    // Split the digits from the trailing unit letter.
    let unit = rest.chars().last()?;
    let digits = &rest[..rest.len() - unit.len_utf8()];
    let n: i64 = digits.parse().ok()?;
    let days = match unit {
        'd' => n,
        'w' => n * 7,
        'm' => n * 30,
        'y' => n * 365,
        _ => return None,
    };
    let shifted = now.checked_add(Duration::days(sign * days))?;
    shifted.format(date_fmt(region)).ok()
}

/// True if a body contains any token — used to decide whether {clipboard}
/// requires reading the clipboard before injection.
pub fn has_tokens(body: &str) -> bool {
    // Cheap check; false positives (e.g. "{{") are harmless.
    body.contains('{')
}

/// Extract the ordered, de-duplicated list of `[[key]]` / `[[key:Label]]` form
/// variables in `body`. Order follows first appearance; a repeated key keeps the
/// first label seen. Returns an empty vec when the body has no form variables —
/// callers use that to skip the fill-in step and insert directly.
pub fn extract_variables(body: &str) -> Vec<Variable> {
    let mut vars: Vec<Variable> = Vec::new();
    let mut i = 0;
    let bytes = body.as_bytes();
    while i < bytes.len() {
        let rest = &body[i..];
        if rest.starts_with("[[") {
            if let Some(end) = rest.find("]]") {
                let inner = &rest[2..end];
                let mut parts = inner.splitn(2, ':');
                let key = parts.next().unwrap_or("").trim();
                if !key.is_empty() {
                    // Label after the colon, trimmed; fall back to the key.
                    let label = parts
                        .next()
                        .map(str::trim)
                        .filter(|l| !l.is_empty())
                        .unwrap_or(key)
                        .to_string();
                    if !vars.iter().any(|v| v.key == key) {
                        vars.push(Variable {
                            key: key.to_string(),
                            label,
                        });
                    }
                }
                i += end + 2;
                continue;
            }
        }
        // Advance one full char so multibyte bodies never split mid-codepoint.
        let ch = rest.chars().next().unwrap();
        i += ch.len_utf8();
    }
    vars
}

fn local_now() -> OffsetDateTime {
    // Prefer the machine's local offset; fall back to UTC if it can't be read
    // (can happen in some sandboxed/multithreaded contexts).
    OffsetDateTime::now_local().unwrap_or_else(|_| OffsetDateTime::now_utc())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx_with_vars(pairs: &[(&str, &str)]) -> TokenContext {
        TokenContext {
            variables: pairs
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
            ..Default::default()
        }
    }

    /// Region-agnostic expand for tests that don't care about date order.
    /// Uses the default (US) region.
    fn exp(body: &str, ctx: &TokenContext) -> String {
        expand(body, ctx, Region::US)
    }

    #[test]
    fn expands_clipboard_token() {
        let ctx = TokenContext {
            clipboard: Some("copiado".into()),
            ..Default::default()
        };
        assert_eq!(exp("x {clipboard} y", &ctx), "x copiado y");
    }

    #[test]
    fn expands_form_variable() {
        let ctx = ctx_with_vars(&[("nome_cliente", "Maria")]);
        assert_eq!(exp("Olá, [[nome_cliente]]!", &ctx), "Olá, Maria!");
    }

    #[test]
    fn labeled_variable_uses_key_for_lookup() {
        // The label after ':' is presentation only; the key drives the lookup.
        let ctx = ctx_with_vars(&[("num", "42")]);
        assert_eq!(exp("Pedido [[num:Número do pedido]]", &ctx), "Pedido 42");
    }

    #[test]
    fn missing_variable_becomes_empty() {
        let ctx = TokenContext::default();
        assert_eq!(exp("A[[x]]B", &ctx), "AB");
    }

    #[test]
    fn same_variable_used_twice() {
        let ctx = ctx_with_vars(&[("n", "Ana")]);
        assert_eq!(exp("[[n]] & [[n]]", &ctx), "Ana & Ana");
    }

    #[test]
    fn lone_open_bracket_is_verbatim() {
        let ctx = TokenContext::default();
        assert_eq!(exp("a [[ b", &ctx), "a [[ b");
        assert_eq!(exp("array[[i]] index", &ctx), "array index");
    }

    #[test]
    fn extract_variables_ordered_and_deduped() {
        let vars = extract_variables("[[b]] [[a:Rótulo A]] [[b]] [[c]]");
        let keys: Vec<_> = vars.iter().map(|v| v.key.as_str()).collect();
        assert_eq!(keys, ["b", "a", "c"]);
        // First appearance of "a" carries the label; "b"/"c" default to the key.
        assert_eq!(vars[1].label, "Rótulo A");
        assert_eq!(vars[0].label, "b");
    }

    #[test]
    fn extract_variables_none() {
        assert!(extract_variables("Olá {date}, sem variáveis").is_empty());
    }

    #[test]
    fn missing_clipboard_becomes_empty() {
        let ctx = TokenContext::default();
        assert_eq!(exp("a{clipboard}b", &ctx), "ab");
    }

    #[test]
    fn variables_and_tokens_coexist() {
        let ctx = ctx_with_vars(&[("nome", "João")]);
        let out = exp("Olá [[nome]], hoje é {date}.", &ctx);
        assert!(out.starts_with("Olá João, hoje é "));
        assert!(out.contains('/')); // {date} still expanded
    }

    #[test]
    fn unknown_token_left_verbatim() {
        let ctx = TokenContext::default();
        assert_eq!(exp("hello {foo} world", &ctx), "hello {foo} world");
    }

    #[test]
    fn escaped_braces() {
        let ctx = TokenContext::default();
        assert_eq!(exp("{{date}}", &ctx), "{date}");
    }

    #[test]
    fn date_and_time_are_nonempty_and_formatted() {
        let ctx = TokenContext::default();
        let out = exp("{date} {time}", &ctx);
        // mm/dd/yyyy hh:mm — 16 chars. Just assert shape, not the value.
        assert!(out.contains('/'));
        assert!(out.contains(':'));
    }

    #[test]
    fn date_order_follows_region() {
        // Same instant, different region -> different {date} order. We can't fix
        // the clock, so compare the two orderings against each other: on any day
        // where month != day they differ, and on the (rare) day they'd match the
        // formats are still each individually valid. Assert the *shape* per region.
        let ctx = TokenContext::default();
        let us = expand("{date}", &ctx, Region::US);
        let br = expand("{date}", &ctx, Region::BR);
        // Both are dd/mm/yyyy-shaped strings.
        for s in [&us, &br] {
            let parts: Vec<&str> = s.split('/').collect();
            assert_eq!(parts.len(), 3, "date has three /-separated parts");
        }
        // The year (last part) is identical; only the first two parts may swap.
        let up: Vec<&str> = us.split('/').collect();
        let bp: Vec<&str> = br.split('/').collect();
        assert_eq!(up[2], bp[2], "year matches across regions");
        // US month-first == BR day-first with the first two fields swapped.
        assert_eq!(up[0], bp[1], "US month == BR's second field");
        assert_eq!(up[1], bp[0], "US day == BR's first field");
    }

    #[test]
    fn preserves_unicode_body() {
        let ctx = TokenContext::default();
        assert_eq!(exp("Olá, ção {foo}", &ctx), "Olá, ção {foo}");
    }

    #[test]
    fn uuid_token_expands_to_a_uuid() {
        let ctx = TokenContext::default();
        let out = exp("id={uuid}", &ctx);
        let uuid_part = out.strip_prefix("id=").unwrap();
        // A v4 UUID is 36 chars with hyphens in the canonical positions.
        assert_eq!(uuid_part.len(), 36);
        assert_eq!(uuid_part.matches('-').count(), 4);
        // Two expansions differ (random).
        assert_ne!(exp("{uuid}", &ctx), exp("{uuid}", &ctx));
    }

    #[test]
    fn cursor_token_emits_nothing_and_reports_offset() {
        let ctx = TokenContext::default();
        let e = expand_full("Dear {cursor},", &ctx, Region::US);
        assert_eq!(e.text, "Dear ,");
        // Caret sits before the comma: 1 char from the end.
        assert_eq!(e.cursor_from_end, Some(1));
    }

    #[test]
    fn cursor_absent_reports_none() {
        let ctx = TokenContext::default();
        assert_eq!(expand_full("no marker", &ctx, Region::US).cursor_from_end, None);
    }

    #[test]
    fn only_first_cursor_is_honoured() {
        let ctx = TokenContext::default();
        let e = expand_full("a{cursor}b{cursor}c", &ctx, Region::US);
        assert_eq!(e.text, "abc");
        // First marker is after "a" -> 2 chars ("bc") from the end.
        assert_eq!(e.cursor_from_end, Some(2));
    }

    #[test]
    fn cursor_offset_counts_unicode_scalars() {
        let ctx = TokenContext::default();
        // "ção" after the marker is 3 scalar values, not bytes.
        let e = expand_full("x{cursor}ção", &ctx, Region::US);
        assert_eq!(e.cursor_from_end, Some(3));
    }

    #[test]
    fn date_offset_days_shape() {
        let ctx = TokenContext::default();
        // Just assert it expands to a date-shaped value, not the literal token.
        let out = exp("{date+7d}", &ctx);
        assert_eq!(out.split('/').count(), 3, "shifted date is dd/mm/yyyy-shaped");
        assert!(!out.contains("date"));
    }

    #[test]
    fn date_offset_negative_and_units() {
        let ctx = TokenContext::default();
        for tok in ["{date-3d}", "{date+2w}", "{date-1m}", "{date+1y}"] {
            let out = exp(tok, &ctx);
            assert_eq!(out.split('/').count(), 3, "{tok} -> date-shaped");
        }
    }

    #[test]
    fn malformed_date_offset_left_verbatim() {
        let ctx = TokenContext::default();
        // Not a valid offset: unknown unit, missing number, bare "date".
        assert_eq!(exp("{date+7z}", &ctx), "{date+7z}");
        assert_eq!(exp("{date+d}", &ctx), "{date+d}");
    }
}
