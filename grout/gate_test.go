// ABOUTME: Verifies brief-path derivation from a decision log and the
// ABOUTME: frontmatter-to-gate-row mapping against the vendored playground brief.

package main

import (
	"path/filepath"
	"testing"
	"time"
)

func TestGateRowFromBrief(t *testing.T) {
	// Inverse of subspace's DecisionLogPath derivation
	// (gate-foo.md → gate-foo.decisions.jsonl).
	if got := briefPathForLog("/x/gate-foo.decisions.jsonl"); got != "/x/gate-foo.md" {
		t.Errorf("briefPathForLog = %q, want /x/gate-foo.md", got)
	}

	gi, err := GateFromLog("testdata/playground-gate.decisions.jsonl")
	if err != nil {
		t.Fatalf("GateFromLog: %v", err)
	}
	if !filepath.IsAbs(gi.LogPath) {
		t.Errorf("LogPath = %q, want absolute", gi.LogPath)
	}
	wantAbs, err := filepath.Abs("testdata/playground-gate.decisions.jsonl")
	if err != nil {
		t.Fatal(err)
	}
	if gi.LogPath != wantAbs {
		t.Errorf("LogPath = %q, want %q", gi.LogPath, wantAbs)
	}

	// Field baselines are the vendored brief's frontmatter values.
	now := time.Date(2026, 7, 7, 5, 0, 0, 0, time.UTC)
	row := BuildGateRow(gi, now)
	want := GateRow{
		Kind:           "gate",
		LogPath:        wantAbs,
		Workflow:       "demo",
		Entity:         "playground",
		EntityTitle:    "Playground handoff demo",
		Stage:          "review",
		Round:          1,
		Recommendation: "approve",
		TS:             "2026-07-07T05:00:00Z",
	}
	if row != want {
		t.Errorf("gate row = %+v, want %+v", row, want)
	}
}
