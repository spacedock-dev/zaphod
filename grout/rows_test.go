// ABOUTME: Pins the emitted row lines to plan decision 3 (docs/plan-agent-rail.md):
// ABOUTME: exact key sets per kind, round a JSON number, ts RFC3339 UTC, 512-byte clamp.

package main

import (
	"encoding/json"
	"strings"
	"testing"
	"time"
	"unicode/utf8"
)

// Key sets transcribed from docs/plan-agent-rail.md decision 3 — the
// baseline lives here, outside the code under test.
var sessionKeys = []string{"kind", "id", "cwd", "agent", "state", "summary", "ts"}
var gateKeys = []string{"kind", "log_path", "workflow", "entity", "entity_title", "stage", "round", "recommendation", "ts"}

func keysOf(t *testing.T, line []byte) map[string]json.RawMessage {
	t.Helper()
	var m map[string]json.RawMessage
	if err := json.Unmarshal(line, &m); err != nil {
		t.Fatalf("line does not unmarshal as a JSON object: %v\n%s", err, line)
	}
	return m
}

func assertExactKeys(t *testing.T, line []byte, want []string) map[string]json.RawMessage {
	t.Helper()
	m := keysOf(t, line)
	for _, k := range want {
		if _, ok := m[k]; !ok {
			t.Errorf("missing key %q in %s", k, line)
		}
	}
	if len(m) != len(want) {
		t.Errorf("key count = %d, want exactly %d: %s", len(m), len(want), line)
	}
	return m
}

// markerVocabulary transcribes the plugin's marker states from src/agent.rs:24-28
// (blocked/working/idle/done) — the baseline lives here, outside the code under
// test. Any other state string renders the neutral Unknown marker.
var markerVocabulary = map[string]bool{"blocked": true, "working": true, "idle": true, "done": true}

// surveyStatuses transcribes the 2026-07-08 vocabulary survey (the entity body's
// survey table): the four observed termination_status values, absent as "".
var surveyStatuses = []string{"awaiting_user", "clean", "tool_call_pending", ""}

// TestSurveyStatusesRenderInsideMarkerVocabulary is AC1: every surveyed status,
// on a fresh-activity session, yields a row state inside the marker vocabulary —
// the count rendering Unknown drops from 4/4 (verbatim pass-through) to 0/4.
func TestSurveyStatusesRenderInsideMarkerVocabulary(t *testing.T) {
	now := time.Date(2026, 7, 8, 12, 0, 0, 0, time.UTC)
	fresh := now.Format(time.RFC3339) // started_at == now -> live
	unknown := 0
	for _, status := range surveyStatuses {
		si := sessionInfo{TerminationStatus: status, StartedAt: fresh}
		row := BuildSessionRow(si, now, 512)
		if !markerVocabulary[row.State] {
			unknown++
			t.Errorf("status %q -> state %q renders the Unknown marker", status, row.State)
		}
	}
	if unknown != 0 {
		t.Errorf("%d/%d survey statuses render Unknown, want 0", unknown, len(surveyStatuses))
	}
}

func TestRowProtocol(t *testing.T) {
	// Non-UTC instant: ts must come out RFC3339 in UTC regardless.
	now := time.Date(2026, 7, 7, 13, 0, 0, 0, time.FixedZone("CST", 8*3600))

	si := sessionInfo{
		ID:                "s-1",
		Cwd:               "/work",
		Agent:             "claude",
		TerminationStatus: "awaiting_user",
		FirstMessage:      "hello",
	}
	sLine, err := json.Marshal(BuildSessionRow(si, now, 512))
	if err != nil {
		t.Fatalf("marshal session row: %v", err)
	}
	sm := assertExactKeys(t, sLine, sessionKeys)
	var sKind string
	if err := json.Unmarshal(sm["kind"], &sKind); err != nil || sKind != "session" {
		t.Errorf("session kind = %s, want \"session\"", sm["kind"])
	}

	gi := gateInfo{
		LogPath:     "/abs/playground-gate.decisions.jsonl",
		Workflow:    "demo",
		Entity:      "playground",
		EntityTitle: "Playground handoff demo",
		Stage:       "review",
		Round:       1,
		Verdict:     "approve",
	}
	gLine, err := json.Marshal(BuildGateRow(gi, now))
	if err != nil {
		t.Fatalf("marshal gate row: %v", err)
	}
	gm := assertExactKeys(t, gLine, gateKeys)
	var gKind string
	if err := json.Unmarshal(gm["kind"], &gKind); err != nil || gKind != "gate" {
		t.Errorf("gate kind = %s, want \"gate\"", gm["kind"])
	}

	// round is a JSON number, not a string.
	if strings.HasPrefix(string(gm["round"]), `"`) {
		t.Errorf("round = %s, want a JSON number", gm["round"])
	}
	var round int
	if err := json.Unmarshal(gm["round"], &round); err != nil || round != 1 {
		t.Errorf("round = %s, want 1", gm["round"])
	}

	// ts is RFC3339 UTC on both kinds, stamped from the passed instant.
	for _, m := range []map[string]json.RawMessage{sm, gm} {
		var ts string
		if err := json.Unmarshal(m["ts"], &ts); err != nil {
			t.Fatalf("ts not a JSON string: %s", m["ts"])
		}
		if ts != "2026-07-07T05:00:00Z" {
			t.Errorf("ts = %q, want 2026-07-07T05:00:00Z", ts)
		}
		if _, err := time.Parse(time.RFC3339, ts); err != nil {
			t.Errorf("ts %q does not parse as RFC3339: %v", ts, err)
		}
	}
}

func TestRowProtocolSummaryClamp(t *testing.T) {
	now := time.Date(2026, 7, 7, 5, 0, 0, 0, time.UTC)

	// 200 three-byte runes = 600 bytes; 512 falls mid-rune, so the clamp
	// must back off to 510 bytes / 170 runes.
	long := strings.Repeat("世", 200)
	row := BuildSessionRow(sessionInfo{FirstMessage: long}, now, 512)
	if len(row.Summary) > 512 {
		t.Errorf("summary = %d bytes, want <= 512", len(row.Summary))
	}
	if len(row.Summary) != 510 {
		t.Errorf("summary = %d bytes, want 510 (rune boundary below 512)", len(row.Summary))
	}
	if !utf8.ValidString(row.Summary) {
		t.Error("clamped summary is not valid UTF-8")
	}
	if !strings.HasPrefix(long, row.Summary) {
		t.Error("clamped summary is not a prefix of the source")
	}

	short := BuildSessionRow(sessionInfo{FirstMessage: "short"}, now, 512)
	if short.Summary != "short" {
		t.Errorf("summary = %q, want unchanged \"short\"", short.Summary)
	}
}

func TestBuildRegisteredSessionRowCarriesExactPaneIdentity(t *testing.T) {
	row := BuildRegisteredSessionRow(sessionInfo{
		ID: "codex:019f5f94-a596-7d92-9928-398653669161", Cwd: "/misleading/same/cwd",
		Agent: "codex", TerminationStatus: "awaiting_user", FirstMessage: "exact pane",
	}, 73, time.Date(2026, 7, 14, 0, 0, 0, 0, time.UTC), 512)
	if row.PaneID != 73 || row.ID != "codex:019f5f94-a596-7d92-9928-398653669161" {
		t.Fatalf("registered row = %#v", row)
	}
}
