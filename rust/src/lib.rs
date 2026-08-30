//! LLMProf Plugin Boilerplate — Rust entry point.
//!
//! This crate compiles to a `wasm32-wasip1` target using `crate-type =
//! ["cdylib"]`. The Extism PDK `#[plugin_fn]` macro exports the `run`
//! function, which the LLMProf host calls with a JSON input payload.
//!
//! The host sends a JSON envelope:
//!
//! ```json
//! { "input": {...}, "plugin_config": {...} }
//! ```
//!
//! `get_input()` and `get_plugin_config()` unwrap the respective fields.
//! `plugin_config` contains the installation's configuration values as
//! defined by the plugin's `configuration_schema` in manifest.json.
//!
//! This is a generic template — it contains no actual functionality.
//! Replace the placeholder code with your own.
//!
//! Plugins may also declare lifecycle hooks (on_install, on_before_upgrade,
//! on_after_upgrade, on_uninstall).
//!
//! See the `examples/` directory for complete working plugins of each type.

use extism_pdk::*;
use serde_json::Value;

mod host_capability;
mod storage;

/// Main plugin entry point.
///
/// This function is exported via the `#[plugin_fn]` macro and is called by
/// the LLMProf Extism host. The host passes a JSON envelope
/// `{"input": {...}, "plugin_config": {...}}` and expects a JSON string
/// in return.
///
/// Rename this function and update `manifest.json` `"execute"` to match.
#[plugin_fn]
pub fn run(envelope: Json<Value>) -> FnResult<String> {
    host_capability::log_info("run: entry point called");

    // --- 1. Unwrap input and plugin_config from the host envelope ----------
    let input_value = get_input(&envelope.0);
    let plugin_config = get_plugin_config(&envelope.0);

    host_capability::log_debug(&format!(
        "run: input: {}",
        serde_json::to_string(&input_value).unwrap_or_default()
    ));

    // --- 2. Your plugin logic goes here ------------------------------------
    //
    // The shape of input_value depends on the capability type:
    //
    //   tool (execute mode): {"mode": "execute", "arguments": {...}}
    //   tool (define mode):  {"mode": "define"}
    //   command:             {"param": "value", ...} (open object)
    //   student_support:     {"locale": "en"}
    //
    // plugin_config contains the installation's configuration values as
    // defined by the plugin's configuration_schema in manifest.json.
    //
    // Available helpers:
    //   host_capability::log_debug(msg)    — log at debug level
    //   host_capability::log_info(msg)     — log at info level
    //   host_capability::log_error(msg)    — log at error level
    //   host_capability::request_capability(name, &input) — call any host capability
    //   host_capability::add_message_activity(activity) — add message activity
    //   host_capability::resolve_context() — resolve current context
    //   host_capability::resolve_message() — resolve current message
    //   host_capability::request_system_model(request, schema) — ask system LLM
    //   storage::read_file(filename)       — read a file from /storage
    //   storage::write_file(filename, content) — write a file to /storage
    //   storage::list_files(path)          — list files in /storage
    //   storage::delete_file(filename)     — delete a file from /storage

    // --- 3. Send output back to the host ----------------------------------
    let result = serde_json::json!({
        "status": "ok"
    });

    Ok(serde_json::to_string(&result)
        .map_err(|e| Error::msg(format!("Failed to serialize result: {e}")))?)
}

// ---------------------------------------------------------------------------
// Input / plugin config helpers
// ---------------------------------------------------------------------------

/// Extract the "input" field from the host envelope
/// `{"input": {...}, "plugin_config": {...}}`.
///
/// Returns `Value::Null` if the field is missing.
pub fn get_input(envelope: &Value) -> Value {
    envelope.get("input").cloned().unwrap_or(Value::Null)
}

/// Extract the "plugin_config" field from the host envelope
/// `{"input": {...}, "plugin_config": {...}}`.
///
/// Returns an empty object `{}` if the field is missing.
pub fn get_plugin_config(envelope: &Value) -> Value {
    envelope
        .get("plugin_config")
        .cloned()
        .unwrap_or(serde_json::json!({}))
}

// ---------------------------------------------------------------------------
// Lifecycle hooks (optional — uncomment and add to manifest.json if needed)
// ---------------------------------------------------------------------------

// #[plugin_fn]
// pub fn on_install(_input: Json<serde_json::Value>) -> FnResult<String> {
//     host_capability::log_info("on_install: called");
//     Ok(r#"{"status":"ok"}"#.to_string())
// }
//
// #[plugin_fn]
// pub fn on_before_upgrade(_input: Json<serde_json::Value>) -> FnResult<String> {
//     host_capability::log_info("on_before_upgrade: called");
//     Ok(r#"{"status":"ok"}"#.to_string())
// }
//
// #[plugin_fn]
// pub fn on_after_upgrade(_input: Json<serde_json::Value>) -> FnResult<String> {
//     host_capability::log_info("on_after_upgrade: called");
//     Ok(r#"{"status":"ok"}"#.to_string())
// }
//
// #[plugin_fn]
// pub fn on_uninstall(_input: Json<serde_json::Value>) -> FnResult<String> {
//     host_capability::log_info("on_uninstall: called");
//     Ok(r#"{"status":"ok"}"#.to_string())
// }