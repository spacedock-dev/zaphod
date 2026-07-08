// ABOUTME: Table-drives MapSessionState across the mapping table's recency
// ABOUTME: boundaries; expected values trace to agentsview's decoded derivation.

package main

import (
	"testing"
	"time"
)

func TestMapSessionState(t *testing.T) {
	now := time.Date(2026, 7, 8, 12, 0, 0, 0, time.UTC)
	cases := []struct {
		name   string
		status string
		age    time.Duration
		want   string
	}{
		{"awaiting_user fresh -> blocked", "awaiting_user", 0, "blocked"},
		{"awaiting_user stale still blocked (no decay)", "awaiting_user", 2 * time.Hour, "blocked"},
		{"clean within working window -> working", "clean", 30 * time.Second, "working"},
		{"clean at working boundary -> done", "clean", workingWindow, "done"},
		{"clean past working window -> done", "clean", 5 * time.Minute, "done"},
		{"tool_call_pending fresh -> working", "tool_call_pending", 30 * time.Second, "working"},
		{"tool_call_pending at working boundary -> idle", "tool_call_pending", workingWindow, "idle"},
		{"tool_call_pending within idle window -> idle", "tool_call_pending", 5 * time.Minute, "idle"},
		{"tool_call_pending at idle boundary -> verbatim", "tool_call_pending", idleWindow, "tool_call_pending"},
		{"tool_call_pending stale -> verbatim", "tool_call_pending", 2 * time.Hour, "tool_call_pending"},
		{"absent status fresh -> working", "", 30 * time.Second, "working"},
		{"absent status within idle window -> idle", "", 5 * time.Minute, "idle"},
		{"absent status stale -> verbatim empty", "", 2 * time.Hour, ""},
		{"truncated fresh -> working", "truncated", 10 * time.Second, "working"},
		{"truncated within idle window -> idle", "truncated", 8 * time.Minute, "idle"},
		{"truncated stale -> verbatim", "truncated", 2 * time.Hour, "truncated"},
		// AC4 vocabulary drift: an unsurveyed status with stale activity passes
		// through verbatim (the plugin renders the unknown marker), no crash.
		{"drift status stale -> verbatim", "future_status", 2 * time.Hour, "future_status"},
	}
	for _, tc := range cases {
		t.Run(tc.name, func(t *testing.T) {
			got := MapSessionState(tc.status, now.Add(-tc.age), now)
			if got != tc.want {
				t.Errorf("MapSessionState(%q, now-%s) = %q, want %q", tc.status, tc.age, got, tc.want)
			}
		})
	}
}

func TestMapSessionStateZeroTimeIsInfiniteAge(t *testing.T) {
	now := time.Date(2026, 7, 8, 12, 0, 0, 0, time.UTC)
	// An unparseable/absent timestamp arrives as the zero time: effectively
	// infinite age, so a live state is never guessed — only rows 3/5.
	if got := MapSessionState("clean", time.Time{}, now); got != "done" {
		t.Errorf("clean @ zero-time = %q, want done", got)
	}
	if got := MapSessionState("tool_call_pending", time.Time{}, now); got != "tool_call_pending" {
		t.Errorf("tool_call_pending @ zero-time = %q, want verbatim tool_call_pending", got)
	}
}
