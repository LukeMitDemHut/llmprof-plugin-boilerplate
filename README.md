# LLMProf Plugin Boilerplate

Templates for building WASM plugins for the LLMProf Extism host.

The LLMProf plugin system runs WebAssembly (WASM) plugins compiled to the
`wasm32-wasip1` target via [Extism](https://extism.org/). This boilerplate gives
plugin developers everything they need to get started in three languages:
**Go**, **Rust**, and **JavaScript**.

Each language template includes a minimal plugin, a `manifest.json`, and a
`build` script that produces a `plugin.wasm` ready to deploy to a running
LLMProf instance.
Additional examples are available.

Authoritative sources (the host implementation in the LLMProf repo is ground
truth):

- **Capabilities & payload contracts:** `mkdocs/docs/plugins/capabilities.md`
- **Manifest reference:** `mkdocs/docs/plugins/extism/manifest-reference.md`
- **Store layout & deployment:** `mkdocs/docs/plugins/extism/connecting-to-host.md`
- **Host implementation:** `plugins/host/` in the LLMProf repo

---

## Repository layout

```
js/            JS (Extism JS PDK / QuickJS) template — no native filesystem;
               storage goes through host functions
go/            Go template — WASI filesystem access via std / os
rust/          Rust template — WASI filesystem access via std::fs
examples/      complete working plugins of each capability type
manifest.schema.json       JSON Schema for manifest.json
host-capabilities.json     reference for callable host capabilities
```

### Examples

| Example | Demonstrates |
|---|---|
| `lifecycle-hooks` | All 4 lifecycle hooks (`on_install`, `on_before_upgrade`, `on_after_upgrade`, `on_uninstall`) |
| `requesting-external-data` | Outbound HTTP requests during `on_install` (`allowed_hosts`) |
| `tool-with-host-capability` | `tool` capability calling `request_system_model`, `add_message_activity`, storage |
| `student-support-strategy` | `student_support` capability with the strict output schema |

---

## Manifest requirements

`manifest.json` is validated strictly by the host (`host/manifest.py` in the
LLMProf repo). See `manifest.schema.json` in this repo for the full shape.

Required store metadata:

```json
{
  "name": "my-plugin",
  "version": "1.0.0",
  "pluginApiVersionRequirement": ">=1.0.0",
  "author": "You",
  "display_name": { "en": "My Plugin", "de": "Mein Plugin" },
  "description": { "en": "Does something useful.", "de": "Tut etwas Nützliches." },
  "capabilities": [ ... ],
  "host_capabilities": [ ... ]
}
```

Rules that are easy to get wrong:

- **`display_name` is required** and must be a localized map
  (`{en, de, ...}`). Missing or empty `display_name` fails the whole
  provider's listing validation — *every* plugin from that host disappears
  from the plugin store, not just yours.
- **Localize `description` too.** A plain string is tolerated (wrapped as
  `{"en": <text>}`), but users on the English locale then see German text.
  Localized values are capped at **500 chars per entry, max 10 locales**.
- **`execution_timeout_ms` is required and positive** for every capability
  and host capability; the ceiling is `PLUGIN_MAX_EXECUTION_MS`
  (default 300000 ms). Missing values fail validation at load time (and, at
  discovery, silently drop the plugin version).
- **Capability ids must be unique.**
- **`configuration_schema: {}`** (the empty object) means "accept any
  configuration object" — that's a valid minimal choice.
- **Lifecycle hooks** must be declared as
  `"lifecycle": {"on_install": "<export fn>", ...}` if you want
  install/upgrade/uninstall callbacks.

## Capability contracts

The host dispatches purely on the manifest: it calls the exported function
named in `capabilities[].execute` — it does **not** parse your input envelope
and route by capability type.

- **One exported function per capability**, each referenced by its own
  `execute` field. Reusing one export for two capabilities (e.g. pointing a
  `student_support` capability at a `tool` export) breaks: the tool entry
  falls into its `mode` dispatch, sees an unexpected input shape, and throws
  `"Unknown mode: undefined"`. If you want shared logic, have both exports
  call a common internal function — exactly what the templates do.

### `tool`

The host calls your export twice per usage:

| Call | Input | Output |
|---|---|---|
| Definition | `{"mode":"define"}` | `{mode, name, description, parameters}` |
| Execution | `{"mode":"execute","arguments":{...}}` | `{mode, result:{content, ui?}}` |

Gotchas:

- **`result.content` is mandatory and non-empty.** There is no
  `success:false` error convention on the wire — if you validate arguments
  and reject them, return the error *as the content string* (the LLM reads
  it and can recover, e.g. retry with a valid parameter). A response of
  `{"result":{"success":false,"error":"..."}}` without `content` is thrown
  away by the host and replaced with a generic "returned no valid result
  payload" — your carefully worded error never reaches the user. The
  `tool-with-host-capability` example shows the correct error path.
- `parameters` needs at least one property; cast empty maps to objects
  (`{}` not `[]`) in JSON.
- Your configuration arrives separately as `plugin_config` (2nd envelope
  key) — never declare configuration as tool parameters.
- Optional extras: `result.ui.sources[]` for document attribution,
  `result.ui.applet_id` to trigger a follow-up `mode:"applet"` call for
  rendering HTML in the chat. If you use them, the `applet` mode must also
  be handled in your dispatch.

### `student_support`

Input is `{"locale": "en"}` — that's all you get (run-all discovery and
prompt resolution both use the same envelope). The output is a **strictly
validated single object**:

| Field | Required | Constraint | Note |
|---|---|---|---|
| `name` | yes | ≤ 40 chars | Human-readable, localized. **Not** your capability id — the dropdown displays it. |
| `description` | yes | ≤ 100 chars | Short localized strategy description. |
| `icon` | yes | `IconType` enum case name | e.g. `"ChalkboardTeacher"`. Invalid/missing values fall back to the default icon. |
| `prompt` | yes | ≤ 10000 chars | The strategy's system prompt, injected verbatim into the system prompt — keep it a *prompt*, not a strategy description. |

Do **not** return extra keys (`title`, `mode`, `locale`, `success`, …) — the
schema is `additionalProperties: false`. Honor the `locale`: return at
minimum the `name`/`description` localized per the requested locale; a
de-only plugin should still fall back to English keys rather than echoing
the locale back unchanged.

## Host capabilities

Every host capability you call must be declared in the manifest:

```json
"host_capabilities": [
  { "id": "log", "execution_timeout_ms": 3000 }
]
```

Undeclared calls return `{"error": "capability not allowed"}` immediately.
Currently available ids: `log`, `add_message_activity`, `resolve_context`,
`resolve_message`, `request_user_i18n`, `request_system_model` — see
`host-capabilities.json` for their input/output schemas.

The always-available storage host functions (`read_file`, `write_file`,
`list_files`, `delete_file`) are separate — they don't need manifest
entries. `request-system-model` calls additionally need a real execution
token (a chat-driven invocation, or the
`app:execute-plugin-capability --message-id` test route).

## Deployment & verification

1. Build: `./build` in the language folder → produces `plugin.wasm`.
2. Copy to the store **as a version directory** — the host discovers plugins
   with the layout `<plugin-name>/<version>/`; a **flat** directory is
   **silently skipped** (no error, no log entry — the plugin never appears):

   ```
   plugins/plugins/
   └── my-plugin/
       └── 1.0.0/              ← directory name = manifest "version"
           ├── manifest.json
           └── plugin.wasm
   ```

   ```bash
   cp manifest.json plugin.wasm /path/to/llmprof/plugins/plugins/<name>/<version>/
   ```

3. Verify discovery (live rescan, no restart needed):

   ```bash
   curl -s -H "Authorization: Bearer <token>" http://localhost:8012/v0/plugins | jq '.plugins[].name'
   ```

4. Install through the webapp plugin store UI at
   `/plugins/store?scope=context&target=<context-uuid>` or:

   ```bash
   ./dev console app:manage-plugin-installation install \
     --provider=extism --plugin=<name> --plugin-version=<version> \
     --scope=context --scope-target=<context-uuid>
   ```

5. Exercise the capability end-to-end:

   ```bash
   ./dev console app:execute-plugin-capability <install-uuid> <capability-id> --mode=define
   ./dev console app:execute-plugin-capability <install-uuid> <capability-id> --mode=execute --arguments='{"number":2}'
   ```

6. If you redeployed the *same version* after rebuilding: restart the host
   container and clear the execute cache
   (`app:plugin-cache:clear-installation <install-uuid>`) — otherwise you
   test stale output.

## Version hygiene

- **Bump `version` for every change** and create a **new** version
  directory; never edit a deployed version in place (execute cache +
  install copies).
- Keep the dev-folder name, manifest `name`, and store folder name aligned
  (e.g. developing `eidp-abnahmen-lernstil` while the manifest says
  `eidp-abnahmesimulation` works, but invites mistakes — the store and
  installs key off the manifest name).
- Old version directories can remain for existing installs; upgrades are
  explicit (webapp/console), never implied by file replacement.

---

The Extism host is documented in detail in the main LLMProf MkDocs site. See the getting started there.