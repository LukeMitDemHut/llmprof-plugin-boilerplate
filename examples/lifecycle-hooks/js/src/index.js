/**
 * Lifecycle Hooks Example Plugin (JavaScript)
 *
 * Demonstrates all 4 lifecycle hooks by appending timestamped entries to
 * /storage/lifecycle.log.
 *
 * Unlike Go and Rust (which use native WASI filesystem access), JS plugins
 * interact with storage through host functions. The `write_file` host function
 * overwrites the entire file, so we read the existing content first, append
 * the new entry, and write the combined content back.
 */

/**
 * Calls a storage host function (read_file, write_file, etc.) and returns the
 * parsed JSON response.
 *
 * JS plugins have NO native filesystem — all I/O must go through host functions
 * in the `extism:host/user` namespace, accessed via `Host.getFunctions()`.
 *
 * @param {string} fnName  - Host function name ("read_file", "write_file", …).
 * @param {object} payload - Input object to JSON-encode and send.
 * @returns {object}        - Parsed JSON response from the host.
 * @throws {Error} If the host function is missing or the response is invalid.
 */
function callStorageFn(fnName, payload) {
  const fns = Host.getFunctions();
  const fn = fns[fnName];
  if (!fn) {
    throw new Error("Host did not provide '" + fnName + "' function.");
  }

  const requestPayload = JSON.stringify(payload);

  // Allocate memory for the request and call the host function.
  const memIn = Memory.fromString(requestPayload);
  const memOutOffset = fn(memIn.offset);
  memIn.free();

  // Read and parse the response.
  const memOut = Memory.find(memOutOffset);
  return JSON.parse(memOut.readString());
}

/**
 * Appends a timestamped entry to /storage/lifecycle.log via host functions.
 *
 * Because `write_file` overwrites (not appends), we:
 *   1. Read the current file contents (if it exists)
 *   2. Append the new entry
 *   3. Write the full contents back
 *
 * @param {string} entry - The log line to append (without trailing newline).
 * @returns {number} 0 on success, non-zero on error.
 */
function appendLog(entry) {
  const filename = "lifecycle.log";
  const line = entry + "\n";

  let existing = "";

  try {
    const result = callStorageFn("read_file", { filename: filename });
    if (result && result.content) {
      existing = result.content;
    }
  } catch (e) {
    // File doesn't exist yet — that's fine for on_install.
    existing = "";
  }

  const newContent = existing + line;

  try {
    callStorageFn("write_file", { filename: filename, content: newContent });
  } catch (e) {
    return 1;
  }

  return 0;
}

/**
 * Returns the current UTC time as an ISO 8601 string (e.g. "2026-07-10T12:00:00.000Z").
 * @returns {string}
 */
function nowISO() {
  return new Date().toISOString();
}

/**
 * on_install — called once when the plugin is first installed.
 * Good for initializing storage, creating config files, seeding data.
 * @returns {number} 0 on success.
 */
function on_install() {
  return appendLog("installed at " + nowISO());
}

/**
 * on_before_upgrade — called on the OLD wasm before files are swapped.
 * Good for backing up data, flushing state, writing migration markers.
 * @returns {number} 0 on success.
 */
function on_before_upgrade() {
  return appendLog("before upgrade at " + nowISO());
}

/**
 * on_after_upgrade — called on the NEW wasm after files are swapped.
 * Good for migrating data formats, verifying integrity, cleaning up backups.
 * @returns {number} 0 on success.
 */
function on_after_upgrade() {
  return appendLog("after upgrade at " + nowISO());
}

/**
 * on_uninstall — called when the plugin is removed.
 * Good for cleanup — note that /storage may be removed after this returns.
 * @returns {number} 0 on success.
 */
function on_uninstall() {
  return appendLog("uninstalled at " + nowISO());
}

/**
 * ping — dummy no-op capability required by the host's manifest validation
 * (capabilities must be a non-empty list). This plugin is primarily a
 * lifecycle-hooks demonstration; the ping capability simply returns 0.
 * @returns {number} 0 on success.
 */
function ping() {
  return 0;
}

module.exports = {
  ping,
  on_install,
  on_before_upgrade,
  on_after_upgrade,
  on_uninstall,
};
