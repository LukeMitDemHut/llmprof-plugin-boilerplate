/**
 * index.js — "student-support-strategy" example plugin (JavaScript).
 *
 * Demonstrates a `student_support` capability: the host asks a plugin in a
 * chat for a learning-mode strategy and shows the returned entry in the
 * student-support dropdown.
 *
 * INPUT CONTRACT
 *
 * The input is a single envelope key — the requested locale:
 *
 *   {"input": {"locale": "en"}, "plugin_config": {...}}
 *
 * That is all you get: run-all discovery and prompt resolution both use the
 * same envelope.
 *
 * OUTPUT CONTRACT (strictly validated, additionalProperties:false)
 *
 * Return EXACTLY these four fields, nothing else:
 *
 *   name         required   ≤ 40 chars    Human-readable, localized. NOT
 *                                        your capability id — this is what
 *                                        the dropdown displays.
 *   description  required   ≤ 100 chars   Short localized strategy
 *                                        description.
 *   icon         required   IconType      Enum case name, e.g.
 *                           case name     "ChalkboardTeacher". Invalid or
 *                                        missing values fall back to the
 *                                        default icon.
 *   prompt       required   ≤ 10000 chars The strategy's system prompt,
 *                                        injected verbatim into the LLM
 *                                        system prompt — keep it a prompt,
 *                                        not a strategy description.
 *
 * Do NOT return extra keys (title, mode, locale, success, ...) — the host
 * schema forbids additional properties.
 *
 * HONOR THE LOCALE: return at minimum `name`/`description` localized per the
 * requested locale; a de-only plugin should still fall back to English keys
 * rather than echoing the locale back unchanged.
 */

/** Localized strings, keyed by locale with an English fallback. */
var STRINGS = {
  en: {
    name: "Learning Style Coach",
    description: "Adapts explanations to visual and verbal learning styles.",
  },
  de: {
    name: "Lernstil-Coach",
    description: "Passt Erklärungen an visuelle und verbale Lernstile an.",
  },
};

/** Prompts, keyed by locale with an English fallback. */
var PROMPTS = {
  en:
    "You are a learning-style coach. Determine from the student's question " +
    "whether they respond better to visual or verbal explanations. Prefer " +
    "diagrams, analogies, and worked examples. Keep answers focused and " +
    "ask one follow-up question at a time to refine the diagnosis.",
  de:
    "Du bist ein Lernstil-Coach. Erkenne aus der Frage der Schülerin oder " +
    "des Schülers, ob sie besser auf visuelle oder verbale Erklärungen " +
    "anspringt. Bevorzuge Diagramme, Analogien und durchgerechnete " +
    "Beispiele. Halte die Antworten fokussiert und stelle jeweils eine " +
    "Nachfrage, um die Diagnose zu präzisieren.",
};

/**
 * Build the strategy response for the requested locale.
 *
 * @param {string} locale - Requested locale, e.g. "en" or "de".
 * @returns {object}       - Exactly {name, description, icon, prompt}.
 */
function buildStrategy(locale) {
  var strings = STRINGS[locale] || STRINGS.en;
  var prompt = PROMPTS[locale] || PROMPTS.en;

  return {
    name: strings.name,
    description: strings.description,
    icon: "ChalkboardTeacher",
    prompt: prompt,
  };
}

/**
 * Main entry point called by the Extism host.
 *
 * @returns {number} 0 on success (convention for Extism PDK entry points).
 */
function studentSupportStrategy() {
  try {
    var inputStr = Host.inputString();
    var envelope = JSON.parse(inputStr);
    var input = envelope.input || {};
    var locale = typeof input.locale === "string" ? input.locale : "en";

    var strategy = buildStrategy(locale);

    Host.outputString(JSON.stringify(strategy));
    return 0;
  } catch (err) {
    // Last-resort English fallback — an error response with extra keys
    // would fail the strict schema, so always emit a valid strategy shape.
    var fallback = buildStrategy("en");
    Host.outputString(JSON.stringify(fallback));
    return 0;
  }
}

module.exports = { studentSupportStrategy };