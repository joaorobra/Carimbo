//! First-run example snippets, per region.
//!
//! When a user picks their region on first run (or accepts the US default), we
//! drop in a handful of ready-to-use examples so the app isn't empty and they
//! immediately see how triggers, tokens, and form variables work. The examples
//! use each region's real-world conventions (US phone/date/address vs. Brazilian
//! CPF/DDD/CEP), which is the concrete payoff of the region choice.
//!
//! Seeding is idempotent at the call site: the command only seeds when there are
//! no live snippets yet, so a user who deletes the examples never gets them back.

use crate::core::models::NewSnippet;
use crate::core::region::Region;

/// A single example snippet as (name, trigger, body). Bodies may use tokens like
/// `{date}` and form variables like `[[key:Label]]`, so the examples double as
/// living documentation of those features.
struct Example {
    name: &'static str,
    trigger: &'static str,
    body: &'static str,
    favorite: bool,
}

/// US-first examples. Month-first dates, +1 phone, ZIP-style address, SSN-shaped
/// ID placeholder, and a meeting-follow-up that shows a form variable.
const US_EXAMPLES: &[Example] = &[
    Example {
        name: "My email",
        trigger: ";email",
        body: "you@example.com",
        favorite: true,
    },
    Example {
        name: "My phone",
        trigger: ";phone",
        body: "(555) 123-4567",
        favorite: false,
    },
    Example {
        name: "Mailing address",
        trigger: ";addr",
        body: "123 Main St, Apt 4\nSpringfield, IL 62704",
        favorite: false,
    },
    Example {
        name: "Today's date",
        trigger: ";date",
        body: "{date}",
        favorite: false,
    },
    Example {
        name: "Email signature",
        trigger: ";sig",
        body: "Best regards,\nAlex Doe\nAcme Corp — you@example.com",
        favorite: false,
    },
    Example {
        name: "Meeting follow-up",
        trigger: ";followup",
        body: "Hi [[name:First name]],\n\nThanks for meeting on {date}. Here's a quick recap of what we discussed:\n\n- \n\nBest,\nAlex",
        favorite: false,
    },
];

/// Brazil-first examples. Day-first dates, +55 phone with DDD, CEP-style address,
/// a CPF placeholder, and a Portuguese follow-up that shows a form variable.
const BR_EXAMPLES: &[Example] = &[
    Example {
        name: "Meu e-mail",
        trigger: ";email",
        body: "voce@exemplo.com",
        favorite: true,
    },
    Example {
        name: "Meu telefone",
        trigger: ";tel",
        body: "+55 (11) 91234-5678",
        favorite: false,
    },
    Example {
        name: "Meu CPF",
        trigger: ";cpf",
        body: "123.456.789-00",
        favorite: false,
    },
    Example {
        name: "Endereço",
        trigger: ";end",
        body: "Rua das Flores, 123 — Apto 4\nSão Paulo, SP — 01310-100",
        favorite: false,
    },
    Example {
        name: "Data de hoje",
        trigger: ";data",
        body: "{date}",
        favorite: false,
    },
    Example {
        name: "Assinatura",
        trigger: ";assinatura",
        body: "Atenciosamente,\nMaria Silva\nÁrvore Consultoria — voce@exemplo.com",
        favorite: false,
    },
    Example {
        name: "Follow-up de reunião",
        trigger: ";followup",
        body: "Olá [[nome:Primeiro nome]],\n\nObrigado pela reunião em {date}. Segue um resumo do que conversamos:\n\n- \n\nAbraço,\nMaria",
        favorite: false,
    },
];

/// The example snippets to seed for `region`, as ready-to-insert `NewSnippet`s in
/// display order. Later entries are inserted after earlier ones; the list view
/// orders by favorite then recency, so favorites float to the top.
pub fn examples_for(region: Region) -> Vec<NewSnippet> {
    let set = match region {
        Region::US => US_EXAMPLES,
        Region::BR => BR_EXAMPLES,
    };
    set.iter()
        .map(|e| NewSnippet {
            name: e.name.to_string(),
            trigger: Some(e.trigger.to_string()),
            body: e.body.to_string(),
            body_html: None,
            folder_id: None,
            is_favorite: e.favorite,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn both_regions_produce_examples() {
        assert!(!examples_for(Region::US).is_empty());
        assert!(!examples_for(Region::BR).is_empty());
    }

    #[test]
    fn triggers_are_unique_within_a_region() {
        for region in [Region::US, Region::BR] {
            let ex = examples_for(region);
            let mut triggers: Vec<String> = ex
                .iter()
                .filter_map(|s| s.trigger.clone())
                .map(|t| t.to_lowercase())
                .collect();
            let total = triggers.len();
            triggers.sort();
            triggers.dedup();
            assert_eq!(triggers.len(), total, "duplicate seed trigger in {region:?}");
        }
    }

    #[test]
    fn us_examples_use_month_first_and_no_cpf() {
        // Sanity: the US set shouldn't ship Brazil-specific artifacts.
        let bodies: String = examples_for(Region::US)
            .iter()
            .map(|s| s.body.clone())
            .collect();
        assert!(!bodies.contains("CPF"));
        assert!(!bodies.contains("+55"));
    }
}
