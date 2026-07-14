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

func TestRegisterAgentSessionCLIUsesHookStdinAndInheritedPaneIdentity(t *testing.T) {
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
	registryRoot := filepath.Join(t.TempDir(), "registry")
	const id = "019f5f94-a596-7d92-9928-398653669161"
	command := exec.Command(bin, "register-agent-session", "--registry-dir", registryRoot)
	command.Env = append(os.Environ(), "ZELLIJ_SESSION_NAME=managed", "ZELLIJ_PANE_ID=7")
	command.Stdin = bytes.NewReader(validHook(id))
	if output, err := command.CombinedOutput(); err != nil {
		t.Fatalf("register: %v\n%s", err, output)
	}
	snapshot, err := (agentRegistryStore{root: registryRoot}).read("managed")
	if err != nil || len(snapshot.Registrations) != 1 {
		t.Fatalf("snapshot = %#v, err = %v", snapshot, err)
	}
	if got := snapshot.Registrations[0]; got.PaneID != 7 || got.AgentsViewSessionID != "codex:"+id {
		t.Fatalf("registration = %#v", got)
	}
}

func TestRegisterAgentSessionCLIIsNoopOutsideZellijAndFailsClosedOnBadInput(t *testing.T) {
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
	registryRoot := filepath.Join(t.TempDir(), "registry")

	outside := exec.Command(bin, "register-agent-session", "--registry-dir", registryRoot)
	outside.Env = []string{"PATH=" + os.Getenv("PATH")}
	outside.Stdin = strings.NewReader("not json")
	if output, err := outside.CombinedOutput(); err != nil {
		t.Fatalf("outside Zellij should be a no-op: %v\n%s", err, output)
	}
	if _, err := os.Stat(registryRoot); !os.IsNotExist(err) {
		t.Fatalf("outside-Zellij hook mutated registry: %v", err)
	}

	bad := exec.Command(bin, "register-agent-session", "--registry-dir", registryRoot)
	bad.Env = append(os.Environ(), "ZELLIJ_SESSION_NAME=managed", "ZELLIJ_PANE_ID=7")
	bad.Stdin = strings.NewReader(`{"session_id":"newest","hook_event_name":"SessionStart","source":"startup"}`)
	if exit, ok := bad.Run().(*exec.ExitError); !ok || exit.ExitCode() != 1 {
		t.Fatalf("bad input exit = %v, want 1", exit)
	}
	if _, err := os.Stat(registryRoot); !os.IsNotExist(err) {
		t.Fatalf("bad hook mutated registry: %v", err)
	}
}

func TestSubscribeUsesTheSameExplicitRegistryRootAsSessionStart(t *testing.T) {
	registryDir := filepath.Join(t.TempDir(), "registry")
	t.Setenv("ZAPHOD_REGISTRY_DIR", registryDir)
	cfg, err := parseSubscribeArgs([]string{
		"--server", "http://127.0.0.1:8080", "--zellij-bin", "zellij",
		"--zellij-config-dir", "/c", "--zellij-config", "/c/config.kdl",
		"--zellij-data-dir", "/d", "--zellij-session", "managed", "--tab-id", "73",
		"--rail-url", "file:/candidate.wasm", "--checkout-cwd", "/checkout",
		"--recipient-token", "token",
	}, &bytes.Buffer{})
	if err != nil {
		t.Fatal(err)
	}
	if cfg.RegistryDir != registryDir {
		t.Fatalf("subscriber registry = %q, want hook registry %q", cfg.RegistryDir, registryDir)
	}
}
