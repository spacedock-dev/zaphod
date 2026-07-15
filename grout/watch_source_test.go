// ABOUTME: Specifies exact readiness acknowledgment atoms from Zellij CLI pipes.
// ABOUTME: Per-client runtime duplication may repeat one logical rail's output.

package main

import (
	"context"
	"strconv"
	"testing"
	"time"
)

func TestCompleteAcknowledgmentAtomsAreExactAndParameterized(t *testing.T) {
	for _, test := range []struct {
		name     string
		expected string
		output   string
		valid    bool
	}{
		{name: "ready single", expected: "ready", output: "ready", valid: true},
		{name: "ready repeated", expected: "ready", output: "readyreadyready", valid: true},
		{name: "accepted single", expected: "accepted", output: "accepted", valid: true},
		{name: "accepted repeated", expected: "accepted", output: "acceptedacceptedaccepted", valid: true},
		{name: "empty expected", expected: "", output: "accepted", valid: false},
		{name: "empty output", expected: "accepted", output: "", valid: false},
		{name: "partial", expected: "accepted", output: "acceptedaccept", valid: false},
		{name: "foreign", expected: "accepted", output: "acceptedforeign", valid: false},
		{name: "wrong atom", expected: "accepted", output: "readyready", valid: false},
		{name: "separated", expected: "accepted", output: "accepted accepted", valid: false},
	} {
		t.Run(test.name, func(t *testing.T) {
			if got := completeAcknowledgmentAtoms([]byte(test.output), test.expected); got != test.valid {
				t.Fatalf("completeAcknowledgmentAtoms(%q, %q) = %v, want %v", test.output, test.expected, got, test.valid)
			}
		})
	}
}

func TestWaitForRecipientAcceptsOnlyCompleteReadyAtoms(t *testing.T) {
	cases := []struct {
		name   string
		output string
		ready  bool
	}{
		{name: "single", output: "ready", ready: true},
		{name: "two retained runtimes", output: "readyready", ready: true},
		{name: "many retained runtimes", output: "readyreadyreadyready", ready: true},
		{name: "empty", output: "", ready: false},
		{name: "partial", output: "read", ready: false},
		{name: "partial final atom", output: "readyread", ready: false},
		{name: "foreign suffix", output: "readyforeign", ready: false},
		{name: "foreign prefix", output: "foreignready", ready: false},
		{name: "internal newline", output: "ready\nready", ready: false},
		{name: "internal space", output: "ready ready", ready: false},
	}
	for _, test := range cases {
		t.Run(test.name, func(t *testing.T) {
			zellij := writeScript(t, t.TempDir(), "zellij", "#!/bin/sh\nprintf '%b' "+strconv.Quote(test.output)+"\n")
			ctx, cancel := context.WithTimeout(context.Background(), 600*time.Millisecond)
			defer cancel()
			err := waitForRecipient(ctx, WatchRoute{
				ZellijBin: zellij, ZellijConfigDir: "/c", ZellijConfigFile: "/c/config.kdl",
				ZellijDataDir: "/d", ZellijSession: "managed", TabID: "73",
				RecipientToken: "token", PipeTimeout: 100 * time.Millisecond,
				recipientWaitTimeout: 250 * time.Millisecond,
			})
			if test.ready && err != nil {
				t.Fatalf("complete readiness output %q was rejected: %v", test.output, err)
			}
			if !test.ready && err == nil {
				t.Fatalf("invalid readiness output %q was accepted", test.output)
			}
		})
	}
}
