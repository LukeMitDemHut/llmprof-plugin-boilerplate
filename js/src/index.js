/**
 * index.js — Generic JS plugin entry point for LLMProf.
 *
 * This template provides the standard structure for a LLMProf JavaScript
 * WASM plugin. It contains no actual functionality — replace the placeholder
 * code with your own.
 *
 * The host sends a JSON envelope:
 *
 *   { "input": {...}, "plugin_config": {...} }
 *
 * getInput() unwraps the "input" object; getPluginConfig() unwraps
 * "plugin_config" (the installation's configuration values, kept separate
 * from input).
 *
 * Plugins may also declare lifecycle hooks (on_install, on_before_upgrade,
 * on_after_upgrade, on_uninstall).
 *
 * See the examples/ directory for complete working plugins of each type.
 */

const hostCapability = require("./host_capability");
const storage = require("./storage");

/**
 * Main entry point called by the Extism host.
 *
 * Reads the JSON envelope from the host, unwraps input and plugin_config,
 * runs the plugin logic, and writes the JSON response back via
 * Host.outputString().
 *
 * The shape of the input depends on the capability type:
 *
 *   tool (execute mode):  {"mode": "execute", "arguments": {...}}
 *   tool (define mode):   {"mode": "define"}
 *   command:              {"param": "value", ...}  (open object)
 *   student_support:       {"locale": "en"}
 *
 * plugin_config contains the installation's configuration values as defined
 * by the plugin's configuration_schema in manifest.json.
 *
 * Rename this function and update manifest.json "execute" to match.
 *
 * @returns {number} 0 on success (convention for Extism PDK entry points).
 */
function run() {
  try {
    hostCapability.logInfo("run: entry point called");

    // --- 1. Read input and plugin config from the host -------------------
    const inputStr = Host.inputString();
    const envelope = JSON.parse(inputStr);

    const input = envelope.input || {};
    const pluginConfig = envelope.plugin_config || {};

    hostCapability.logDebug("run: input keys: " + Object.keys(input).length);

    // --- 2. Your plugin logic goes here ----------------------------------
    //
    // Available helpers:
    //   hostCapability.logDebug(msg)   — log at debug level
    //   hostCapability.logInfo(msg)    — log at info level
    //   hostCapability.logError(msg)   — log at error level
    //   hostCapability.requestCapability(name, inputObj) — call any host capability
    //   hostCapability.addMessageActivity(activity) — add message activity
    //   hostCapability.resolveContext() — resolve current context
    //   hostCapability.resolveMessage() — resolve current message
    //   storage.readFile(filename)    — read a file from /storage
    //   storage.writeFile(filename, content) — write a file to /storage
    //   storage.listFiles(path)       — list files in /storage
    //   storage.deleteFile(filename)  — delete a file from /storage
    //
    // See examples/ for complete implementations of each capability type.

    // --- 3. Send output back to the host ---------------------------------
    const result = {
      status: "ok",
    };

    Host.outputString(JSON.stringify(result));
    return 0;
  } catch (err) {
    hostCapability.logError("run: " + (err.message || String(err)));
    Host.outputString(
      JSON.stringify({
        error: err.message || String(err),
      }),
    );
    throw err;
  }
}

// ---------------------------------------------------------------------------
// Lifecycle hooks (optional — uncomment and add to manifest.json if needed)
// ---------------------------------------------------------------------------

// function onInstall() {
//   hostCapability.logInfo("on_install: called");
//   return 0;
// }
//
// function onBeforeUpgrade() {
//   hostCapability.logInfo("on_before_upgrade: called");
//   return 0;
// }
//
// function onAfterUpgrade() {
//   hostCapability.logInfo("on_after_upgrade: called");
//   return 0;
// }
//
// function onUninstall() {
//   hostCapability.logInfo("on_uninstall: called");
//   return 0;
// }

module.exports = { run };
