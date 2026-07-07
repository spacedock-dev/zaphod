// ABOUTME: Pipes one row per zellij invocation, broadcast by pipe name only —
// ABOUTME: never --plugin, which would launch a non-running plugin (SPEC #3).

package main

import (
	"encoding/json"
	"io"
	"os"
	"os/exec"
)

func pipeArgs(name, payload string) []string {
	return []string{"pipe", "--name", name, "--", payload}
}

// EmitRow pipes one row as a single JSON line in one argv token.
func EmitRow(cfg Config, row any, stderr io.Writer) error {
	payload, err := json.Marshal(row)
	if err != nil {
		return err
	}
	cmd := exec.Command(cfg.ZellijBin, pipeArgs(cfg.PipeName, string(payload))...)
	if cfg.ZellijSession != "" {
		cmd.Env = append(os.Environ(), "ZELLIJ_SESSION_NAME="+cfg.ZellijSession)
	}
	cmd.Stderr = stderr
	return cmd.Run()
}
