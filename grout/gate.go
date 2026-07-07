// ABOUTME: Gate source side — brief-path derivation from a decision log and
// ABOUTME: the brief-frontmatter fields grout maps onto a gate row.

package main

// gateInfo carries the brief frontmatter fields (gate.* and
// recommendation.verdict) plus the absolute decision-log path.
type gateInfo struct {
	LogPath     string
	Workflow    string
	Entity      string
	EntityTitle string
	Stage       string
	Round       int
	Verdict     string
}
