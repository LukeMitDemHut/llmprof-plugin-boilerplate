//! Host capability helper module.
//!
//! This module provides typed wrappers around the LLMProf host capability
//! system. The host exposes a single raw WASM import — `call_host_capability` —
//! that accepts a JSON envelope and returns a JSON response. All higher-level
//! capability requests (`add_message_activity`, `request_system_model`,
//! `request_user_i18n`) are built on top of this single import.
//!
//! # Memory protocol
//!
//! 1. Allocate a [`Memory`] block large enough for the JSON payload string.
//! 2. Copy the payload bytes into the allocated memory.
//! 3. Pass the memory offset (as `u64`) to `call_host_capability`.
//! 4. The host returns a `u64` offset pointing to its response memory.
//! 5. Read the response bytes from that offset and decode as UTF-8.

use extism_pdk::Memory;
use serde_json::Value;

// Raw WASM import provided by the LLMProf Extism host.
//
// The argument is a `u64` memory offset pointing to a JSON envelope string.
// The return value is a `u64` memory offset pointing to a JSON response string.
//
// The import must be in the `extism:host/user` module to match the
// Extism host function registration (same namespace used by Go and JS
// plugins).
#[link(wasm_import_module = "extism:host/user")]
extern "C" {
    fn call_host_capability(ptr: u64) -> u64;
}

/// Low-level helper: send a raw JSON string to the host and read back the
/// response string.
///
/// This function handles the full memory lifecycle:
/// - Allocates Extism memory for the payload
/// - Copies the payload bytes in
/// - Calls the raw `call_host_capability` import
/// - Reads the response from the returned memory offset
///
/// # Arguments
///
/// * `payload` — A JSON string to send to the host.
///
/// # Returns
///
/// The host's response as a `String`, or an error message if memory
/// allocation or UTF-8 decoding fails.
pub fn call_host_capability_raw(payload: &str) -> Result<String, String> {
    // Allocate Extism memory for the payload.
    let memory = Memory::from_bytes(payload.as_bytes())
        .map_err(|e| format!("Failed to allocate memory for payload: {e}"))?;

    // Call the raw host import with the memory offset.
    let response_offset = unsafe { call_host_capability(memory.offset()) };

    // Read the response bytes from the returned offset.
    let response_memory = Memory::find(response_offset)
        .ok_or_else(|| format!("Failed to locate response memory at offset {response_offset}"))?;

    let response_bytes = response_memory.to_vec();

    String::from_utf8(response_bytes)
        .map_err(|e| format!("Host response is not valid UTF-8: {e}"))
}

/// Send a capability request to the host.
///
/// Builds the JSON envelope expected by the host:
///
/// ```json
/// {
///   "capability": "add_message_activity",
///   "input": { ... }
/// }
/// ```
///
/// # Arguments
///
/// * `capability` — The host capability identifier (e.g. `"add_message_activity"`).
/// * `input` — The JSON value to pass as the capability input.
///
/// # Returns
///
/// The parsed JSON response from the host, or an error string.
pub fn request_capability(
    capability: &str,
    input: &Value,
) -> Result<Value, String> {
    // Build the JSON envelope.
    let envelope = serde_json::json!({
        "capability": capability,
        "input": input,
    });

    // Serialize the envelope to a string.
    let payload = serde_json::to_string(&envelope)
        .map_err(|e| format!("Failed to serialize capability envelope: {e}"))?;

    // Send to host and read response.
    let response_str = call_host_capability_raw(&payload)?;

    // Parse the response as JSON.
    let response: Value = serde_json::from_str(&response_str)
        .map_err(|e| format!("Failed to parse host response as JSON: {e}"))?;

    Ok(response)
}

/// Add a message activity entry via the host.
///
/// The message is resolved automatically from the execution token's
/// `message_id` claim — no key or message reference needs to be passed.
///
/// # Arguments
///
/// * `activity` — The activity payload as a JSON value.
///
/// # Returns
///
/// The host's JSON response, or an error string.
pub fn add_message_activity(activity: Value) -> Result<Value, String> {
    let input = serde_json::json!({
        "activity": activity,
    });

    request_capability("add_message_activity", &input)
}

/// Resolve the current context for this plugin execution.
///
/// The context is derived from the execution token's `message_id` claim
/// (message → chat → context). Returns the context UUID and name, or `null`
/// when no message is available.
///
/// # Returns
///
/// The host's JSON response containing `{"context": {"uuid": "...", "name": "..."}}`,
/// or `{"context": null}` when no message is available.
pub fn resolve_context() -> Result<Value, String> {
    request_capability("resolve_context", &serde_json::json!({}))
}

/// Resolve the current message for this plugin execution.
///
/// The message is identified by the `message_id` claim in the execution token.
/// Returns the message ID, or `null` when no message is available.
///
/// # Returns
///
/// The host's JSON response containing `{"message": {"id": 42}}`,
/// or `{"message": null}` when no message is available.
pub fn resolve_message() -> Result<Value, String> {
    request_capability("resolve_message", &serde_json::json!({}))
}

/// Request a completion from the system LLM model.
///
/// # Arguments
///
/// * `request` — The prompt or request string to send to the model.
/// * `schema` — The JSON schema describing the expected response format.
///
/// # Returns
///
/// The model's response as a JSON value, or an error string.
pub fn request_system_model(
    request: &str,
    schema: Value,
) -> Result<Value, String> {
    let input = serde_json::json!({
        "request": request,
        "schema": schema,
    });

    request_capability("request_system_model", &input)
}

/// Request the user's i18n / locale settings.
///
/// # Returns
///
/// The user's i18n configuration as a JSON value, or an error string.
pub fn request_user_i18n() -> Result<Value, String> {
    request_capability("request_user_i18n", &serde_json::json!({}))
}

// ---------------------------------------------------------------------------
// Logging helpers
// ---------------------------------------------------------------------------

/// Send a log entry to the host's `log` host capability.
///
/// If the host does not support the `log` capability the call is silently
/// ignored — logging must never break plugin execution.
///
/// # Arguments
///
/// * `level` — Log level: `"debug"`, `"info"`, `"warn"`, `"error"`.
/// * `message` — Log message.
pub fn log_host(level: &str, message: &str) {
    let input = serde_json::json!({
        "level": level,
        "message": message,
    });
    // Best-effort: ignore the response and any errors.
    let _ = request_capability("log", &input);
}

/// Convenience wrapper: log at debug level.
pub fn log_debug(message: &str) {
    log_host("debug", message);
}

/// Convenience wrapper: log at info level.
pub fn log_info(message: &str) {
    log_host("info", message);
}

/// Convenience wrapper: log at error level.
pub fn log_error(message: &str) {
    log_host("error", message);
}