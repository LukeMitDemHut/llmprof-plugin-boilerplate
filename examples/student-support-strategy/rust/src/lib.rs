//! "student-support-strategy" example plugin (Rust).
//!
//! Demonstrates a `student_support` capability: the host asks a plugin in a
//! chat for a learning-mode strategy and shows the returned entry in the
//! student-support dropdown.
//!
//! # Input contract
//!
//! The input is a single envelope key — the requested locale:
//!
//! ```json
//! { "input": {"locale": "en"}, "plugin_config": {} }
//! ```
//!
//! That is all you get: run-all discovery and prompt resolution both use the
//! same envelope.
//!
//! # Output contract (strictly validated, additionalProperties:false)
//!
//! Return EXACTLY these four fields, nothing else:
//!
//! | Field         | Required | Constraint        | Note                              |
//! |---------------|----------|-------------------|-----------------------------------|
//! | `name`        | yes      | ≤ 40 chars        | Human-readable, localized. NOT the capability id — the dropdown displays it. |
//! | `description` | yes      | ≤ 100 chars       | Short localized strategy description. |
//! | `icon`        | yes      | `IconType` enum   | e.g. `"ChalkboardTeacher"`. Invalid values fall back to the default icon. |
//! | `prompt`      | yes      | ≤ 10000 chars     | The strategy's system prompt, injected verbatim into the LLM system prompt. |
//!
//! Do NOT return extra keys (`title`, `mode`, `locale`, `success`, ...) —
//! the host schema forbids additional properties. Honor the locale: return
//! at minimum `name`/`description` localized per the requested locale.

use extism_pdk::*;
use serde_json::{json, Value};

/// Localized strings with an English fallback.
fn strings_for_locale(locale: &str) -> (String, String) {
    match locale {
        "de" => (
            "Lernstil-Coach".to_string(),
            "Passt Erklärungen an visuelle und verbale Lernstile an.".to_string(),
        ),
        // English fallback for unknown locales
        _ => (
            "Learning Style Coach".to_string(),
            "Adapts explanations to visual and verbal learning styles.".to_string(),
        ),
    }
}

/// Strategy prompts with an English fallback.
fn prompt_for_locale(locale: &str) -> String {
    match locale {
        "de" => "Du bist ein Lernstil-Coach. Erkenne aus der Frage der Schülerin oder \
                 des Schülers, ob sie besser auf visuelle oder verbale Erklärungen \
                 anspringt. Bevorzuge Diagramme, Analogien und durchgerechnete \
                 Beispiele. Halte die Antworten fokussiert und stelle jeweils eine \
                 Nachfrage, um die Diagnose zu präzisieren."
            .to_string(),
        _ => "You are a learning-style coach. Determine from the student's question \
              whether they respond better to visual or verbal explanations. Prefer \
              diagrams, analogies, and worked examples. Keep answers focused and \
              ask one follow-up question at a time to refine the diagnosis."
            .to_string(),
    }
}

/// Main plugin entry point, exported via the `#[plugin_fn]` macro.
///
/// It returns `Ok` even for unexpected input: the plugin emits a valid
/// English-fallback strategy, because a malformed or extra-keyed response
/// would fail the host's strict schema instead of degrading gracefully.
#[plugin_fn]
pub fn student_support_strategy(envelope: Json<Value>) -> FnResult<String> {
    let locale = envelope
        .0
        .get("input")
        .and_then(|i| i.get("locale"))
        .and_then(|l| l.as_str())
        .unwrap_or("en");

    let (name, description) = strings_for_locale(locale);
    let prompt = prompt_for_locale(locale);

    let strategy = json!({
        "name": name,
        "description": description,
        "icon": "ChalkboardTeacher",
        "prompt": prompt,
    });

    serde_json::to_string(&strategy)
        .map_err(|e| Error::msg(format!("Failed to serialize strategy: {e}")))
        .map_err(|e| WithReturnCode::new(e, 1))
}