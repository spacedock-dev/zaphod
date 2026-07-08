// ABOUTME: AC5 — grout requires both positionals; with fewer it exits 2 and
// ABOUTME: prints usage naming both, identically from any cwd (no cwd default).

package main

import (
	"bytes"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"testing"
)

func TestCLIRequiresBothPositionals(t *testing.T) {
	wd, err := os.Getwd()
	if err != nil {
		t.Fatal(err)
	}
	bin := filepath.Join(t.TempDir(), "grout")
	build := exec.Command("go", "build", "-o", bin, ".")
	build.Dir = wd
	if out, err := build.CombinedOutput(); err != nil {
		t.Fatalf("go build: %v\n%s", err, out)
	}

	// The usage path never touches the filesystem, so it must behave
	// identically from the repo root and from grout/ (the cwd CL was bitten in).
	for _, cwd := range []string{wd, filepath.Dir(wd)} {
		t.Run(cwd, func(t *testing.T) {
			cmd := exec.Command(bin)
			cmd.Dir = cwd
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
			for _, want := range []string{"usage", "session-id", "gate-log"} {
				if !strings.Contains(got, want) {
					t.Errorf("stderr %q missing %q", got, want)
				}
			}
		})
	}
}
