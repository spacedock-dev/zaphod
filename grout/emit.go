// ABOUTME: Pipes one row per zellij invocation, broadcast by pipe name only —
// ABOUTME: never --plugin, which would launch a non-running plugin (SPEC #3).

package main

import (
	"bytes"
	"context"
	"encoding/json"
	"fmt"
	"io"
	"os/exec"
	"strings"
	"time"
)

func pipeArgs(name, payload string) []string {
	return []string{"pipe", "--name", name, "--", payload}
}

func privateAgentPipeName(recipientToken, kind string) string {
	return "zaphod-agent-v1-" + recipientToken + "-" + kind
}

// pipeArgsForTab leaves delivery as a named-pipe broadcast while carrying the
// stable server tab ID for every receiver to verify. It never names a plugin:
// Zellij would launch an absent plugin for --plugin, which is not delivery.
func pipeArgsForTab(name, payload, recipientTabID, recipientToken string) []string {
	return []string{
		"pipe", "--name", name,
		"--args", "recipient-tab-id=" + recipientTabID + ",recipient-token=" + recipientToken,
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

// EmitRowForTab is the private watcher's acknowledged session-row seam.
// The exact stable-tab receiver replies only after accepting the JSON row;
// an unacknowledged successful broadcast is retried within PipeTimeout.
func EmitRowForTab(
	ctx context.Context,
	cfg Config,
	kind string,
	row any,
	recipientTabID string,
	recipientToken string,
	stderr io.Writer,
) error {
	payload, err := json.Marshal(row)
	if err != nil {
		return err
	}
	args := pipeArgsForTab(privateAgentPipeName(recipientToken, "event"), string(payload), recipientTabID, recipientToken)
	return emitAcknowledged(ctx, cfg, kind, args, "", stderr)
}

// EmitSnapshotForTab sends the complete initial snapshot through stdin in one
// bounded CLI invocation, avoiding both argv limits and per-row startup cost.
func EmitSnapshotForTab(
	ctx context.Context,
	cfg Config,
	rows []SessionRow,
	recipientTabID string,
	recipientToken string,
	stderr io.Writer,
) error {
	payload, err := json.Marshal(rows)
	if err != nil {
		return err
	}
	args := []string{
		"pipe", "--name", privateAgentPipeName(recipientToken, "snapshot"),
		"--args", "recipient-tab-id=" + recipientTabID + ",recipient-token=" + recipientToken,
	}
	return emitAcknowledged(ctx, cfg, "snapshot", args, string(payload), stderr)
}

func EmitLeasedSnapshotForTab(
	ctx context.Context,
	cfg Config,
	rows []SessionRow,
	recipientTabID string,
	recipientToken string,
	generation string,
	lease time.Duration,
	stderr io.Writer,
) error {
	if generation == "" || strings.ContainsAny(generation, ",=") {
		return fmt.Errorf("invalid watch generation")
	}
	leaseMS := lease.Milliseconds()
	if leaseMS < 100 || leaseMS > 2500 {
		return fmt.Errorf("watch lease must be between 100ms and 2500ms")
	}
	payload, err := json.Marshal(rows)
	if err != nil {
		return err
	}
	args := []string{
		"pipe", "--name", privateAgentPipeName(recipientToken, "snapshot"),
		"--args", fmt.Sprintf("recipient-tab-id=%s,recipient-token=%s,watch-generation=%s,lease-ms=%d",
			recipientTabID, recipientToken, generation, leaseMS),
	}
	return emitAcknowledged(ctx, cfg, "leased-snapshot", args, string(payload), stderr)
}

// EmitLeaseHeartbeatForTab renews an already-accepted generation without
// waiting for plugin output. A busy plugin may process the heartbeat late and
// let the lease expire, but it cannot hold this native client open and
// multiply congestion for unrelated pane/tab actions.
func EmitLeaseHeartbeatForTab(
	ctx context.Context,
	cfg Config,
	recipientTabID string,
	recipientToken string,
	generation string,
	lease time.Duration,
	stderr io.Writer,
) error {
	if generation == "" || strings.ContainsAny(generation, ",=") {
		return fmt.Errorf("invalid watch generation")
	}
	leaseMS := lease.Milliseconds()
	if leaseMS < 100 || leaseMS > 2500 {
		return fmt.Errorf("watch lease must be between 100ms and 2500ms")
	}
	args := []string{
		"pipe", "--name", privateAgentPipeName(recipientToken, "heartbeat"),
		"--args", fmt.Sprintf("recipient-tab-id=%s,recipient-token=%s,watch-generation=%s,lease-ms=%d",
			recipientTabID, recipientToken, generation, leaseMS),
	}
	return emitUnacknowledged(ctx, cfg, "lease-heartbeat", args, stderr)
}

func emitUnacknowledged(
	ctx context.Context,
	cfg Config,
	kind string,
	pipeArgs []string,
	stderr io.Writer,
) error {
	deliveryCtx, cancel := context.WithTimeout(ctx, cfg.PipeTimeout)
	defer cancel()
	args := append(zellijProfileArgs(cfg), pipeArgs...)
	cmd := exec.CommandContext(deliveryCtx, cfg.ZellijBin, args...)
	cmd.WaitDelay = 2 * time.Second
	cmd.Stderr = stderr
	err := cmd.Run()
	if deliveryCtx.Err() != nil {
		return fmt.Errorf("pipe timeout after %s: kind=%s", cfg.PipeTimeout, kind)
	}
	return err
}

func emitAcknowledged(
	ctx context.Context,
	cfg Config,
	kind string,
	pipeArgs []string,
	stdinPayload string,
	stderr io.Writer,
) error {
	deliveryCtx, cancel := context.WithTimeout(ctx, cfg.PipeTimeout)
	defer cancel()
	args := append(zellijProfileArgs(cfg), pipeArgs...)
	for {
		var stdout bytes.Buffer
		cmd := exec.CommandContext(deliveryCtx, cfg.ZellijBin, args...)
		cmd.WaitDelay = 2 * time.Second
		if stdinPayload != "" {
			cmd.Stdin = strings.NewReader(stdinPayload)
		}
		cmd.Stdout = &stdout
		cmd.Stderr = stderr
		err := cmd.Run()
		if err == nil && strings.TrimSpace(stdout.String()) == "accepted" {
			return nil
		}
		if deliveryCtx.Err() != nil {
			return fmt.Errorf("pipe timeout after %s without recipient acknowledgment: kind=%s", cfg.PipeTimeout, kind)
		}
		if err != nil {
			return err
		}
		retry := time.NewTimer(50 * time.Millisecond)
		select {
		case <-deliveryCtx.Done():
			retry.Stop()
			return fmt.Errorf("pipe timeout after %s without recipient acknowledgment: kind=%s", cfg.PipeTimeout, kind)
		case <-retry.C:
		}
	}
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
		args = pipeArgsForTab(cfg.PipeName, string(payload), recipientTabID, "")
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
