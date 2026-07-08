// ABOUTME: Row structs and pure builders for the two agent-event row kinds
// ABOUTME: pinned by plan decision 3, plus the rune-safe summary clamp.

package main

import (
	"time"
	"unicode/utf8"
)

type SessionRow struct {
	Kind    string `json:"kind"`
	ID      string `json:"id"`
	Cwd     string `json:"cwd"`
	Agent   string `json:"agent"`
	State   string `json:"state"`
	Summary string `json:"summary"`
	TS      string `json:"ts"`
}

type GateRow struct {
	Kind           string `json:"kind"`
	LogPath        string `json:"log_path"`
	Workflow       string `json:"workflow"`
	Entity         string `json:"entity"`
	EntityTitle    string `json:"entity_title"`
	Stage          string `json:"stage"`
	Round          int    `json:"round"`
	Recommendation string `json:"recommendation"`
	TS             string `json:"ts"`
}

func BuildSessionRow(si sessionInfo, now time.Time, clampBytes int) SessionRow {
	return SessionRow{
		Kind:    "session",
		ID:      si.ID,
		Cwd:     si.Cwd,
		Agent:   si.Agent,
		State:   MapSessionState(si.TerminationStatus, lastActivity(si), now),
		Summary: clampSummary(si.FirstMessage, clampBytes),
		TS:      now.UTC().Format(time.RFC3339),
	}
}

func BuildGateRow(gi gateInfo, now time.Time) GateRow {
	return GateRow{
		Kind:           "gate",
		LogPath:        gi.LogPath,
		Workflow:       gi.Workflow,
		Entity:         gi.Entity,
		EntityTitle:    gi.EntityTitle,
		Stage:          gi.Stage,
		Round:          gi.Round,
		Recommendation: gi.Verdict,
		TS:             now.UTC().Format(time.RFC3339),
	}
}

// clampSummary truncates s to at most maxBytes without splitting a rune.
func clampSummary(s string, maxBytes int) string {
	if len(s) <= maxBytes {
		return s
	}
	cut := maxBytes
	for cut > 0 && !utf8.RuneStart(s[cut]) {
		cut--
	}
	return s[:cut]
}
