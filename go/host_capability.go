package main

// host_capability.go — Helpers for calling LLMProf host capabilities from a Go WASM plugin.
//
// The Extism host exposes a single imported function `call_host_capability` that
// accepts a JSON envelope and returns a JSON string. This file provides:
//
//   - RequestCapability       : low-level string wrapper
//   - RequestCapabilityJSON   : convenience wrapper that parses the JSON response
//   - AddMessageActivity      : typed wrapper for the `add_message_activity` capability
//   - RequestSystemModel      : typed wrapper for the `request_system_model` capability
//   - RequestUserI18n         : typed wrapper for the `request_user_i18n` capability
//
// Usage:
//
//   result, err := RequestSystemModel("Summarize this text", schemaMap)
//   if err != nil { ... }

import (
	"encoding/json"
	"fmt"

	"github.com/extism/go-pdk"
)

// callHostCapability is the raw WASM import binding to the Extism host function.
// The host reads a JSON envelope from the given memory offset and writes a
// JSON response into a new memory block, returning its offset.
//
//go:wasmimport extism:host/user call_host_capability
func callHostCapability(ptr uint64) uint64

// RequestCapability sends a capability request to the host and returns the
// raw JSON string response.
//
// The envelope sent to the host has the shape:
//
//	{"capability": "<capabilityName>", "input": <inputObj>}
//
// inputObj may be any JSON-serialisable value (map, string, nil, etc.).
func RequestCapability(capabilityName string, inputObj any) string {
	payload := map[string]any{
		"capability": capabilityName,
		"input":      inputObj,
	}

	reqBytes, err := json.Marshal(payload)
	if err != nil {
		// json.Marshal on a map[string]any should only fail in pathological cases,
		// but guard against it so we never panic inside WASM.
		pdk.SetError(fmt.Errorf("failed to marshal capability request: %w", err))
		return ""
	}

	// Allocate the request bytes in WASM shared memory and pass the offset
	// to the host function.
	memIn := pdk.AllocateBytes(reqBytes)
	memOutOffset := callHostCapability(memIn.Offset())

	// Look up the response memory block by offset and read its contents.
	memOut := pdk.FindMemory(memOutOffset)
	return string(memOut.ReadBytes())
}

// RequestCapabilityJSON is a convenience wrapper around RequestCapability that
// parses the JSON string response into a map[string]any.
//
// Returns an error if the host response is empty or not valid JSON.
func RequestCapabilityJSON(capabilityName string, inputObj any) (map[string]any, error) {
	raw := RequestCapability(capabilityName, inputObj)
	if raw == "" {
		return nil, fmt.Errorf("empty response from host for capability %q", capabilityName)
	}

	var result map[string]any
	if err := json.Unmarshal([]byte(raw), &result); err != nil {
		return nil, fmt.Errorf("failed to parse host response for capability %q: %w", capabilityName, err)
	}

	return result, nil
}

// AddMessageActivity calls the `add_message_activity` host capability.
//
// messageActivityKey identifies the message this activity belongs to (provided
// by the host in the tool input under the key "messageActivityKey").
//
// activity is the activity payload, e.g.:
//
//	{
//	    "type": "tool_call",
//	    "label": "Searching documents",
//	    "status": "completed"
//	}
func AddMessageActivity(messageActivityKey string, activity map[string]any) (map[string]any, error) {
	input := map[string]any{
		"messageActivityKey": messageActivityKey,
		"activity":           activity,
	}
	return RequestCapabilityJSON("add_message_activity", input)
}

// RequestSystemModel calls the `request_system_model` host capability to ask
// the system-configured LLM for a structured response.
//
// request is the natural-language prompt to send to the model.
//
// schema describes the expected JSON shape of the response, e.g.:
//
//	{
//	    "type": "object",
//	    "properties": {
//	        "summary": {"type": "string"}
//	    }
//	}
func RequestSystemModel(request string, schema map[string]any) (map[string]any, error) {
	input := map[string]any{
		"request": request,
		"schema":  schema,
	}
	return RequestCapabilityJSON("request_system_model", input)
}

// RequestUserI18n calls the `request_user_i18n` host capability to retrieve
// internationalisation strings for the current user's locale.
//
// The host returns a map of translation keys to localised values.
func RequestUserI18n() (map[string]any, error) {
	return RequestCapabilityJSON("request_user_i18n", nil)
}