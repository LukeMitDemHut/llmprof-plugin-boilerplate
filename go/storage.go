package main

// storage.go — Filesystem helpers for reading and writing plugin data.
//
// Go WASM plugins run under the WASI preview-1 ABI and have access to a
// native filesystem. The LLMProf host mounts a per-plugin directory at
// `/storage`. Use the standard `os` package — no host functions are needed.
//
// All helpers in this file enforce path safety: they reject absolute paths
// and `..` traversal segments so a plugin cannot escape the /storage sandbox.

import (
	"fmt"
	"os"
	"path/filepath"
	"strings"
)

// StorageRoot is the mount point the host provides for plugin-private data.
const StorageRoot = "/storage"

// resolvePath joins the given filename onto StorageRoot and verifies the
// resulting path stays within the sandbox.
//
// It rejects:
//   - absolute paths (e.g. "/etc/passwd")
//   - paths containing ".." traversal segments
//
// The returned path is always absolute and rooted under StorageRoot.
func resolvePath(filename string) (string, error) {
	if filepath.IsAbs(filename) {
		return "", fmt.Errorf("absolute paths are not allowed: %q", filename)
	}

	// Normalise the user-supplied path and check every segment for "..".
	cleaned := filepath.Clean(filename)
	for _, segment := range strings.Split(cleaned, string(filepath.Separator)) {
		if segment == ".." {
			return "", fmt.Errorf("path traversal (..) is not allowed: %q", filename)
		}
	}

	return filepath.Join(StorageRoot, cleaned), nil
}

// ReadFile reads the contents of a file relative to /storage and returns it
// as a string.
func ReadFile(filename string) (string, error) {
	path, err := resolvePath(filename)
	if err != nil {
		return "", err
	}

	data, err := os.ReadFile(path)
	if err != nil {
		return "", fmt.Errorf("failed to read %q: %w", filename, err)
	}

	return string(data), nil
}

// WriteFile writes content to a file relative to /storage. Parent directories
// are created automatically with os.MkdirAll.
func WriteFile(filename string, content string) error {
	path, err := resolvePath(filename)
	if err != nil {
		return err
	}

	// Ensure the parent directory exists.
	dir := filepath.Dir(path)
	if err := os.MkdirAll(dir, 0o755); err != nil {
		return fmt.Errorf("failed to create directories for %q: %w", filename, err)
	}

	if err := os.WriteFile(path, []byte(content), 0o644); err != nil {
		return fmt.Errorf("failed to write %q: %w", filename, err)
	}

	return nil
}

// ListFiles recursively lists all files under the given path relative to
// /storage. Pass an empty string to list everything in /storage.
//
// Returned paths are relative to /storage.
func ListFiles(path string) ([]string, error) {
	root, err := resolvePath(path)
	if err != nil {
		return nil, err
	}

	var files []string

	err = filepath.Walk(root, func(currentPath string, info os.FileInfo, walkErr error) error {
		if walkErr != nil {
			return walkErr
		}
		if info.IsDir() {
			return nil
		}

		// Convert back to a path relative to StorageRoot for the caller.
		rel, relErr := filepath.Rel(StorageRoot, currentPath)
		if relErr != nil {
			return relErr
		}
		files = append(files, rel)
		return nil
	})
	if err != nil {
		return nil, fmt.Errorf("failed to list %q: %w", path, err)
	}

	return files, nil
}

// DeleteFile removes a file relative to /storage. If the file does not exist
// the error is returned as-is from os.Remove.
func DeleteFile(filename string) error {
	path, err := resolvePath(filename)
	if err != nil {
		return err
	}

	if err := os.Remove(path); err != nil {
		return fmt.Errorf("failed to delete %q: %w", filename, err)
	}

	return nil
}