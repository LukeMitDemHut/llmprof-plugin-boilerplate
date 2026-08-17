//! requesting-external-data — Rust example plugin for LLMProf.
//!
//! Demonstrates an outbound HTTP request during the on_install lifecycle hook.
//! On install, the plugin fetches https://llm-prof.de/ and stores the HTTP
//! status code plus a snippet of the response body to /storage/fetch-result.json.
//!
//! Key points:
//! - The target host MUST be listed in manifest.json `allowed_hosts`.
//! - Rust PDK provides `extism_pdk::HttpRequest` for HTTP without WASI.
//! - /storage persists across upgrades; use native WASI filesystem (std::fs).

use std::fs;

use extism_pdk::*;
use serde_json::{json, Value};

// ---------------------------------------------------------------------------
// Host capability helpers
// ---------------------------------------------------------------------------

// Raw WASM import binding for the Extism host function.
// The host function lives in the "extism:host/user" module.
#[link(wasm_import_module = "extism:host/user")]
extern "C" {
    fn call_host_capability(ptr: u64) -> u64;
}

/// Sends a host capability request and returns the raw JSON string response.
fn request_capability(capability: &str, input: Value) -> String {
    let payload = json!({"capability": capability, "input": input});
    let bytes = payload.to_string().into_bytes();
    let mem_in = Memory::from_bytes(&bytes).expect("alloc");
    let mem_out_offset = unsafe { call_host_capability(mem_in.offset()) };
    let mem_out = Memory::find(mem_out_offset).expect("response");
    String::from_utf8(mem_out.to_vec()).unwrap_or_default()
}

/// Writes a log entry to the host's `log` host capability.
fn log_host(level: &str, message: &str) {
    let _ = request_capability("log", json!({
        "level": level,
        "message": message,
    }));
}

fn log_info(message: &str) {
    log_host("info", message);
}

fn log_error(message: &str) {
    log_host("error", message);
}

// ---------------------------------------------------------------------------
// Lifecycle hooks
// ---------------------------------------------------------------------------

/// on_install is called once when the plugin is first installed.
/// It fetches https://llm-prof.de/ and stores the result in /storage.
#[plugin_fn]
pub fn on_install(_input: Json<Value>) -> FnResult<i32> {
    log_info("on_install: starting HTTP request to llm-prof.de");

    // --- 1. Make an HTTP GET request ---------------------------------------
    // The host must be listed in manifest.json `allowed_hosts`.
    let request = HttpRequest::new("https://llm-prof.de/")
        .with_header("User-Agent", "llmprof-plugin-requesting-external-data/1.0");

    let response = http::request::<&[u8]>(&request, None)?;

    let status = response.status_code();
    log_info(&format!("on_install: HTTP response status: {}", status));

    // Read the response body (may be large — we store only a snippet)
    let body = response.body();
    let body_str = String::from_utf8_lossy(&body);
    let body_snippet = if body_str.len() > 500 {
        &body_str[..500]
    } else {
        &body_str
    };

    // --- 2. Store the result in /storage -----------------------------------
    fs::create_dir_all("/storage").map_err(|e| {
        log_error(&format!("on_install: failed to create /storage: {}", e));
        Error::msg(format!("failed to create /storage: {e}"))
    })?;

    let result = json!({
        "url": "https://llm-prof.de/",
        "status": status,
        "body_snippet": body_snippet,
        "body_length": body.len(),
    });

    let result_json = serde_json::to_string_pretty(&result).map_err(|e| {
        log_error(&format!("on_install: failed to serialize result: {}", e));
        Error::msg(format!("serialize error: {e}"))
    })?;

    fs::write("/storage/fetch-result.json", result_json).map_err(|e| {
        log_error(&format!("on_install: failed to write fetch-result.json: {}", e));
        Error::msg(format!("write error: {e}"))
    })?;

    log_info(&format!(
        "on_install: stored fetch result (status={}, body_length={}) to /storage/fetch-result.json",
        status,
        body.len()
    ));

    Ok(0)
}

/// on_uninstall is called when the plugin is removed.
/// /storage may be deleted after this returns.
#[plugin_fn]
pub fn on_uninstall(_input: Json<Value>) -> FnResult<i32> {
    log_info("on_uninstall: called");
    Ok(0)
}

// ---------------------------------------------------------------------------
// Dummy capability (required by manifest validation)
// ---------------------------------------------------------------------------

/// ping is a no-op capability required by the host's manifest validation
/// (capabilities must be a non-empty list). This plugin is primarily an
/// HTTP-request demonstration; ping simply returns 0.
#[plugin_fn]
pub fn ping(_input: Json<Value>) -> FnResult<i32> {
    Ok(0)
}