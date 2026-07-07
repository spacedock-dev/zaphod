// ABOUTME: AC-1 wire-level check — one grout run emits exactly the two row kinds
// ABOUTME: as zellij pipe invocations on agent-event, plus the AC-3 kill timer.

package main

import (
	"bytes"
	"encoding/json"
	"os"
	"path/filepath"
	"strconv"
	"strings"
	"testing"
	"time"
)

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
		"state":   "awaiting_user",
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
