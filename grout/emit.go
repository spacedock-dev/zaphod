// ABOUTME: Pipes one row per zellij invocation, broadcast by pipe name only —
// ABOUTME: never --plugin, which would launch a non-running plugin (SPEC #3).

package main

import (
	"context"
	"encoding/json"
	"fmt"
	"io"
	"os/exec"
	"time"
)

func pipeArgs(name, payload string) []string {
	return []string{"pipe", "--name", name, "--", payload}
}

// pipeArgsForTab leaves delivery as a named-pipe broadcast while carrying the
// stable server tab ID for every receiver to verify. It never names a plugin:
// Zellij would launch an absent plugin for --plugin, which is not delivery.
func pipeArgsForTab(name, payload, recipientTabID string) []string {
	return []string{
		"pipe", "--name", name,
		"--args", "recipient-tab-id=" + recipientTabID,
		"--", payload,
	}
}

func zellijProfileArgs(cfg Config) []string {
	args := make([]string, 0, 8)
	if cfg.ZellijConfigDir != "" {
		args = append(args, "--config-dir", cfg.ZellijConfigDir)
	}
	if cfg.ZellijConfigFile != "" {
		args = append(args, "--config", cfg.ZellijConfigFile)
	}
	if cfg.ZellijDataDir != "" {
		args = append(args, "--data-dir", cfg.ZellijDataDir)
	}
	if cfg.ZellijSession != "" {
		args = append(args, "--session", cfg.ZellijSession)
	}
	return args
}

// EmitRow pipes one row as a single JSON line in one argv token,
// fire-and-forget: on PipeTimeout the child is killed and the timeout
// reported — no retry, no blocking beyond budget. WaitDelay makes Wait
// return even against a held stderr pipe.
func EmitRow(cfg Config, kind string, row any, stderr io.Writer) error {
	return emitRow(cfg, kind, row, "", stderr)
}

// EmitRowForTab is the private subscriber's one session-row delivery seam.
// It retains the existing JSON row protocol, but supplies the stable tab ID
// receiver guard on every otherwise-session-wide pipe broadcast.
func EmitRowForTab(
	cfg Config,
	kind string,
	row any,
	recipientTabID string,
	stderr io.Writer,
) error {
	return emitRow(cfg, kind, row, recipientTabID, stderr)
}

func emitRow(
	cfg Config,
	kind string,
	row any,
	recipientTabID string,
	stderr io.Writer,
) error {
	payload, err := json.Marshal(row)
	if err != nil {
		return err
	}
	ctx, cancel := context.WithTimeout(context.Background(), cfg.PipeTimeout)
	defer cancel()
	args := pipeArgs(cfg.PipeName, string(payload))
	if recipientTabID != "" {
		args = pipeArgsForTab(cfg.PipeName, string(payload), recipientTabID)
	}
	args = append(zellijProfileArgs(cfg), args...)
	cmd := exec.CommandContext(ctx, cfg.ZellijBin, args...)
	cmd.WaitDelay = 2 * time.Second
	cmd.Stderr = stderr
	err = cmd.Run()
	if ctx.Err() == context.DeadlineExceeded {
		return fmt.Errorf("pipe timeout after %s: kind=%s", cfg.PipeTimeout, kind)
	}
	return err
}
