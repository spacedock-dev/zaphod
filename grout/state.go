// ABOUTME: Maps agentsview's termination_status plus transcript recency onto
// ABOUTME: the plugin's marker vocabulary; first match wins, else verbatim.

package main

import "time"

// workingWindow and idleWindow mirror agentsview v0.36.1's own liveness
// derivation (decoded from its served UI bundle): a transcript that last moved
// within workingWindow reads as still working, within idleWindow as idle.
const (
	workingWindow = 60 * time.Second
	idleWindow    = 10 * time.Minute
)

// MapSessionState maps termination_status + last-activity recency onto the
// plugin's marker vocabulary (blocked/working/idle/done); first match wins,
// unmatched statuses pass through verbatim (the plugin's unknown marker).
// awaiting_user never decays — the rail's return-later scenario, so it reports
// blocked at any age (agentsview decays it to quiet at 10m; grout does not).
// The recency check precedes clean->done, so a just-clean session that may
// still be appending reads as working. A zero-time lastActivity gives
// effectively-infinite age, so a stale row never guesses a live state.
func MapSessionState(status string, lastActivity, now time.Time) string {
	if status == "awaiting_user" {
		return "blocked"
	}
	age := now.Sub(lastActivity)
	if age < workingWindow {
		return "working"
	}
	if status == "clean" {
		return "done"
	}
	if age < idleWindow {
		return "idle"
	}
	return status
}
