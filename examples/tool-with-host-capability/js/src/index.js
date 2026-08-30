/**
 * index.js — "tool-with-host-capability" example plugin (JavaScript).
 *
 * This example demonstrates a tool capability that:
 *  1. In "define" mode:  returns a tool schema (name, description, parameters).
 *  2. In "execute" mode: calls the `request_system_model` host capability to
 *     get an LLM summary, saves the result to /storage/results.json via the
 *     `write_file` host function, and sends an `add_message_activity` card to
 *     report progress.
 *
 * The host invokes the exported `summarize` function, passing a JSON payload
 * via Host.inputString(). The payload's "mode" field selects the behaviour.
 *
 * IMPORTANT: JavaScript running inside a WASM module (Extism JS PDK / QuickJS)
 * has **no native filesystem access**. All file operations must go through
 * host functions (read_file, write_file) provided in the extism:host/user
 * namespace.
 */

/**
 * Low-level helper: call a host capability and return the raw JSON response
 * string.
 *
 * @param {string} capabilityName - Name of the host capability to invoke.
 * @param {object} inputObj      - Capability-specific input payload.
 * @returns {string}             - Raw JSON response string from the host.
 * @throws {Error} If the host did not provide `call_host_capability`.
 */
function callHostCapability(capabilityName, inputObj) {
  // Obtain the host function table. In the Extism JS PDK this is the only
  // way to reach functions declared in the `extism:host` namespace.
  const { call_host_capability } = Host.getFunctions();
  if (!call_host_capability) {
    throw new Error("Host did not provide 'call_host_capability' function.");
  }

  // Build the JSON envelope expected by the host gateway.
  const requestPayload = JSON.stringify({
    capability: capabilityName,
    input: inputObj,
  });

  // Allocate a memory block for the request string and remember its offset.
  const memIn = Memory.fromString(requestPayload);

  // Call the host gateway. It returns an offset to a memory block containing
  // the JSON response.
  const memOutOffset = call_host_capability(memIn.offset);

  // Free the input memory — we are responsible for cleaning up what we
  // allocated.
  memIn.free();

  // Wrap the output offset in a Memory object so we can read it.
  const memOut = Memory.find(memOutOffset);
  return memOut.readString();
}

/**
 * Convenience wrapper: call a host capability and parse the JSON response into
 * an object.
 *
 * @param {string} capabilityName - Name of the host capability to invoke.
 * @param {object} inputObj      - Capability-specific input payload.
 * @returns {object}             - Parsed JSON response object.
 * @throws {Error} If the response cannot be parsed as JSON.
 */
function requestCapability(capabilityName, inputObj) {
  const raw = callHostCapability(capabilityName, inputObj);
  return JSON.parse(raw);
}

// ---------------------------------------------------------------------------
// Logging helpers (best-effort — never throw)
// ---------------------------------------------------------------------------

/**
 * Send a log entry to the host's `log` host capability.
 * If the host does not support the `log` capability the call is silently
 * ignored — logging must never break plugin execution.
 *
 * @param {string} level    - Log level ("debug", "info", "error", etc.).
 * @param {string} message  - Log message.
 * @param {object} [context] - Optional context object.
 */
function logHost(level, message, context) {
  try {
    var input = { level: level, message: message };
    if (context) {
      input.context = context;
    }
    requestCapability("log", input);
  } catch (e) {
    // Best-effort: ignore errors from logging.
  }
}

/** Debug-level log. */
function logDebug(message) {
  logHost("debug", message, null);
}

/** Info-level log. */
function logInfo(message) {
  logHost("info", message, null);
}

/** Error-level log. */
function logError(message) {
  logHost("error", message, null);
}

/**
 * Write a file to the plugin's virtual storage via the `write_file` host
 * function.
 *
 * JS plugins have NO native filesystem — all I/O must go through host
 * functions. The write_file host function expects:
 *
 *   input  {"filename":"...","content":"..."}
 *   output {"success":true}
 *
 * @param {string} filename - Name (or relative path) of the file to write.
 * @param {string} content  - Contents to write.
 * @returns {boolean}        - true on success.
 * @throws {Error} If the host function is missing or reports an error.
 */
function writeStorageFile(filename, content) {
  const { write_file } = Host.getFunctions();
  if (!write_file) {
    throw new Error("Host did not provide 'write_file' function.");
  }

  const payload = JSON.stringify({ filename: filename, content: content });

  // Allocate memory for the request and call the host function.
  const memIn = Memory.fromString(payload);
  const memOutOffset = write_file(memIn.offset);
  memIn.free();

  // Read and parse the response.
  const memOut = Memory.find(memOutOffset);
  const response = JSON.parse(memOut.readString());

  if (response.error) {
    throw new Error("write_file error: " + response.error);
  }

  return true;
}

/**
 * Return the tool schema when the host asks the plugin to describe itself.
 *
 * @returns {object} Tool definition (mode, name, description, parameters).
 */
function defineTool() {
  logInfo("defineTool: building tool schema");
  return {
    mode: "define",
    name: "summarize",
    description:
      "Summarise a block of text using the system LLM model and save the result to storage.",
    parameters: {
      type: "object",
      properties: {
        query: {
          type: "string",
          description: "The text to summarise.",
        },
      },
      required: ["query"],
    },
  };
}

/**
 * Execute the tool logic.
 *
 * 1. Calls request_system_model with the query.
 * 2. Saves the model response to /storage/results.json via write_file.
 * 3. Reports progress via add_message_activity.
 * 4. Returns the tool execute response.
 *
 * @param {object} input - The full input object from the host.
 * @returns {object}     - Tool execute response.
 */
function executeTool(input) {
  logInfo("executeTool: starting");

  // Extract the tool arguments.
  var args = input.arguments || {};
  var query = args.query || "";

  logDebug("executeTool: query length: " + query.length);

  // 1. Request a summary from the system LLM model.
  logInfo("executeTool: calling request_system_model");
  var modelResponse = requestCapability("request_system_model", {
    request: "Summarise the following: " + query,
    schema: {
      type: "object",
      properties: {
        summary: { type: "string" },
      },
    },
  });

  logDebug("executeTool: model response received");
  logInfo("executeTool: model response parsed successfully");

  // 2. Save the result to /storage/results.json via the write_file host
  //    function. JS plugins have no native filesystem.
  logInfo("executeTool: writing results.json to storage");
  writeStorageFile("results.json", JSON.stringify(modelResponse));
  logInfo("executeTool: results.json written successfully");

  // 3. Extract the summary from the model response.
  var summary = "";
  if (modelResponse.result && modelResponse.result.summary) {
    summary = modelResponse.result.summary;
  }
  logDebug("executeTool: extracted summary length: " + summary.length);

  // 4. Report progress via add_message_activity.
  logInfo("executeTool: calling add_message_activity");
  var activityResponse = requestCapability("add_message_activity", {
    activity: {
      title: "Summary Generated",
      content: summary,
      origin: "tool-with-host-capability",
      icon: "FileText",
    },
  });
  logDebug("executeTool: add_message_activity response: " + activityResponse);

  // 5. Return the tool execute response.
  logInfo("executeTool: building final result");
  return {
    mode: "execute",
    result: {
      content: summary,
    },
  };
}

/**
 * Main entry point called by the Extism host.
 *
 * Reads the JSON input from the host, dispatches by "mode", and writes the
 * JSON response back via Host.outputString().
 *
 * @returns {number} 0 on success (convention for Extism PDK entry points).
 */
function summarize() {
  try {
    logInfo("summarize entry point called");

    // Read the JSON envelope provided by the host.
    // The host sends: {"input": {...}, "plugin_config": {...}}
    var inputStr = Host.inputString();
    var envelope = JSON.parse(inputStr);
    var input = envelope.input || {};

    logDebug("input mode: " + input.mode);

    // Dispatch based on the "mode" field.
    var result;
    switch (input.mode) {
      case "define":
        result = defineTool();
        break;
      case "execute":
        result = executeTool(input);
        break;
      default:
        throw new Error("Unknown mode: " + input.mode);
    }

    // Send the JSON response back to the host.
    Host.outputString(JSON.stringify(result));

    logInfo("summarize: returning success");
    return 0;
  } catch (err) {
    logError("summarize error: " + (err.message || String(err)));

    // On error, return a JSON error object so the host can surface it.
    Host.outputString(
      JSON.stringify({
        error: err.message || String(err),
      }),
    );
    // Re-throw so the Extism runtime records a non-zero exit.
    throw err;
  }
}

module.exports = { summarize };
