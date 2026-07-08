// ABOUTME: agentsview source side — the subset of `session get --format json`
// ABOUTME: output that grout maps onto a session row.

package main

import (
	"bytes"
	"encoding/json"
	"fmt"
	"io"
	"os/exec"
)

// sessionInfo pins the source field names off the recorded fixture
// (testdata/session-get.json, agentsview v0.36.1): termination_status plus the
// three activity timestamps feed the row's state, first_message its summary.
// The row fields, not these source fields, are the protocol contract.
type sessionInfo struct {
	ID                string `json:"id"`
	Cwd               string `json:"cwd"`
	Agent             string `json:"agent"`
	TerminationStatus string `json:"termination_status"`
	FirstMessage      string `json:"first_message"`
	EndedAt           string `json:"ended_at"`
	StartedAt         string `json:"started_at"`
	CreatedAt         string `json:"created_at"`
}

func decodeSession(r io.Reader) (sessionInfo, error) {
	var si sessionInfo
	err := json.NewDecoder(r).Decode(&si)
	return si, err
}

// FetchSession runs a one-shot `session get` against the agentsview binary.
func FetchSession(bin, id string) (sessionInfo, error) {
	out, err := exec.Command(bin, "session", "get", id, "--format", "json").Output()
	if err != nil {
		return sessionInfo{}, fmt.Errorf("%s session get %s: %w", bin, id, err)
	}
	return decodeSession(bytes.NewReader(out))
}
