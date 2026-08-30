//! "tool-with-host-capability" example plugin (Rust).
//!
//! This example demonstrates a tool capability that:
//! 1. In "define" mode:  returns a tool schema (name, description, parameters).
//! 2. In "execute" mode: calls the `request_system_model` host capability to
//!    get an LLM summary, saves the result to `/storage/results.json` using
//!    the native WASI filesystem (`std::fs`), and sends an
//!    `add_message_activity` card to report progress.
//!
//! The host invokes the exported `summarize` function with a JSON envelope
//! that includes "input" and "plugin_config" as separate top-level keys.
//!
//! # Input JSON structure
//!
//! ```json
//! { "input": {"mode": "define"}, "plugin_config": {} }
//! ```
//!
//! ```json
//! {
//!   "input": {
//!     "mode": "execute",
//!     "arguments": { "query": "..." }
//!   },
//!   "plugin_config": {}
//! }
//! ```

use extism_pdk::*;
use serde_json::Value;
use std::fs;

// Raw WASM import provided by the LLMProf Extism host.
//
// The argument is a `u64` memory offset pointing to a JSON envelope string.
// The return value is a `u64` memory offset pointing to a JSON response
// string.
//
// The import must be in the `extism:host/user` module to match the
// Extism host function registration (same namespace used by Go and JS
// plugins).
#[link(wasm_import_module = "extism:host/user")]
extern "C" {
    fn call_host_capability(ptr: u64) -> u64;
}

/// Send a capability request to the host and return the parsed JSON response.
///
/// Builds the JSON envelope expected by the host:
///
/// ```json
/// { "capability": "<capability>", "input": <input> }
/// ```
fn request_capability(capability: &str, input: &Value) -> Result<Value, String> {
    let envelope = serde_json::json!({
        "capability": capability,
        "input": input,
    });

    let payload = serde_json::to_string(&envelope)
        .map_err(|e| format!("Failed to serialize capability envelope: {e}"))?;

    let memory = Memory::from_bytes(payload.as_bytes())
        .map_err(|e| format!("Failed to allocate memory for payload: {e}"))?;

    let response_offset = unsafe { call_host_capability(memory.offset()) };

    let response_memory = Memory::find(response_offset)
        .ok_or_else(|| format!("Failed to locate response memory at offset {response_offset}"))?;

    let response_bytes = response_memory.to_vec();

    let response_str = String::from_utf8(response_bytes)
        .map_err(|e| format!("Host response is not valid UTF-8: {e}"))?;

    let response: Value = serde_json::from_str(&response_str)
        .map_err(|e| format!("Failed to parse host response as JSON: {e}"))?;

    Ok(response)
}

// ---------------------------------------------------------------------------
// Logging helpers (best-effort — never fail the plugin)
// ---------------------------------------------------------------------------

/// Send a log entry to the host's `log` host capability.
/// If the host does not support the `log` capability the call is silently
/// ignored — logging must never break plugin execution.
fn log_host(level: &str, message: &str) {
    let input = serde_json::json!({
        "level": level,
        "message": message,
    });
    let _ = request_capability("log", &input);
}

/// Debug-level log.
fn log_debug(message: &str) {
    log_host("debug", message);
}

/// Info-level log.
fn log_info(message: &str) {
    log_host("info", message);
}

/// Error-level log.
fn log_error(message: &str) {
    log_host("error", message);
}

/// Main plugin entry point, exported via the `#[plugin_fn]` macro.
///
/// The host passes a JSON envelope `{"input": {...}, "plugin_config": {...}}`
/// and expects a JSON string in return. Errors are reported back to the host
/// with a non-zero exit code via `WithReturnCode`.
#[plugin_fn]
pub fn summarize(envelope: Json<Value>) -> FnResult<String> {
    log_info("summarize entry point called");

    // Unwrap the "input" field from the host envelope.
    let input_value = envelope
        .0
        .get("input")
        .cloned()
        .unwrap_or(Value::Null);

    // Determine the mode: "define" or "execute".
    let mode = input_value
        .get("mode")
        .and_then(|m| m.as_str())
        .unwrap_or("");

    log_info(&format!("dispatching mode: \"{}\"", mode));

    match mode {
        // --- Define mode ----------------------------------------------------
        //
        // Return the tool schema so the host can register the tool with the
        // LLM function-calling interface.
        "define" => {
            log_info("handleDefine: building tool schema");
            let definition = serde_json::json!({
                "mode": "define",
                "name": "summarize",
                "description": "Summarise a block of text using the system LLM model and save the result to storage.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "query": {
                            "type": "string",
                            "description": "The text to summarise."
                        }
                    },
                    "required": ["query"]
                }
            });

            let result = serde_json::to_string(&definition).map_err(|e| {
                Error::msg(format!("Failed to serialize tool definition: {e}"))
            })?;

            Ok(result)
        }

        // --- Execute mode ---------------------------------------------------
        //
        // Run the tool: call request_system_model, save to /storage, report
        // progress via add_message_activity, and return the result.
        "execute" => {
            log_info("handleExecute: starting");

            // Extract the tool arguments.
            let args = input_value
                .get("arguments")
                .cloned()
                .unwrap_or(Value::Null);

            let query = args
                .get("query")
                .and_then(|q| q.as_str())
                .unwrap_or("");

            log_debug(&format!("handleExecute: query length: {}", query.len()));

            // 1. Request a summary from the system LLM model.
            log_info("handleExecute: calling request_system_model");
            let model_input = serde_json::json!({
                "request": format!("Summarise the following: {query}"),
                "schema": {
                    "type": "object",
                    "properties": {
                        "summary": { "type": "string" }
                    }
                }
            });

            let model_response =
                request_capability("request_system_model", &model_input).map_err(|e| {
                    Error::msg(format!("request_system_model failed: {e}"))
                })?;

            log_info("handleExecute: model response parsed successfully");

            // 2. Save the result to /storage/results.json.
            //    Rust plugins run under WASI and have native filesystem
            //    access via std::fs — no host functions needed.
            let response_str = serde_json::to_string(&model_response).map_err(|e| {
                Error::msg(format!("Failed to serialize model response: {e}"))
            })?;

            log_info("handleExecute: writing /storage/results.json");
            fs::create_dir_all("/storage")
                .map_err(|e| Error::msg(format!("Failed to create /storage: {e}")))?;

            fs::write("/storage/results.json", &response_str)
                .map_err(|e| Error::msg(format!("Failed to write results.json: {e}")))?;

            log_info("handleExecute: results.json written successfully");

            // 3. Extract the summary from the model response.
            let summary = model_response
                .get("result")
                .and_then(|r| r.get("summary"))
                .and_then(|s| s.as_str())
                .unwrap_or("");

            log_debug(&format!(
                "handleExecute: extracted summary length: {}",
                summary.len()
            ));

            // 4. Report progress via add_message_activity.
            log_info("handleExecute: calling add_message_activity");
            let activity_input = serde_json::json!({
                "activity": {
                    "title": "Summary Generated",
                    "content": summary,
                    "origin": "tool-with-host-capability",
                    "icon": "FileText"
                }
            });

            // Best-effort: report the activity but don't fail the tool if
            // the host rejects it.
            let _ = request_capability("add_message_activity", &activity_input);

            // 5. Return the tool execute response.
            log_info("handleExecute: building final result");
            let result = serde_json::json!({
                "mode": "execute",
                "result": {
                    "content": summary
                }
            });

            let result_str = serde_json::to_string(&result).map_err(|e| {
                Error::msg(format!("Failed to serialize result: {e}"))
            })?;

            log_info("handleExecute: returning success");
            Ok(result_str)
        }

        // --- Unknown mode ---------------------------------------------------
        _ => {
            log_error(&format!(
                "unknown mode: \"{}\" (expected \"define\" or \"execute\")",
                mode
            ));
            let err = Error::msg(format!("Unknown mode: '{mode}'"));
            Err(WithReturnCode::new(err, 1))
        }
    }
}