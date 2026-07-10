package main

// main.go — "tool-with-host-capability" example plugin (Go).
//
// This example demonstrates a tool capability that:
//  1. In "define" mode:  returns a tool schema (name, description, parameters).
//  2. In "execute" mode: calls the `request_system_model` host capability to
//     get an LLM summary, saves the result to /storage/results.json using the
//     native WASI filesystem, and sends an `add_message_activity` card to
//     report progress.
//
// The host invokes the exported `summarize` function with a JSON payload that
// includes a "mode" field ("define" or "execute").

import (
	"encoding/json"
	"errors"
	"fmt"
	"os"
	"path/filepath"
	"strings"

	"github.com/extism/go-pdk"
)

// callHostCapabilityRaw is the raw WASM import binding to the Extism host
// function. The host reads a JSON envelope from the given memory offset and
// writes a JSON response into a new memory block, returning its offset.
//
//go:wasmimport extism:host/user call_host_capability
func callHostCapabilityRaw(ptr uint64) uint64

// logHost sends a log entry to the host's `log` host capability.
// If the host does not support the `log` capability the call is silently
// ignored — logging must never break plugin execution.
func logHost(level, message string, context map[string]any) {
	input := map[string]any{
		"level":   level,
		"message": message,
	}
	if context != nil {
		input["context"] = context
	}
	// Best-effort: ignore the response and any errors.
	_ = requestCapability("log", input)
}

// logDebug is a convenience wrapper for logHost at debug level.
func logDebug(message string) {
	logHost("debug", message, nil)
}

// logInfo is a convenience wrapper for logHost at info level.
func logInfo(message string) {
	logHost("info", message, nil)
}

// logError is a convenience wrapper for logHost at error level.
func logError(message string) {
	logHost("error", message, nil)
}

// requestCapability sends a capability request to the host and returns the
// raw JSON string response.
//
// The envelope sent to the host has the shape:
//
//	{"capability": "<capabilityName>", "input": <inputObj>}
//
// inputObj may be any JSON-serialisable value (map, string, nil, etc.).
func requestCapability(capabilityName string, inputObj any) string {
	payload := map[string]any{
		"capability": capabilityName,
		"input":      inputObj,
	}

	reqBytes, _ := json.Marshal(payload)
	memIn := pdk.AllocateBytes(reqBytes)
	memOutOffset := callHostCapabilityRaw(memIn.Offset())
	memOut := pdk.FindMemory(memOutOffset)
	return string(memOut.ReadBytes())
}

// Summarize is the single WASM-exported entry point invoked by the host.
//
// Return value convention:
//   0 = success (output written via pdk.OutputString)
//   1 = error   (error message set via pdk.SetError)
//
//go:wasmexport summarize
func Summarize() int32 {
	logInfo("summarize entry point called")

	// --- 1. Read input from the host ---------------------------------------
	inputBytes := pdk.Input()
	logDebug(fmt.Sprintf("input size: %d bytes", len(inputBytes)))

	var inputData map[string]any
	if err := json.Unmarshal(inputBytes, &inputData); err != nil {
		logError(fmt.Sprintf("failed to parse input JSON: %v", err))
		pdk.SetError(errors.New("invalid JSON input from host"))
		return 1
	}

	// Log the raw input keys for debugging (without values to avoid leaking secrets)
	inputKeys := make([]string, 0, len(inputData))
	for k := range inputData {
		inputKeys = append(inputKeys, k)
	}
	logDebug(fmt.Sprintf("input keys: [%s]", strings.Join(inputKeys, ", ")))

	// --- 2. Dispatch by mode -----------------------------------------------
	mode, _ := inputData["mode"].(string)
	logInfo(fmt.Sprintf("dispatching mode: %q", mode))

	switch mode {
	case "define":
		return handleDefine()
	case "execute":
		return handleExecute(inputData)
	default:
		logError(fmt.Sprintf("unknown mode: %q (expected \"define\" or \"execute\")", mode))
		pdk.SetError(fmt.Errorf("unknown mode: %q (expected \"define\" or \"execute\")", mode))
		return 1
	}
}

// handleDefine returns the tool schema that the LLM uses for function calling.
func handleDefine() int32 {
	logInfo("handleDefine: building tool schema")
	definition := map[string]any{
		"mode":        "define",
		"name":        "summarize",
		"description": "Summarise a block of text using the system LLM model and save the result to storage.",
		"parameters": map[string]any{
			"type": "object",
			"properties": map[string]any{
				"query": map[string]any{
					"type":        "string",
					"description": "The text to summarise.",
				},
				"messageActivityKey": map[string]any{
					"type":             "string",
					"description":       "Opaque key injected by the host to append message activities.",
					"x-system-provided": true,
				},
			},
			"required": []string{"query"},
		},
	}

	outputJSON, err := json.Marshal(definition)
	if err != nil {
		pdk.SetError(fmt.Errorf("failed to marshal tool definition: %w", err))
		return 1
	}
	pdk.OutputString(string(outputJSON))
	return 0
}

// handleExecute runs the tool logic:
//  1. Calls request_system_model with the query.
//  2. Saves the model response to /storage/results.json.
//  3. Reports progress via add_message_activity.
//  4. Returns the tool execute response.
func handleExecute(inputData map[string]any) int32 {
	logInfo("handleExecute: starting")

	// Extract the tool arguments and the message activity key.
	args, _ := inputData["arguments"].(map[string]any)
	if args == nil {
		logError("handleExecute: arguments is nil or not a map")
		pdk.SetError(errors.New("arguments is required and must be an object"))
		return 1
	}

	// Log argument keys (not values — may contain system-provided data)
	argKeys := make([]string, 0, len(args))
	for k := range args {
		argKeys = append(argKeys, k)
	}
	logDebug(fmt.Sprintf("handleExecute: argument keys: [%s]", strings.Join(argKeys, ", ")))

	query, _ := args["query"].(string)
	logDebug(fmt.Sprintf("handleExecute: query length: %d", len(query)))

	messageActivityKey, _ := args["messageActivityKey"].(string)
	if messageActivityKey == "" {
		logInfo("handleExecute: messageActivityKey is empty or missing from arguments")
	} else {
		logDebug(fmt.Sprintf("handleExecute: messageActivityKey present (length: %d)", len(messageActivityKey)))
	}

	// --- 1. Request a summary from the system LLM model --------------------
	logInfo("handleExecute: calling request_system_model")
	modelInput := map[string]any{
		"request": fmt.Sprintf("Summarise the following: %s", query),
		"schema": map[string]any{
			"type": "object",
			"properties": map[string]any{
				"summary": map[string]any{"type": "string"},
			},
		},
	}
	modelResponseRaw := requestCapability("request_system_model", modelInput)
	logDebug(fmt.Sprintf("handleExecute: model response size: %d bytes", len(modelResponseRaw)))
	logDebug(fmt.Sprintf("handleExecute: model response (first 500 chars): %.500s", modelResponseRaw))

	var modelResponse map[string]any
	if err := json.Unmarshal([]byte(modelResponseRaw), &modelResponse); err != nil {
		logError(fmt.Sprintf("handleExecute: failed to parse model response: %v", err))
		pdk.SetError(fmt.Errorf("failed to parse model response: %w", err))
		return 1
	}
	logInfo("handleExecute: model response parsed successfully")

	// --- 2. Save the result to /storage/results.json -----------------------
	// Go WASM plugins run under WASI and have native filesystem access.
	logInfo("handleExecute: writing /storage/results.json")
	if err := os.MkdirAll("/storage", 0o755); err != nil {
		logError(fmt.Sprintf("handleExecute: failed to create /storage directory: %v", err))
		pdk.SetError(fmt.Errorf("failed to create /storage directory: %w", err))
		return 1
	}
	storagePath := filepath.Join("/storage", "results.json")
	if err := os.WriteFile(storagePath, []byte(modelResponseRaw), 0o644); err != nil {
		logError(fmt.Sprintf("handleExecute: failed to write results.json: %v", err))
		pdk.SetError(fmt.Errorf("failed to write results.json: %w", err))
		return 1
	}
	logInfo("handleExecute: results.json written successfully")

	// --- 3. Report progress via add_message_activity -----------------------
	summary := ""
	if result, ok := modelResponse["result"].(map[string]any); ok {
		if s, ok := result["summary"].(string); ok {
			summary = s
		}
	}
	logDebug(fmt.Sprintf("handleExecute: extracted summary length: %d", len(summary)))

	if messageActivityKey != "" {
		logInfo("handleExecute: calling add_message_activity")
		activityInput := map[string]any{
			"messageActivityKey": messageActivityKey,
			"activity": map[string]any{
				"title":   "Summary Generated",
				"content": summary,
				"origin":  "tool-with-host-capability",
				"icon":    "FileText",
			},
		}
		activityResponse := requestCapability("add_message_activity", activityInput)
		logDebug(fmt.Sprintf("handleExecute: add_message_activity response: %s", activityResponse))
	} else {
		logInfo("handleExecute: skipping add_message_activity (messageActivityKey not injected)")
	}

	// --- 4. Return the tool execute response -------------------------------
	logInfo("handleExecute: building final result")
	result := map[string]any{
		"mode": "execute",
		"result": map[string]any{
			"content": summary,
		},
	}

	outputJSON, err := json.Marshal(result)
	if err != nil {
		logError(fmt.Sprintf("handleExecute: failed to marshal result: %v", err))
		pdk.SetError(fmt.Errorf("failed to marshal result: %w", err))
		return 1
	}
	logInfo(fmt.Sprintf("handleExecute: returning success (output size: %d bytes)", len(outputJSON)))
	pdk.OutputString(string(outputJSON))
	return 0
}

// main is required by the Go compiler for a c-shared build but is never
// called in the WASM environment — execution starts at the exported
// functions.
func main() {}