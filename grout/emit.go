// ABOUTME: Pipes one row per zellij invocation, broadcast by pipe name only —
// ABOUTME: never --plugin, which would launch a non-running plugin (SPEC #3).

package main

import (
	"context"
	"encoding/json"
	"fmt"
	"io"
	"os"
	"os/exec"
	"time"
)

func pipeArgs(name, payload string) []string {
	return []string{"pipe", "--name", name, "--", payload}
}

// EmitRow pipes one row as a single JSON line in one argv token,
// fire-and-forget: on PipeTimeout the child is killed and the timeout
// reported — no retry, no blocking beyond budget. WaitDelay makes Wait
// return even against a held stderr pipe.
func EmitRow(cfg Config, kind string, row any, stderr io.Writer) error {
	payload, err := json.Marshal(row)
	if err != nil {
		return err
	}
	ctx, cancel := context.WithTimeout(context.Background(), cfg.PipeTimeout)
	defer cancel()
	cmd := exec.CommandContext(ctx, cfg.ZellijBin, pipeArgs(cfg.PipeName, string(payload))...)
	cmd.WaitDelay = 2 * time.Second
	if cfg.ZellijSession != "" {
		cmd.Env = append(os.Environ(), "ZELLIJ_SESSION_NAME="+cfg.ZellijSession)
	}
	cmd.Stderr = stderr
	err = cmd.Run()
	if ctx.Err() == context.DeadlineExceeded {
		return fmt.Errorf("pipe timeout after %s: kind=%s", cfg.PipeTimeout, kind)
	}
	return err
}
