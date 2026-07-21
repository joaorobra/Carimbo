//! OS-agnostic abbreviation matcher.
//!
//! Consumes a stream of typed characters (already translated from key events by
//! the platform layer) plus edit signals (backspace / reset), maintains a
//! rolling buffer of recent input, and reports when the buffer's suffix matches a
//! known trigger. It knows nothing about Win32 — it is unit-testable in
//! isolation, which matters because trigger matching is easy to get subtly wrong.

use std::collections::HashMap;

/// Maximum characters kept in the rolling buffer. Longer than any realistic
/// trigger; keeps memory trivial and matching cheap.
const MAX_BUFFER: usize = 64;

/// A resolved expansion: how many characters to delete (the trigger length in
/// characters) and the replacement text to inject.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Expansion {
    pub delete_chars: usize,
    pub replacement: String,
    /// The trigger that fired, lowercased (the key it matched on). Used to look
    /// up confusably-similar sibling triggers for the disambiguation picker.
    pub matched_trigger: String,
}

/// Case-insensitive Levenshtein edit distance between two strings, with an early
/// exit once the distance is known to exceed `max` (returns `max + 1`). Used to
/// decide which triggers are "confusably similar" (differ by only 1–2 letters or
/// case) so we can offer a picker instead of silently choosing one.
pub fn similar_distance(a: &str, b: &str, max: usize) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    // Quick length-based lower bound.
    let (la, lb) = (a.len(), b.len());
    if la.abs_diff(lb) > max {
        return max + 1;
    }
    // Classic DP row. Small strings (triggers), so this is cheap.
    let mut prev: Vec<usize> = (0..=lb).collect();
    let mut curr = vec![0usize; lb + 1];
    for i in 1..=la {
        curr[0] = i;
        let mut row_min = curr[0];
        for j in 1..=lb {
            let cost = if a[i - 1].eq_ignore_ascii_case(&b[j - 1]) {
                // eq_ignore_ascii_case only folds ASCII; fall back to a full
                // lowercase compare for non-ASCII (e.g. accented) trigger chars.
                0
            } else if a[i - 1].to_lowercase().eq(b[j - 1].to_lowercase()) {
                0
            } else {
                1
            };
            curr[j] = (prev[j] + 1).min(curr[j - 1] + 1).min(prev[j - 1] + cost);
            row_min = row_min.min(curr[j]);
        }
        // If the whole row already exceeds max, no later row can improve it.
        if row_min > max {
            return max + 1;
        }
        std::mem::swap(&mut prev, &mut curr);
    }
    prev[lb]
}

/// Matcher over a set of triggers. Triggers are matched as a suffix of the
/// buffer, so `;cpf` fires the moment the user finishes typing it regardless of
/// what came before.
///
/// Matching is **case-insensitive**: the DB enforces NOCASE uniqueness on
/// triggers (a user can't have both `;cpf` and `;CPF`), and a person typing
/// `;cpf` doesn't distinguish it from a snippet saved as `;CPF`. So we key the
/// lookup table on the lowercased trigger and lowercase the buffer suffix before
/// comparing. `delete_chars` is a character count, which is case-independent.
pub struct Matcher {
    buffer: String,
    /// lowercased trigger -> replacement. Lookups iterate candidate suffixes,
    /// lowercasing each before comparing so case never blocks a match.
    triggers: HashMap<String, String>,
    /// Length in characters of the longest trigger, to bound suffix checks.
    max_trigger_chars: usize,
}

impl Matcher {
    pub fn new() -> Self {
        Matcher {
            buffer: String::new(),
            triggers: HashMap::new(),
            max_trigger_chars: 0,
        }
    }

    /// Replace the full trigger set (called on startup and when snippets change).
    pub fn set_triggers<I, S>(&mut self, triggers: I)
    where
        I: IntoIterator<Item = (S, S)>,
        S: Into<String>,
    {
        self.triggers.clear();
        self.max_trigger_chars = 0;
        for (t, repl) in triggers {
            let t: String = t.into();
            if t.is_empty() {
                continue;
            }
            // Bound suffix checks by the original char length (== lowercased
            // char length for the scripts we support).
            let len = t.chars().count();
            if len > self.max_trigger_chars {
                self.max_trigger_chars = len;
            }
            // Key on the lowercased trigger so matching ignores case.
            self.triggers.insert(t.to_lowercase(), repl.into());
        }
    }

    pub fn clear_buffer(&mut self) {
        self.buffer.clear();
    }

    pub fn backspace(&mut self) {
        self.buffer.pop();
    }

    /// Feed one typed character. Returns `Some(Expansion)` if a trigger now
    /// matches the buffer suffix. On a match the buffer is cleared so the same
    /// trigger can't double-fire.
    pub fn push_char(&mut self, c: char) -> Option<Expansion> {
        // Keep the buffer bounded (char-aware, not byte-aware).
        self.buffer.push(c);
        while self.buffer.chars().count() > MAX_BUFFER {
            // Drop from the front by one char.
            let mut it = self.buffer.char_indices();
            if let Some((_, _)) = it.next() {
                let cut = it.next().map(|(i, _)| i).unwrap_or(self.buffer.len());
                self.buffer.drain(..cut);
            } else {
                break;
            }
        }

        if self.triggers.is_empty() {
            return None;
        }

        // Check suffixes from longest possible trigger down to length 1.
        // Collect the buffer's trailing chars once.
        let chars: Vec<char> = self.buffer.chars().collect();
        let max = self.max_trigger_chars.min(chars.len());
        for len in (1..=max).rev() {
            let start = chars.len() - len;
            // Lowercase the candidate suffix so matching is case-insensitive,
            // mirroring how triggers were stored (see `set_triggers`).
            let candidate: String = chars[start..].iter().collect::<String>().to_lowercase();
            if let Some(repl) = self.triggers.get(&candidate) {
                let expansion = Expansion {
                    delete_chars: len,
                    replacement: repl.clone(),
                    matched_trigger: candidate,
                };
                self.clear_buffer();
                return Some(expansion);
            }
        }
        None
    }

    /// All trigger keys within `max_dist` case-insensitive edits of `trigger`
    /// (inclusive of `trigger` itself if present). These are the "confusably
    /// similar" triggers — the ones a user might mean when they type something
    /// close. Returned lowercased (the map keys); the caller resolves display
    /// text and ranking from the DB.
    pub fn similar_triggers(&self, trigger: &str, max_dist: usize) -> Vec<String> {
        let needle = trigger.to_lowercase();
        self.triggers
            .keys()
            .filter(|k| similar_distance(&needle, k, max_dist) <= max_dist)
            .cloned()
            .collect()
    }
}

impl Default for Matcher {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn feed(m: &mut Matcher, s: &str) -> Option<Expansion> {
        let mut last = None;
        for c in s.chars() {
            last = m.push_char(c);
        }
        last
    }

    #[test]
    fn matches_simple_trigger() {
        let mut m = Matcher::new();
        m.set_triggers([(";cpf", "123.456.789-00")]);
        let e = feed(&mut m, ";cpf").unwrap();
        assert_eq!(e.delete_chars, 4);
        assert_eq!(e.replacement, "123.456.789-00");
    }

    #[test]
    fn matching_is_case_insensitive() {
        // Trigger saved as ";CPF"; the user types ";cpf" (and other casings).
        // All must fire — the DB enforces NOCASE uniqueness so these are "the
        // same" trigger from the user's point of view.
        let mut m = Matcher::new();
        m.set_triggers([(";CPF", "123.456.789-00")]);
        assert!(feed(&mut m, ";cpf").is_some(), "lowercase typed should match");
        assert!(feed(&mut m, ";CPF").is_some(), "exact case should match");
        assert!(feed(&mut m, ";Cpf").is_some(), "mixed case should match");
    }

    #[test]
    fn matches_only_at_suffix_after_other_text() {
        let mut m = Matcher::new();
        m.set_triggers([(";oi", "Olá")]);
        // Typing arbitrary text then the trigger still fires.
        assert!(feed(&mut m, "bom dia ").is_none());
        let e = feed(&mut m, ";oi").unwrap();
        assert_eq!(e.delete_chars, 3);
        assert_eq!(e.replacement, "Olá");
    }

    #[test]
    fn backspace_shortens_buffer_and_prevents_match() {
        let mut m = Matcher::new();
        m.set_triggers([(";oi", "Olá")]);
        feed(&mut m, ";o");
        m.backspace(); // buffer now ";"
        assert!(m.push_char('i').is_none()); // ";i" is not a trigger
    }

    #[test]
    fn reset_clears_partial_trigger() {
        let mut m = Matcher::new();
        m.set_triggers([(";oi", "Olá")]);
        feed(&mut m, ";o");
        m.clear_buffer();
        assert!(m.push_char('i').is_none());
    }

    #[test]
    fn longest_trigger_wins_on_overlap() {
        let mut m = Matcher::new();
        m.set_triggers([(";a", "short"), ("x;a", "long")]);
        // Buffer "x;a" — both ";a" and "x;a" are suffixes; longest should win.
        let e = feed(&mut m, "x;a").unwrap();
        assert_eq!(e.replacement, "long");
        assert_eq!(e.delete_chars, 3);
    }

    #[test]
    fn no_double_fire_after_match() {
        let mut m = Matcher::new();
        m.set_triggers([(";oi", "Olá")]);
        feed(&mut m, ";oi");
        // Buffer cleared; typing more doesn't immediately re-fire.
        assert!(m.push_char('!').is_none());
    }

    #[test]
    fn handles_unicode_trigger_chars() {
        let mut m = Matcher::new();
        m.set_triggers([("café", "coffee")]);
        let e = feed(&mut m, "café").unwrap();
        // "café" is 4 chars even though 'é' is multibyte.
        assert_eq!(e.delete_chars, 4);
    }

    #[test]
    fn expansion_reports_matched_trigger() {
        let mut m = Matcher::new();
        m.set_triggers([(";CPF", "123")]);
        let e = feed(&mut m, ";cpf").unwrap();
        // Always lowercased (the key it matched on).
        assert_eq!(e.matched_trigger, ";cpf");
    }

    #[test]
    fn similar_distance_basics() {
        assert_eq!(similar_distance(";cpf", ";cpf", 2), 0);
        assert_eq!(similar_distance(";cpf", ";CPF", 2), 0); // case-only
        assert_eq!(similar_distance(";cpf", ";cpfs", 2), 1); // one insert
        // ";cpf" -> ";cnpf" (insert n) -> ";cnpj" (sub f->j) = 2 edits.
        assert_eq!(similar_distance(";cpf", ";cnpj", 2), 2);
        // "abc" -> "xyz" is 3 edits, beyond max=2, so capped at max+1.
        assert_eq!(similar_distance("abc", "xyz", 2), 3);
        // Length gap alone exceeds max -> capped early.
        assert_eq!(similar_distance("a", "abcd", 2), 3);
    }

    #[test]
    fn similar_triggers_finds_confusable_set() {
        let mut m = Matcher::new();
        m.set_triggers([
            (";cpf", "a"),
            (";cpfs", "b"),
            (";cpj", "c"),
            (";endereco", "d"),
        ]);
        let mut got = m.similar_triggers(";cpf", 2);
        got.sort();
        // ;cpf (0), ;cpfs (1), ;cpj (1) are within 2; ;endereco is far.
        assert_eq!(got, vec![";cpf", ";cpfs", ";cpj"]);
    }
}
