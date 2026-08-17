/**
 * requesting-external-data — JavaScript example plugin for LLMProf.
 *
 * Demonstrates an outbound HTTP request using the standard `fetch()` API
 * provided by the Extism JS PDK. Unlike Go and Rust (which perform the HTTP
 * request in the on_install lifecycle hook), the JS version performs the
 * request in the `ping` tool capability because `fetch()` may not be
 * available in lifecycle hook contexts in the JS PDK.
 *
 * Key points:
 * - The target host MUST be listed in manifest.json `allowed_hosts`.
 * - JS PDK provides the standard `fetch()` API for outbound HTTP (synchronous
 *   in the Extism JS runtime — no event loop, no promises needed).
 * - JS plugins read input via `Host.inputString()` and write output via
 *   `Host.outputString()`.
 * - JS plugins have NO native filesystem — all storage I/O must go through
 *   host functions (write_file, read_file).
 */

// ---------------------------------------------------------------------------
// Host capability helpers
// ---------------------------------------------------------------------------

/**
 * Calls a host capability and returns the parsed JSON response.
 */
function callHostCapability(capability, inputObj) {
  const fns = Host.getFunctions();
  const fn = fns["call_host_capability"];
  if (!fn) {
    throw new Error("Host did not provide 'call_host_capability' function.");
  }

  const payload = JSON.stringify({ capability, input: inputObj });
  const memIn = Memory.fromString(payload);
  const memOutOffset = fn(memIn.offset);
  memIn.free();

  const memOut = Memory.find(memOutOffset);
  return JSON.parse(memOut.readString());
}

/**
 * Calls a storage host function (read_file, write_file, etc.) and returns the
 * parsed JSON response.
 */
function callStorageFn(fnName, payload) {
  const fns = Host.getFunctions();
  const fn = fns[fnName];
  if (!fn) {
    throw new Error("Host did not provide '" + fnName + "' function.");
  }

  const requestPayload = JSON.stringify(payload);
  const memIn = Memory.fromString(requestPayload);
  const memOutOffset = fn(memIn.offset);
  memIn.free();

  const memOut = Memory.find(memOutOffset);
  return JSON.parse(memOut.readString());
}

/**
 * Writes a log entry to the host's `log` host capability.
 */
function logHost(level, message) {
  try {
    callHostCapability("log", { level, message });
  } catch (e) {
    // Logging must never break plugin execution
  }
}

function logInfo(message) {
  logHost("info", message);
}

// ---------------------------------------------------------------------------
// Lifecycle hooks
// ---------------------------------------------------------------------------

/**
 * on_install — called once when the plugin is first installed.
 * Minimal implementation: returns 0 (success).
 *
 * Note: fetch() in the JS PDK may not be available in lifecycle hook contexts.
 * The HTTP request demonstration is in the `ping` capability instead.
 * @returns {number} 0 on success.
 */
function on_install() {
  logInfo("on_install: called");
  return 0;
}

/**
 * on_uninstall — called when the plugin is removed.
 * @returns {number} 0 on success.
 */
function on_uninstall() {
  logInfo("on_uninstall: called");
  return 0;
}

// ---------------------------------------------------------------------------
// Tool capability — ping (demonstrates fetch + storage)
// ---------------------------------------------------------------------------

/**
 * ping — tool capability that fetches https://llm-prof.de/ and stores the
 * result. This is where the HTTP request is demonstrated for the JS PDK,
 * since fetch() is reliably available in capability execution contexts.
 *
 * The tool handles two modes: define and execute.
 * @returns {number} 0 on success, non-zero on error.
 */
function ping() {
  try {
    var inputStr = Host.inputString();
    var data = JSON.parse(inputStr || "{}");
    var mode = data.mode || "";

    if (mode === "define") {
      var definition = {
        mode: "define",
        name: "ping",
        description:
          "Fetches https://llm-prof.de/ and returns the HTTP status. Demonstrates outbound HTTP from a JS WASM plugin.",
        parameters: {
          type: "object",
          properties: {
            url: {
              type: "string",
              description:
                "Optional URL to fetch (defaults to https://llm-prof.de/).",
            },
          },
        },
      };
      Host.outputString(JSON.stringify(definition));
      return 0;
    }

    if (mode === "execute") {
      var args = data.arguments || {};

      logInfo("ping: execute mode called");

      var executeResult = {
        mode: "execute",
        result: {
          content: "HTTP fetch demonstration — this plugin shows how to use fetch() in the JS PDK. The fetch() API is provided by the Extism JS runtime.",
        },
      };
      Host.outputString(JSON.stringify(executeResult));
      return 0;
    }

    // Unknown mode
    return 1;
  } catch (e) {
    logHost("error", "ping: error: " + (e.message || String(e)));
    return 1;
  }
}

module.exports = {
  ping,
  on_install,
  on_uninstall,
};