// ABOUTME: Verifies the recorded `agentsview session get --format json` fixture
// ABOUTME: decodes and maps onto a session row with the pinned source fields.

package main

import (
	"os"
	"testing"
	"time"
)

func TestSessionRowFromFixture(t *testing.T) {
	f, err := os.Open("testdata/session-get.json")
	if err != nil {
		t.Fatal(err)
	}
	defer f.Close()
	si, err := decodeSession(f)
	if err != nil {
		t.Fatalf("decodeSession: %v", err)
	}

	// Field baselines are the recorded fixture's values (agentsview v0.36.1).
	now := time.Date(2026, 7, 7, 5, 0, 0, 0, time.UTC)
	row := BuildSessionRow(si, now, 512)
	want := SessionRow{
		Kind:    "session",
		ID:      "31dbb8ee-1d55-40ad-aa71-66c58790b708",
		Cwd:     "/Users/clkao/git/zaphod",
		Agent:   "claude",
		State:   "awaiting_user",
		Summary: "Wire the grout skeleton fixture",
		TS:      "2026-07-07T05:00:00Z",
	}
	if row != want {
		t.Errorf("session row = %+v, want %+v", row, want)
	}
}
