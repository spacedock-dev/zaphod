// ABOUTME: The native artifact exposes only its private subscribe subcommand.
// ABOUTME: Missing target/profile arguments fail before any source or Zellij work.

package main

import (
	"bytes"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"testing"
)

func TestCLIRequiresPrivateSubscribeTarget(t *testing.T) {
	wd, err := os.Getwd()
	if err != nil {
		t.Fatal(err)
	}
	bin := filepath.Join(t.TempDir(), "zaphod")
	build := exec.Command("go", "build", "-o", bin, ".")
	build.Dir = wd
	if out, err := build.CombinedOutput(); err != nil {
		t.Fatalf("go build: %v\n%s", err, out)
	}

	// Validation must not depend on the caller's CWD or discover a target
	// ambiently. The sidecar only starts from the direct script's full tuple.
	for _, tc := range []struct {
		name string
		args []string
		want []string
	}{
		{"no subcommand", nil, []string{"usage", "zaphod subscribe"}},
		{
			"missing stable target",
			[]string{"subscribe", "--server", "http://127.0.0.1:8080", "--zellij-bin", "zellij"},
			[]string{"usage", "--tab-id", "--rail-url"},
		},
	} {
		t.Run(tc.name, func(t *testing.T) {
			cmd := exec.Command(bin, tc.args...)
			// Use the repo parent so no implementation may rely on grout/ as a
			// working directory.
			cmd.Dir = filepath.Dir(wd)
			var stderr bytes.Buffer
			cmd.Stderr = &stderr
			exit, ok := cmd.Run().(*exec.ExitError)
			if !ok {
				t.Fatalf("run did not exit non-zero (stderr: %s)", stderr.String())
			}
			if exit.ExitCode() != 2 {
				t.Errorf("exit code = %d, want 2", exit.ExitCode())
			}
			got := stderr.String()
			for _, want := range tc.want {
				if !strings.Contains(got, want) {
					t.Errorf("stderr %q missing %q", got, want)
				}
			}
		})
	}
}
