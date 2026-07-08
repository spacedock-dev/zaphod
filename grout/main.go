// ABOUTME: Grout sprint-0 skeleton — one-shot run that fetches one agentsview
// ABOUTME: session, reads one gate log, and pipes both rows into zellij.

package main

import (
	"fmt"
	"io"
	"os"
	"time"
)

type Config struct {
	AgentsviewBin     string        // "agentsview"
	SessionID         string        // required argv[1]
	GateLog           string        // required argv[2]
	ZellijBin         string        // "zellij"
	ZellijSession     string        // "" = inherit env; non-empty sets ZELLIJ_SESSION_NAME on the child
	PipeName          string        // "agent-event" — protocol constant
	PipeTimeout       time.Duration // 5 * time.Second — kill timer
	SummaryClampBytes int           // 512
}

// defaultConfig carries only cwd-independent knobs. SessionID and GateLog have
// no default: a demo session id exists only in one machine's DB and a relative
// gate-log path resolves against the cwd, so both must come from argv.
func defaultConfig() Config {
	return Config{
		AgentsviewBin:     "agentsview",
		ZellijBin:         "zellij",
		ZellijSession:     "",
		PipeName:          "agent-event",
		PipeTimeout:       5 * time.Second,
		SummaryClampBytes: 512,
	}
}

// run emits the session row then the gate row. Source failures are fatal
// before anything is piped; pipe failures are per-row independent — a
// timed-out session pipe must not stop the gate row.
func run(cfg Config, stderr io.Writer) error {
	si, err := FetchSession(cfg.AgentsviewBin, cfg.SessionID)
	if err != nil {
		return err
	}
	gi, err := GateFromLog(cfg.GateLog)
	if err != nil {
		return err
	}

	sRow := BuildSessionRow(si, time.Now(), cfg.SummaryClampBytes)
	gRow := BuildGateRow(gi, time.Now())
	failed := false
	for _, it := range []struct {
		kind string
		row  any
	}{{sRow.Kind, sRow}, {gRow.Kind, gRow}} {
		if err := EmitRow(cfg, it.kind, it.row, stderr); err != nil {
			fmt.Fprintln(stderr, err)
			failed = true
		}
	}
	if failed {
		return fmt.Errorf("not every row was piped cleanly")
	}
	return nil
}

func main() {
	if len(os.Args) < 3 {
		fmt.Fprintln(os.Stderr, "usage: grout <session-id> <gate-log>")
		os.Exit(2)
	}
	cfg := defaultConfig()
	cfg.SessionID = os.Args[1]
	cfg.GateLog = os.Args[2]
	if err := run(cfg, os.Stderr); err != nil {
		fmt.Fprintln(os.Stderr, err)
		os.Exit(1)
	}
}
