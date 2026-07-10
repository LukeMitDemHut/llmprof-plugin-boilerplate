use std::fs::OpenOptions;
use std::io::Write;
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{json, Value};
use extism_pdk::*;

/// Opens /storage/lifecycle.log in append mode and writes a single line.
/// The /storage directory persists across upgrades, so entries written by
/// on_before_upgrade (old wasm) are visible to on_after_upgrade (new wasm).
fn append_log(entry: &str) -> Result<(), String> {
    // Ensure /storage exists (only needed on first install, but harmless otherwise).
    std::fs::create_dir_all("/storage").map_err(|e| format!("failed to create /storage: {e}"))?;

    let mut file = OpenOptions::new()
        .append(true)
        .create(true)
        .open("/storage/lifecycle.log")
        .map_err(|e| format!("failed to open log: {e}"))?;

    writeln!(file, "{entry}").map_err(|e| format!("failed to write log: {e}"))
}

/// Returns the current UTC time as an RFC 3339 string.
/// WASI does not provide a rich timezone-aware clock, so we build the string
/// from Unix seconds manually.
fn now_rfc3339() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    // Simple conversion to a human-readable UTC timestamp.
    // For a production plugin you'd use a crate like `chrono`, but we keep
    // dependencies minimal here and fall back to Unix seconds formatting.
    let (year, month, day, hour, min, sec) = unix_to_utc(secs);
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{min:02}:{sec:02}Z")
}

/// Converts Unix seconds to broken-down UTC time.
/// Based on the civil-from-days algorithm by Howard Hinnant.
fn unix_to_utc(secs: u64) -> (i32, u32, u32, u32, u32, u32) {
    let days = (secs / 86400) as i64;
    let rem = (secs % 86400) as u32;
    let hour = rem / 3600;
    let min = (rem % 3600) / 60;
    let sec = rem % 60;

    // Civil-from-days (Howard Hinnant)
    let z = days + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u64; // [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365; // [0, 399]
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
    let mp = (5 * doy + 2) / 153; // [0, 11]
    let d = doy - (153 * mp + 2) / 5 + 1; // [1, 31]
    let m = if mp < 10 { mp + 3 } else { mp - 9 }; // [1, 12]

    let year = if m <= 2 { y + 1 } else { y } as i32;
    (year, m as u32, d as u32, hour, min, sec)
}

/// on_install is called once when the plugin is first installed.
/// Good for initializing storage, creating config files, seeding data.
#[plugin_fn]
pub fn on_install(_input: Json<Value>) -> FnResult<i32> {
    let timestamp = now_rfc3339();
    append_log(&format!("installed at {timestamp}"))
        .map_err(|e| Error::msg(format!("on_install: {e}")))?;
    Ok(0)
}

/// on_before_upgrade is called on the OLD wasm before files are swapped.
/// Good for backing up data, flushing state, writing migration markers.
#[plugin_fn]
pub fn on_before_upgrade(_input: Json<Value>) -> FnResult<i32> {
    let timestamp = now_rfc3339();
    append_log(&format!("before upgrade at {timestamp}"))
        .map_err(|e| Error::msg(format!("on_before_upgrade: {e}")))?;
    Ok(0)
}

/// on_after_upgrade is called on the NEW wasm after files are swapped.
/// Good for migrating data formats, verifying integrity, cleaning up backups.
#[plugin_fn]
pub fn on_after_upgrade(_input: Json<Value>) -> FnResult<i32> {
    let timestamp = now_rfc3339();
    append_log(&format!("after upgrade at {timestamp}"))
        .map_err(|e| Error::msg(format!("on_after_upgrade: {e}")))?;
    Ok(0)
}

/// on_uninstall is called when the plugin is removed.
/// Good for cleanup — note that /storage may be removed after this returns.
#[plugin_fn]
pub fn on_uninstall(_input: Json<Value>) -> FnResult<i32> {
    let timestamp = now_rfc3339();
    append_log(&format!("uninstalled at {timestamp}"))
        .map_err(|e| Error::msg(format!("on_uninstall: {e}")))?;
    Ok(0)
}

/// ping is a dummy no-op capability required by the host's manifest validation
/// (capabilities must be a non-empty list). This plugin is primarily a
/// lifecycle-hooks demonstration; ping simply returns 0.
#[plugin_fn]
pub fn ping(_input: Json<Value>) -> FnResult<i32> {
    Ok(0)
}