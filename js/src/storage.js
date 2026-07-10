/**
 * storage.js
 *
 * Filesystem helpers for JS/WASM plugins.
 *
 * CRITICAL: JavaScript running inside a WASM module (via the Extism JS PDK /
 * QuickJS runtime) has **no native filesystem access**.  All file operations
 * must go through host functions provided in the `extism:host/user` namespace:
 *
 *   read_file   — input {"filename":"..."} → {"success":true,"content":"..."}
 *   write_file  — input {"filename":"...","content":"..."} → {"success":true}
 *   list_files  — input {"path":"..."} → {"success":true,"files":[...]}
 *   delete_file — input {"filename":"..."} → {"success":true}
 *
 * On failure each host function returns an object with an `error` field instead.
 */

/**
 * Generic helper: call a storage host function and parse the JSON response.
 *
 * @param {string} fnName  - Name of the host function to call
 *                            ("read_file", "write_file", …).
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
  const responseStr = memOut.readString();
  return JSON.parse(responseStr);
}

/**
 * Read a file from the plugin's virtual storage.
 *
 * @param {string} filename - Name (or relative path) of the file to read.
 * @returns {string}         - File contents as a UTF-8 string.
 * @throws {Error} If the file does not exist or the host reports an error.
 */
function readFile(filename) {
  const result = callStorageFn("read_file", { filename: filename });
  if (result.error) {
    throw new Error("read_file error: " + result.error);
  }
  return result.content;
}

/**
 * Write a file to the plugin's virtual storage.
 *
 * @param {string} filename - Name (or relative path) of the file to write.
 * @param {string} content  - Contents to write.
 * @throws {Error} If the host reports an error.
 */
function writeFile(filename, content) {
  const result = callStorageFn("write_file", {
    filename: filename,
    content: content,
  });
  if (result.error) {
    throw new Error("write_file error: " + result.error);
  }
  return true;
}

/**
 * List files under a given path in the plugin's virtual storage.
 *
 * @param {string} path - Directory path to list.
 * @returns {string[]}   - Array of file names / paths.
 * @throws {Error} If the host reports an error.
 */
function listFiles(path) {
  const result = callStorageFn("list_files", { path: path });
  if (result.error) {
    throw new Error("list_files error: " + result.error);
  }
  return result.files || [];
}

/**
 * Delete a file from the plugin's virtual storage.
 *
 * @param {string} filename - Name (or relative path) of the file to delete.
 * @throws {Error} If the host reports an error.
 */
function deleteFile(filename) {
  const result = callStorageFn("delete_file", { filename: filename });
  if (result.error) {
    throw new Error("delete_file error: " + result.error);
  }
  return true;
}

module.exports = {
  readFile,
  writeFile,
  listFiles,
  deleteFile,
};
