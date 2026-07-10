package main

import (
	"fmt"
	"os"
	"time"
)

// appendLog opens /storage/lifecycle.log in append mode and writes a single
// line. The /storage directory persists across upgrades, so entries written
// by on_before_upgrade (old wasm) are visible to on_after_upgrade (new wasm).
func appendLog(entry string) {
	// Ensure /storage exists (only needed on first install, but harmless otherwise).
	if err := os.MkdirAll("/storage", 0755); err != nil {
		fmt.Fprintf(os.Stderr, "lifecycle-hooks: failed to create /storage: %v\n", err)
		return
	}

	f, err := os.OpenFile("/storage/lifecycle.log", os.O_APPEND|os.O_CREATE|os.O_WRONLY, 0644)
	if err != nil {
		fmt.Fprintf(os.Stderr, "lifecycle-hooks: failed to open log: %v\n", err)
		return
	}
	defer f.Close()

	if _, err := f.WriteString(entry + "\n"); err != nil {
		fmt.Fprintf(os.Stderr, "lifecycle-hooks: failed to write log: %v\n", err)
	}
}

// on_install is called once when the plugin is first installed.
// Good for initializing storage, creating config files, seeding data.
//
//go:wasmexport on_install
func onInstall() int32 {
	appendLog(fmt.Sprintf("installed at %s", time.Now().UTC().Format(time.RFC3339)))
	return 0
}

// on_before_upgrade is called on the OLD wasm before files are swapped.
// Good for backing up data, flushing state, writing migration markers.
//
//go:wasmexport on_before_upgrade
func onBeforeUpgrade() int32 {
	appendLog(fmt.Sprintf("before upgrade at %s", time.Now().UTC().Format(time.RFC3339)))
	return 0
}

// on_after_upgrade is called on the NEW wasm after files are swapped.
// Good for migrating data formats, verifying integrity, cleaning up backups.
//
//go:wasmexport on_after_upgrade
func onAfterUpgrade() int32 {
	appendLog(fmt.Sprintf("after upgrade at %s", time.Now().UTC().Format(time.RFC3339)))
	return 0
}

// on_uninstall is called when the plugin is removed.
// Good for cleanup — note that /storage may be removed after this returns.
//
//go:wasmexport on_uninstall
func onUninstall() int32 {
	appendLog(fmt.Sprintf("uninstalled at %s", time.Now().UTC().Format(time.RFC3339)))
	return 0
}

// ping is a dummy no-op capability required by the host's manifest validation
// (capabilities must be a non-empty list). This plugin is primarily a
// lifecycle-hooks demonstration; ping simply returns 0.
//
//go:wasmexport ping
func ping() int32 {
	return 0
}

func main() {}