//! LLMProf Plugin Boilerplate — Rust entry point.
//!
//! This crate compiles to a `wasm32-wasip1` target using `crate-type =
//! ["cdylib"]`. The Extism PDK `#[plugin_fn]` macro exports the `run`
//! function, which the LLMProf host calls with a JSON input payload.
//!
//! This is a generic template — it contains no actual functionality.
//! Replace the placeholder code with your own.
//!
//! Plugins may also declare lifecycle hooks (on_install, on_before_upgrade,
//! on_after_upgrade, on_uninstall).
//!
//! See the `examples/` directory for complete working plugins of each type.

use extism_pdk::*;

mod host_capability;
mod storage;

/// Main plugin entry point.
///
/// This function is exported via the `#[plugin_fn]` macro and is called by
/// the LLMProf Extism host. The host passes a JSON object (wrapped in
/// `Json<serde_json::Value>`) and expects a JSON string in return.
///
/// The shape of the input depends on the capability type:
///
/// - tool (execute mode): `{"mode": "execute", "arguments": {...}}`
/// - tool (define mode):  `{"mode": "define"}`
/// - command:             `{"param": "value", ...}` (open object)
///
/// Rename this function and update `manifest.json` `"execute"` to match.
#[plugin_fn]
pub fn run(input: Json<serde_json::Value>) -> FnResult<String> {
    host_capability::log_info("run: entry point called");

    // --- 1. Read input from the host ---------------------------------------
    let input_value = input.0;
    host_capability::log_debug(&format!(
        "run: input: {}",
        serde_json::to_string(&input_value).unwrap_or_default()
    ));

    // --- 2. Your plugin logic goes here ------------------------------------
    //
    // Available helpers:
    //   host_capability::log_debug(msg)    — log at debug level
    //   host_capability::log_info(msg)     — log at info level
    //   host_capability::log_error(msg)    — log at error level
    //   host_capability::request_capability(name, &input) — call any host capability
    //   host_capability::add_message_activity(key, activity) — add message activity
    //   host_capability::request_system_model(request, schema) — ask system LLM
    //   storage::read_file(filename)       — read a file from /storage
    //   storage::write_file(filename, content) — write a file to /storage
    //   storage::list_files(path)          — list files in /storage
    //   storage::delete_file(filename)     — delete a file from /storage
    //
    // See examples/ for complete implementations of each capability type.

    // --- 3. Send output back to the host ----------------------------------
    let result = serde_json::json!({
        "status": "ok"
    });

    Ok(serde_json::to_string(&result)
        .map_err(|e| Error::msg(format!("Failed to serialize result: {e}")))?)
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