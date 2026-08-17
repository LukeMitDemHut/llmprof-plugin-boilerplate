package main

// requesting-external-data — Go example plugin for LLMProf.
//
// Demonstrates an outbound HTTP request during the on_install lifecycle hook.
// On install, the plugin fetches https://llm-prof.de/ and stores the HTTP
// status code plus a snippet of the response body to /storage/fetch-result.json.
//
// Key points:
//   - The target host MUST be listed in manifest.json `allowed_hosts`.
//   - Go PDK provides pdk.NewHTTPRequest / req.Send() for HTTP without WASI.
//   - /storage persists across upgrades; use native WASI filesystem (os.WriteFile).

import (
	"encoding/json"
	"fmt"
	"os"
	"strconv"

	"github.com/extism/go-pdk"
)

// ---------------------------------------------------------------------------
// Host capability helpers
// ---------------------------------------------------------------------------

//go:wasmimport extism:host/user call_host_capability
func callHostCapabilityRaw(ptr uint64) uint64

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

func logHost(level, message string) {
	_ = requestCapability("log", map[string]any{
		"level":   level,
		"message": message,
	})
}

func logInfo(message string)  { logHost("info", message) }
func logError(message string) { logHost("error", message) }

// ---------------------------------------------------------------------------
// Lifecycle hooks
// ---------------------------------------------------------------------------

// on_install is called once when the plugin is first installed.
// It fetches https://llm-prof.de/ and stores the result in /storage.
//
//go:wasmexport on_install
func onInstall() int32 {
	logInfo("on_install: starting HTTP request to llm-prof.de")

	// --- 1. Make an HTTP GET request ---------------------------------------
	// The host must be listed in manifest.json `allowed_hosts`.
	// Without it, the request will fail with a permission error.
	req := pdk.NewHTTPRequest(pdk.MethodGet, "https://llm-prof.de/")
	req.SetHeader("User-Agent", "llmprof-plugin-requesting-external-data/1.0")

	res := req.Send()
	status := res.Status()
	logInfo("on_install: HTTP response status: " + strconv.Itoa(int(status)))

	// Read the response body (may be large — we store only a snippet)
	bodyBytes := res.Body()
	bodySnippet := string(bodyBytes)
	if len(bodySnippet) > 500 {
		bodySnippet = bodySnippet[:500]
	}

	// --- 2. Store the result in /storage -----------------------------------
	if err := os.MkdirAll("/storage", 0755); err != nil {
		logError("on_install: failed to create /storage: " + err.Error())
		return 1
	}

	result := map[string]any{
		"url":           "https://llm-prof.de/",
		"status":        status,
		"body_snippet":  bodySnippet,
		"body_length":   len(bodyBytes),
	}

	resultJSON, _ := json.MarshalIndent(result, "", "  ")
	if err := os.WriteFile("/storage/fetch-result.json", resultJSON, 0644); err != nil {
		logError("on_install: failed to write fetch-result.json: " + err.Error())
		return 1
	}

	logInfo(fmt.Sprintf("on_install: stored fetch result (status=%d, body_length=%d) to /storage/fetch-result.json", status, len(bodyBytes)))
	return 0
}

// on_uninstall is called when the plugin is removed.
// /storage may be deleted after this returns.
//
//go:wasmexport on_uninstall
func onUninstall() int32 {
	logInfo("on_uninstall: called")
	return 0
}

// ---------------------------------------------------------------------------
// Dummy capability (required by manifest validation)
// ---------------------------------------------------------------------------

// ping is a no-op capability required by the host's manifest validation
// (capabilities must be a non-empty list). This plugin is primarily an
// HTTP-request demonstration; ping simply returns 0.
//
//go:wasmexport ping
func ping() int32 {
	return 0
}

// main is required by the Go compiler for a c-shared build but is never
// called in the WASM environment.
func main() {}