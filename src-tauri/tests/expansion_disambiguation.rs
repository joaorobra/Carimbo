//! End-to-end (sans-GUI) test for the two snippet-detection fixes, reproducing
//! the exact scenario observed in a real user's database:
//!   - a snippet with trigger ";CPF"
//!   - a snippet with trigger ";Cpfs"
//!   - the user types ";cpf" (lowercase)
//!
//! Before the fix, the runtime matcher used a case-sensitive lookup, so ";cpf"
//! matched neither ";CPF" nor ";Cpfs" and nothing expanded. After the fix:
//!   1. Matching is case-insensitive → ";cpf" fires.
//!   2. ";CPF" and ";Cpfs" are confusably similar (edit distance 0 and 1 from
//!      what was typed), so the engine should offer BOTH in the radial picker,
//!      ranked by relevance (exact/closest first).
//!
//! This drives the real `Matcher` + `snippet_repo` against a real SQLite DB,
//! mirroring what `perform_expansion`/`try_show_radial` do on the worker thread.

use carimbo_lib::core::db::Db;
use carimbo_lib::core::expansion::{similar_distance, Matcher};
use carimbo_lib::core::models::NewSnippet;
use carimbo_lib::core::repo::snippet_repo;

fn new(name: &str, trigger: &str, body: &str) -> NewSnippet {
    NewSnippet {
        name: name.into(),
        trigger: Some(trigger.into()),
        body: body.into(),
        body_html: None,
        folder_id: None,
        is_favorite: false,
    }
}

fn feed(m: &mut Matcher, s: &str) -> Option<carimbo_lib::core::expansion::Expansion> {
    let mut last = None;
    for c in s.chars() {
        last = m.push_char(c);
    }
    last
}

#[test]
fn typing_lowercase_trigger_matches_and_offers_similar() {
    let db = Db::open_in_memory().expect("db");
    let conn = db.lock();

    // The user's real data.
    snippet_repo::create(&conn, new("CPF", ";CPF", "123.456.789-00")).unwrap();
    snippet_repo::create(&conn, new("CPF cônjuge", ";Cpfs", "987.654.321-11")).unwrap();
    // A far-away trigger that must NOT be offered.
    snippet_repo::create(&conn, new("Endereço", ";end", "Rua X")).unwrap();

    // Load triggers into the matcher exactly like the service does.
    let mut matcher = Matcher::new();
    matcher.set_triggers(snippet_repo::all_triggers(&conn).unwrap());

    // 1. Typing ";cpf" (lowercase) fires despite the stored casing.
    let exp = feed(&mut matcher, ";cpf").expect("lowercase ;cpf must expand");
    assert_eq!(exp.matched_trigger, ";cpf");
    assert_eq!(exp.delete_chars, 4);

    // 2. The engine gathers confusably-similar triggers for the radial.
    let similar = matcher.similar_triggers(&exp.matched_trigger, 2);
    assert!(
        similar.len() >= 2,
        "expected an ambiguous set (>=2), got {similar:?}"
    );
    assert!(similar.contains(&";cpf".to_string()));
    assert!(similar.contains(&";cpfs".to_string()));
    assert!(
        !similar.contains(&";end".to_string()),
        "far trigger must not be offered"
    );

    // 3. Resolve them to snippets and rank by relevance (distance, then name).
    let mut snippets = snippet_repo::by_triggers(&conn, &similar).unwrap();
    assert_eq!(snippets.len(), 2, "both CPF snippets resolved");
    snippets.sort_by_key(|s| {
        similar_distance(
            &exp.matched_trigger,
            &s.trigger.clone().unwrap_or_default().to_lowercase(),
            2,
        )
    });
    // The exact match (";CPF", distance 0) ranks first; ";Cpfs" (distance 1) next.
    assert_eq!(snippets[0].trigger.as_deref(), Some(";CPF"));
    assert_eq!(snippets[1].trigger.as_deref(), Some(";Cpfs"));
}

#[test]
fn unambiguous_trigger_does_not_trigger_radial() {
    // When only one trigger is anywhere near what was typed, there's nothing to
    // disambiguate — the direct-expansion path should be taken (similar set == 1).
    let db = Db::open_in_memory().expect("db");
    let conn = db.lock();
    snippet_repo::create(&conn, new("Saudação", ";oi", "Olá!")).unwrap();
    snippet_repo::create(&conn, new("Endereço", ";endereco", "Rua X")).unwrap();

    let mut matcher = Matcher::new();
    matcher.set_triggers(snippet_repo::all_triggers(&conn).unwrap());

    let exp = feed(&mut matcher, ";oi").expect("must expand");
    let similar = matcher.similar_triggers(&exp.matched_trigger, 2);
    assert_eq!(similar, vec![";oi".to_string()], "only itself is similar");
}
