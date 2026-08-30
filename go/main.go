package main

// main.go — Generic boilerplate template for a LLMProf Go WASM plugin.
//
// This template provides the host-capability helpers, storage helpers, and
// input/output patterns that every LLMProf Go plugin needs. It contains no
// actual functionality — replace the placeholder code with your own.
//
// Plugins may also declare lifecycle hooks (on_install, on_before_upgrade,
// on_after_upgrade, on_uninstall).
//
// See the examples/ directory for complete working plugins of each type.

import (
	"encoding/json"
	"fmt"
	"os"

	"github.com/extism/go-pdk"
)

// ---------------------------------------------------------------------------
// Host capability helpers
// ---------------------------------------------------------------------------

// callHostCapabilityRaw is the raw WASM import binding to the Extism host
// function. The host reads a JSON envelope from the given memory offset and
// writes a JSON response into a new memory block, returning its offset.
//
//go:wasmimport extism:host/user call_host_capability
func callHostCapabilityRaw(ptr uint64) uint64

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

// logHost sends a log entry to the host's `log` host capability.
// If the host does not support the `log` capability the call is silently
// ignored — logging must never break plugin execution.
func logHost(level, message string) {
	input := map[string]any{
		"level":   level,
		"message": message,
	}
	// Best-effort: ignore the response and any errors.
	_ = requestCapability("log", input)
}

// logDebug is a convenience wrapper for logHost at debug level.
func logDebug(message string) {
	logHost("debug", message)
}

// logInfo is a convenience wrapper for logHost at info level.
func logInfo(message string) {
	logHost("info", message)
}

// logError is a convenience wrapper for logHost at error level.
func logError(message string) {
	logHost("error", message)
}

// ---------------------------------------------------------------------------
// Input / plugin config helpers
// ---------------------------------------------------------------------------

// readHostEnvelope reads the raw host input and parses the
// {"input": {...}, "plugin_config": {...}} envelope once, returning both
// values. This avoids calling pdk.Input() multiple times.
func readHostEnvelope() (inputData map[string]any, pluginConfig map[string]any) {
	inputBytes := pdk.Input()
	var envelope struct {
		Input        map[string]any `json:"input"`
		PluginConfig map[string]any `json:"plugin_config"`
	}
	if err := json.Unmarshal(inputBytes, &envelope); err != nil {
		logError(fmt.Sprintf("readHostEnvelope: failed to parse input JSON: %v", err))
		return map[string]any{}, map[string]any{}
	}
	if envelope.Input == nil {
		envelope.Input = map[string]any{}
	}
	if envelope.PluginConfig == nil {
		envelope.PluginConfig = map[string]any{}
	}
	return envelope.Input, envelope.PluginConfig
}

// getInput reads the raw host input, unwraps the "input" key from the
// {"input": {...}, "plugin_config": {...}} envelope, and returns it as a
// map[string]any. Returns an empty map on parse failure.
//
// Note: this calls pdk.Input() internally. If you need both input and
// plugin_config, use readHostEnvelope() instead to avoid reading the input
// buffer twice.
func getInput() map[string]any {
	input, _ := readHostEnvelope()
	return input
}

// getPluginConfig reads the raw host input, unwraps the "plugin_config" key
// from the {"input": {...}, "plugin_config": {...}} envelope, and returns it
// as a map[string]any. Returns an empty map on parse failure or when no config
// is set.
//
// plugin_config contains the installation's configuration values as defined
// by the plugin's configuration_schema in manifest.json.
//
// Note: this calls pdk.Input() internally. If you need both input and
// plugin_config, use readHostEnvelope() instead to avoid reading the input
// buffer twice.
func getPluginConfig() map[string]any {
	_, config := readHostEnvelope()
	return config
}

// ---------------------------------------------------------------------------
// Storage helpers
// ---------------------------------------------------------------------------

// storageWrite writes data to a file in the /storage directory.
// The /storage directory persists across plugin upgrades.
func storageWrite(filename string, data []byte) error {
	if err := os.MkdirAll("/storage", 0755); err != nil {
		return fmt.Errorf("failed to create /storage: %w", err)
	}
	path := "/storage/" + filename
	if err := os.WriteFile(path, data, 0644); err != nil {
		return fmt.Errorf("failed to write %s: %w", path, err)
	}
	return nil
}

// storageRead reads a file from the /storage directory.
func storageRead(filename string) ([]byte, error) {
	path := "/storage/" + filename
	data, err := os.ReadFile(path)
	if err != nil {
		return nil, fmt.Errorf("failed to read %s: %w", path, err)
	}
	return data, nil
}

// ---------------------------------------------------------------------------
// Plugin capability — replace with your own
// ---------------------------------------------------------------------------

// run is the exported capability function. The host invokes it when the
// capability declared in manifest.json is triggered. The input format depends
// on the capability type (tool, command).
//
// Rename this function and update manifest.json "execute" to match.
//
//go:wasmexport run
func run() int32 {
	logInfo("run: entry point called")

	// --- 1. Read input and plugin config from the host ---------------------
	//
	// The host sends a JSON envelope:
	//   {"input": {...}, "plugin_config": {...}}
	//
	// readHostEnvelope() unwraps both fields in a single parse.
	// getInput() unwraps just "input"; getPluginConfig() unwraps just
	// "plugin_config" (the installation's configuration values, kept
	// separate from input).
	inputData, pluginConfig := readHostEnvelope()

	logDebug(fmt.Sprintf("run: input keys: %d, plugin_config keys: %d", len(inputData), len(pluginConfig)))

	// --- 2. Your plugin logic goes here ------------------------------------
	//
	// The shape of inputData depends on the capability type:
	//
	//   tool (execute mode):  {"mode": "execute", "arguments": {...}}
	//   tool (define mode):   {"mode": "define"}
	//   command:              {"param": "value", ...}  (open object)
	//   student_support:       {"locale": "en"}
	//
	// pluginConfig contains the installation's configuration values as
	// defined by the plugin's configuration_schema in manifest.json.

	// --- 3. Send output back to the host ----------------------------------
	result := map[string]any{
		"status": "ok",
	}

	outputJSON, err := json.Marshal(result)
	if err != nil {
		pdk.SetError(fmt.Errorf("failed to marshal result: %w", err))
		return 1
	}

	pdk.OutputString(string(outputJSON))
	return 0
}

// ---------------------------------------------------------------------------
// Lifecycle hooks (optional — uncomment and add to manifest.json if needed)
// ---------------------------------------------------------------------------

// //go:wasmexport on_install
// func onInstall() int32 {
// 	logInfo("on_install: called")
// 	return 0
// }
//
// //go:wasmexport on_before_upgrade
// func onBeforeUpgrade() int32 {
// 	logInfo("on_before_upgrade: called")
// 	return 0
// }
//
// //go:wasmexport on_after_upgrade
// func onAfterUpgrade() int32 {
// 	logInfo("on_after_upgrade: called")
// 	return 0
// }
//
// //go:wasmexport on_uninstall
// func onUninstall() int32 {
// 	logInfo("on_uninstall: called")
// 	return 0
// }

// main is required by the Go compiler for a c-shared build but is never
// called in the WASM environment — execution starts at the exported
// functions.
func main() {}