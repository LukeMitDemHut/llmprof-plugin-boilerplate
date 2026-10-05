package main

// main.go — "student-support-strategy" example plugin (Go).
//
// Demonstrates a `student_support` capability: the host asks a plugin in a
// chat for a learning-mode strategy and shows the returned entry in the
// student-support dropdown.
//
// INPUT CONTRACT
//
// The input is a single envelope key — the requested locale:
//
//	{"input": {"locale": "en"}, "plugin_config": {...}}
//
// That is all you get: run-all discovery and prompt resolution both use the
// same envelope.
//
// OUTPUT CONTRACT (strictly validated, additionalProperties:false)
//
// Return EXACTLY these four fields, nothing else:
//
//	name         required   ≤ 40 chars    Human-readable, localized. NOT
//	                                     your capability id — this is what
//	                                     the dropdown displays.
//	description  required   ≤ 100 chars   Short localized strategy
//	                                     description.
//	icon         required   IconType      Enum case name, e.g.
//	                        case name     "ChalkboardTeacher". Invalid or
//	                                     missing values fall back to the
//	                                     default icon.
//	prompt       required   ≤ 10000 chars The strategy's system prompt,
//	                                     injected verbatim into the LLM
//	                                     system prompt — keep it a prompt,
//	                                     not a strategy description.
//
// Do NOT return extra keys (title, mode, locale, success, ...) — the host
// schema forbids additional properties.
//
// HONOR THE LOCALE: return at minimum `name`/`description` localized per the
// requested locale; a de-only plugin should still fall back to English keys
// rather than echoing the locale back unchanged.

import (
	"encoding/json"
	"fmt"

	"github.com/extism/go-pdk"
)

type localizedStrings struct {
	name        string
	description string
}

var stringsByLocale = map[string]localizedStrings{
	"en": {
		name:        "Learning Style Coach",
		description: "Adapts explanations to visual and verbal learning styles.",
	},
	"de": {
		name:        "Lernstil-Coach",
		description: "Passt Erklärungen an visuelle und verbale Lernstile an.",
	},
}

var promptsByLocale = map[string]string{
	"en": "You are a learning-style coach. Determine from the student's question " +
		"whether they respond better to visual or verbal explanations. Prefer " +
		"diagrams, analogies, and worked examples. Keep answers focused and " +
		"ask one follow-up question at a time to refine the diagnosis.",
	"de": "Du bist ein Lernstil-Coach. Erkenne aus der Frage der Schülerin oder " +
		"des Schülers, ob sie besser auf visuelle oder verbale Erklärungen " +
		"anspringt. Bevorzuge Diagramme, Analogien und durchgerechnete " +
		"Beispiele. Halte die Antworten fokussiert und stelle jeweils eine " +
		"Nachfrage, um die Diagnose zu präzisieren.",
}

// StudentSupportStrategy is the WASM-exported entry point invoked by the
// host for the student_support capability.
//
// It always returns 0: even on internal errors the plugin emits a valid
// English-fallback strategy, because a malformed or extra-keyed response
// would fail the host's strict schema instead of degrading gracefully.
//
//go:wasmexport studentSupportStrategy
func StudentSupportStrategy() int32 {
	var envelope struct {
		Input struct {
			Locale string `json:"locale"`
		} `json:"input"`
	}

	if err := json.Unmarshal(pdk.Input(), &envelope); err != nil {
		logFallbackError(fmt.Sprintf("failed to parse input: %v", err))
	}

	locale := envelope.Input.Locale
	strs, ok := stringsByLocale[locale]
	if !ok {
		strs = stringsByLocale["en"]
	}
	prompt, ok := promptsByLocale[locale]
	if !ok {
		prompt = promptsByLocale["en"]
	}

	strategy := map[string]any{
		"name":        strs.name,
		"description": strs.description,
		"icon":        "ChalkboardTeacher",
		"prompt":      prompt,
	}

	outputJSON, err := json.Marshal(strategy)
	if err != nil {
		logFallbackError(fmt.Sprintf("failed to marshal strategy: %v", err))
		return 1
	}

	pdk.OutputString(string(outputJSON))
	return 0
}

// logFallbackError is a stub: this plugin declares no host_capabilities in
// its manifest, so there is nothing to call — replace with host logging if
// you declare the `log` capability.
func logFallbackError(message string) {
	_ = message
}

// main is required by the Go compiler for a c-shared build but is never
// called in the WASM environment — execution starts at the exported
// functions.
func main() {}