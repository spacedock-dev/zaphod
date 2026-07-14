// ABOUTME: AC-1 wire-level check — one grout run emits exactly the two row kinds
// ABOUTME: as zellij pipe invocations on agent-event, plus the AC-3 kill timer.

package main

import (
	"bytes"
	"context"
	"encoding/json"
	"io"
	"os"
	"path/filepath"
	"strconv"
	"strings"
	"syscall"
	"testing"
	"time"
)

func TestLeasedSnapshotCarriesGenerationAndPositiveAcknowledgment(t *testing.T) {
	dir := t.TempDir()
	argvLog := filepath.Join(dir, "argv")
	stdinLog := filepath.Join(dir, "stdin")
	zellij := writeScript(t, dir, "leased-zellij", "#!/bin/sh\n"+
		"{ echo \"$#\"; for a in \"$@\"; do printf '%s\\n' \"$a\"; done; } > "+argvLog+"\n"+
		"cat > "+stdinLog+"\necho accepted\n")
	cfg := Config{ZellijBin: zellij, ZellijSession: "managed", PipeTimeout: time.Second}
	paneID := uint32(7)
	rows := []SessionRow{{Kind: "session", ID: "codex:one", PaneID: &paneID}}
	if err := EmitLeasedSnapshotForTab(context.Background(), cfg, rows, "73", "token", "generation-a", 500*time.Millisecond, io.Discard); err != nil {
		t.Fatal(err)
	}
	argv := readInvocations(t, argvLog)
	if len(argv) != 1 {
		t.Fatalf("invocations = %d", len(argv))
	}
	joined := strings.Join(argv[0], " ")
	for _, want := range []string{"zaphod-agent-v1-token-snapshot", "recipient-tab-id=73", "recipient-token=token", "watch-generation=generation-a", "lease-ms=500"} {
		if !strings.Contains(joined, want) {
			t.Fatalf("leased snapshot argv omitted %q: %q", want, argv[0])
		}
	}
	payload, err := os.ReadFile(stdinLog)
	if err != nil {
		t.Fatal(err)
	}
	if !bytes.Contains(payload, []byte(`"pane_id":7`)) {
		t.Fatalf("leased snapshot payload = %s", payload)
	}
}

func TestLeaseHeartbeatIsOneUnacknowledgedNativeMessage(t *testing.T) {
	dir := t.TempDir()
	argvLog := filepath.Join(dir, "argv")
	zellij := writeScript(t, dir, "heartbeat-zellij", "#!/bin/sh\n"+
		"{ echo \"$#\"; for a in \"$@\"; do printf '%s\\n' \"$a\"; done; } > "+argvLog+"\n")
	cfg := Config{ZellijBin: zellij, ZellijSession: "managed", PipeTimeout: time.Second}
	if err := EmitLeaseHeartbeatForTab(context.Background(), cfg, "73", "token", "generation-a", 500*time.Millisecond, io.Discard); err != nil {
		t.Fatal(err)
	}
	argv := readInvocations(t, argvLog)
	if len(argv) != 1 {
		t.Fatalf("invocations = %d", len(argv))
	}
	joined := strings.Join(argv[0], " ")
	for _, want := range []string{"zaphod-agent-v1-token-heartbeat", "recipient-tab-id=73", "recipient-token=token", "watch-generation=generation-a", "lease-ms=500"} {
		if !strings.Contains(joined, want) {
			t.Fatalf("lease heartbeat argv omitted %q: %q", want, argv[0])
		}
	}
}

func writeScript(t *testing.T, dir, name, body string) string {
	t.Helper()
	path := filepath.Join(dir, name)
	if err := os.WriteFile(path, []byte(body), 0o755); err != nil {
		t.Fatal(err)
	}
	return path
}

// fakeZellij records each invocation's argv (count line, then one line
// per arg) to log. Compact JSON payloads carry no raw newlines, so
// line-per-arg is unambiguous.
func fakeZellij(t *testing.T, dir, log string) string {
	t.Helper()
	body := "#!/bin/sh\n{\necho \"$#\"\nfor a in \"$@\"; do printf '%s\\n' \"$a\"; done\n} >> " + log + "\n"
	return writeScript(t, dir, "zellij", body)
}

func readInvocations(t *testing.T, log string) [][]string {
	t.Helper()
	data, err := os.ReadFile(log)
	if err != nil {
		t.Fatalf("no zellij invocations recorded: %v", err)
	}
	lines := strings.Split(strings.TrimRight(string(data), "\n"), "\n")
	var invs [][]string
	for i := 0; i < len(lines); {
		n, err := strconv.Atoi(lines[i])
		if err != nil {
			t.Fatalf("argv log corrupt at line %d: %q", i, lines[i])
		}
		invs = append(invs, lines[i+1:i+1+n])
		i += 1 + n
	}
	return invs
}

func assertPipeInvocation(t *testing.T, argv []string) string {
	t.Helper()
	if len(argv) != 5 || argv[0] != "pipe" || argv[1] != "--name" || argv[2] != "agent-event" || argv[3] != "--" {
		t.Fatalf("argv = %q, want [pipe --name agent-event -- <payload>]", argv)
	}
	for _, a := range argv {
		if strings.HasPrefix(a, "--plugin") {
			t.Errorf("argv contains %q — broadcast by name only, never --plugin", a)
		}
	}
	return argv[4]
}

func TestEmitEndToEnd(t *testing.T) {
	dir := t.TempDir()
	argvLog := filepath.Join(dir, "zellij-argv.log")
	fixture, err := filepath.Abs("testdata/session-get.json")
	if err != nil {
		t.Fatal(err)
	}

	cfg := Config{
		AgentsviewBin:     writeScript(t, dir, "agentsview", "#!/bin/sh\ncat "+fixture+"\n"),
		SessionID:         "31dbb8ee-1d55-40ad-aa71-66c58790b708",
		GateLog:           "testdata/playground-gate.decisions.jsonl",
		ZellijBin:         fakeZellij(t, dir, argvLog),
		PipeName:          "agent-event",
		PipeTimeout:       5 * time.Second,
		SummaryClampBytes: 512,
	}
	var stderr bytes.Buffer
	if err := run(cfg, &stderr); err != nil {
		t.Fatalf("run: %v (stderr: %s)", err, stderr.String())
	}
	if stderr.Len() != 0 {
		t.Errorf("stderr not pristine: %s", stderr.String())
	}

	invs := readInvocations(t, argvLog)
	if len(invs) != 2 {
		t.Fatalf("zellij invoked %d times, want exactly 2", len(invs))
	}

	// Session row first, gate row second; field values are the fixtures'.
	var srow map[string]any
	if err := json.Unmarshal([]byte(assertPipeInvocation(t, invs[0])), &srow); err != nil {
		t.Fatalf("session payload: %v", err)
	}
	wantSession := map[string]any{
		"kind":    "session",
		"id":      "31dbb8ee-1d55-40ad-aa71-66c58790b708",
		"cwd":     "/Users/clkao/git/zaphod",
		"agent":   "claude",
		"state":   "blocked",
		"summary": "Wire the grout skeleton fixture",
	}
	for k, want := range wantSession {
		if srow[k] != want {
			t.Errorf("session row %s = %v, want %v", k, srow[k], want)
		}
	}

	var grow map[string]any
	if err := json.Unmarshal([]byte(assertPipeInvocation(t, invs[1])), &grow); err != nil {
		t.Fatalf("gate payload: %v", err)
	}
	absLog, err := filepath.Abs(cfg.GateLog)
	if err != nil {
		t.Fatal(err)
	}
	wantGate := map[string]any{
		"kind":           "gate",
		"log_path":       absLog,
		"workflow":       "demo",
		"entity":         "playground",
		"entity_title":   "Playground handoff demo",
		"stage":          "review",
		"round":          float64(1),
		"recommendation": "approve",
	}
	for k, want := range wantGate {
		if grow[k] != want {
			t.Errorf("gate row %s = %v, want %v", k, grow[k], want)
		}
	}
}

func TestPipeKillTimer(t *testing.T) {
	dir := t.TempDir()
	argvLog := filepath.Join(dir, "argv.log")
	pidLog := filepath.Join(dir, "pid.log")
	fixture, err := filepath.Abs("testdata/session-get.json")
	if err != nil {
		t.Fatal(err)
	}
	// Wedged zellij: records argv and its pid, then never exits. exec makes
	// $$ the sleep itself, so the recorded pid is the process grout must kill.
	wedged := writeScript(t, dir, "zellij",
		"#!/bin/sh\n{\necho \"$#\"\nfor a in \"$@\"; do printf '%s\\n' \"$a\"; done\n} >> "+argvLog+
			"\necho $$ >> "+pidLog+"\nexec sleep 300\n")

	cfg := Config{
		AgentsviewBin:     writeScript(t, dir, "agentsview", "#!/bin/sh\ncat "+fixture+"\n"),
		SessionID:         "31dbb8ee-1d55-40ad-aa71-66c58790b708",
		GateLog:           "testdata/playground-gate.decisions.jsonl",
		ZellijBin:         wedged,
		PipeName:          "agent-event",
		PipeTimeout:       time.Second,
		SummaryClampBytes: 512,
	}
	var stderr bytes.Buffer
	start := time.Now()
	err = run(cfg, &stderr)
	elapsed := time.Since(start)

	if budget := 2*cfg.PipeTimeout + 2*time.Second; elapsed > budget {
		t.Errorf("run took %s, want under %s", elapsed, budget)
	}
	if err == nil {
		t.Error("run succeeded, want an error (exit 1)")
	}
	for _, want := range []string{"pipe timeout after 1s: kind=session", "kind=gate"} {
		if !strings.Contains(stderr.String(), want) {
			t.Errorf("stderr missing %q:\n%s", want, stderr.String())
		}
	}

	// A timed-out session pipe must not stop the gate row.
	if invs := readInvocations(t, argvLog); len(invs) != 2 {
		t.Errorf("zellij invoked %d times, want both rows attempted", len(invs))
	}

	// No child left behind: kill -0 fails for every recorded pid.
	data, err := os.ReadFile(pidLog)
	if err != nil {
		t.Fatal(err)
	}
	pidLines := strings.Fields(string(data))
	if len(pidLines) != 2 {
		t.Fatalf("recorded %d child pids, want 2", len(pidLines))
	}
	for _, s := range pidLines {
		pid, err := strconv.Atoi(s)
		if err != nil {
			t.Fatal(err)
		}
		if err := syscall.Kill(pid, 0); err != syscall.ESRCH {
			t.Errorf("child %d still alive (kill -0 err = %v)", pid, err)
		}
	}
}
