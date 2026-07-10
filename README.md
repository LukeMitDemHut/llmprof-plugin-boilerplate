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

The Extism host is documented in detail in the main LLMProf MkDocs site. See the getting started there.
