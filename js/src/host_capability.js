/**
 * host_capability.js
 *
 * Helpers for calling LLMProf host capabilities from a JS/WASM plugin.
 *
 * The host exposes a single gateway function — `call_host_capability` — in the
 * `extism:host/user` namespace.  Every higher-level capability (adding message
 * activities, requesting LLM completions, fetching i18n strings, …) is invoked
 * by sending a JSON envelope through this gateway:
 *
 *   {
 *     "capability": "<capabilityName>",
 *     "input": { ...capability-specific payload... }
 *   }
 *
 * The host returns a JSON string that the caller parses.
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
  // Obtain the host function table.  In the Extism JS PDK this is the only
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

  // Call the host gateway.  It returns an offset to a memory block containing
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

/**
 * Send a log entry to the host's `log` host capability.
 * If the host does not support the `log` capability the call is silently
 * ignored — logging must never break plugin execution.
 *
 * @param {string} level   - Log level: "debug", "info", "warn", "error".
 * @param {string} message - Log message.
 */
function logHost(level, message) {
  try {
    requestCapability("log", { level: level, message: message });
  } catch (e) {
    // Best-effort: ignore errors from logging.
  }
}

/** Convenience wrapper: log at debug level. */
function logDebug(message) {
  logHost("debug", message);
}

/** Convenience wrapper: log at info level. */
function logInfo(message) {
  logHost("info", message);
}

/** Convenience wrapper: log at error level. */
function logError(message) {
  logHost("error", message);
}

/**
 * Add a message activity entry via the `add_message_activity` host capability.
 *
 * The message is resolved automatically from the execution token's
 * `message_id` claim — no key or message reference needs to be passed.
 *
 * @param {object} activity - The activity payload.
 * @returns {object}        - The host's JSON response.
 */
function addMessageActivity(activity) {
  return requestCapability("add_message_activity", { activity: activity });
}

/**
 * Resolve the current context for this plugin execution.
 *
 * The context is derived from the execution token's `message_id` claim
 * (message → chat → context). Returns the context UUID and name, or `null`
 * when no message is available.
 *
 * @returns {object} The host's JSON response containing
 *                   `{context: {uuid: "...", name: "..."}}` or
 *                   `{context: null}`.
 */
function resolveContext() {
  return requestCapability("resolve_context", {});
}

/**
 * Resolve the current message for this plugin execution.
 *
 * The message is identified by the `message_id` claim in the execution token.
 * Returns the message ID, or `null` when no message is available.
 *
 * @returns {object} The host's JSON response containing
 *                   `{message: {id: 42}}` or `{message: null}`.
 */
function resolveMessage() {
  return requestCapability("resolve_message", {});
}

/**
 * Request a completion from the system-configured LLM model via the
 * `request_system_model` host capability.
 *
 * NOTE: `request` is a plain prompt string — the host builds the messages
 * array internally as a single user message. Pass a `messages` array instead
 * for multi-turn control. Calls additionally require a real execution token
 * (i.e. a chat-driven invocation or the
 * `app:execute-plugin-capability --message-id` test route), and the
 * capability must be declared in the manifest's host_capabilities.
 *
 * @param {string} request - The plain-text prompt for the model.
 * @param {object} [schema] - Optional JSON Schema for structured output
 *                            (also accepted by the host as `response_schema`).
 * @returns {object}       - The host's JSON response.
 */
function requestSystemModel(request, schema) {
  var input = { request: request };
  if (schema) {
    input.schema = schema;
  }
  return requestCapability("request_system_model", input);
}

/**
 * Request the user's i18n / locale preference via the `request_user_i18n`
 * host capability.
 *
 * @returns {object} The host's JSON response containing
 *                   `{locale: "en"}` or `{locale: null}`.
 */
function requestUserI18n() {
  return requestCapability("request_user_i18n", {});
}

module.exports = {
  callHostCapability,
  requestCapability,
  addMessageActivity,
  resolveContext,
  resolveMessage,
  requestSystemModel,
  requestUserI18n,
  logHost,
  logDebug,
  logInfo,
  logError,
};
