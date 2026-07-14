// ABOUTME: The native artifact exposes only its private subscribe subcommand.
// ABOUTME: Missing target/profile arguments fail before any source or Zellij work.

package main

import (
	"bytes"
	"context"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"testing"
	"time"
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
	watchRoot, err := os.MkdirTemp("/tmp", "zwt.")
	if err != nil {
		t.Fatal(err)
	}
	t.Cleanup(func() { _ = os.RemoveAll(watchRoot) })
	listener, _, err := listenWatchSocket(watchRoot, "managed", "7")
	if err != nil {
		t.Fatal(err)
	}
	defer listener.Close()
	accepted := make(chan WatchRegistration, 1)
	acceptErr := make(chan error, 1)
	go func() {
		registration, err := acceptWatchRegistration(context.Background(), listener, "managed", "7")
		if err != nil {
			acceptErr <- err
			return
		}
		accepted <- registration
	}()
	const id = "019f5f94-a596-7d92-9928-398653669161"
	command := exec.Command(bin, "register-agent-session", "--watch-dir", watchRoot)
	command.Env = append(os.Environ(), "ZELLIJ_SESSION_NAME=managed", "ZELLIJ_PANE_ID=7")
	command.Stdin = bytes.NewReader(validHook(id))
	if output, err := command.CombinedOutput(); err != nil {
		t.Fatalf("register: %v\n%s", err, output)
	}
	select {
	case got := <-accepted:
		if got.PaneID != 7 || got.AgentsViewSessionID != "codex:"+id {
			t.Fatalf("registration = %#v", got)
		}
	case err := <-acceptErr:
		t.Fatal(err)
	case <-time.After(time.Second):
		t.Fatal("hook was not delivered")
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
	watchRoot := filepath.Join(t.TempDir(), "watch")

	outside := exec.Command(bin, "register-agent-session", "--watch-dir", watchRoot)
	outside.Env = []string{"PATH=" + os.Getenv("PATH")}
	outside.Stdin = strings.NewReader("not json")
	if output, err := outside.CombinedOutput(); err != nil {
		t.Fatalf("outside Zellij should be a no-op: %v\n%s", err, output)
	}
	if _, err := os.Stat(watchRoot); !os.IsNotExist(err) {
		t.Fatalf("outside-Zellij hook created a watch root: %v", err)
	}

	bad := exec.Command(bin, "register-agent-session", "--watch-dir", watchRoot)
	bad.Env = append(os.Environ(), "ZELLIJ_SESSION_NAME=managed", "ZELLIJ_PANE_ID=7")
	bad.Stdin = strings.NewReader(`{"session_id":"newest","hook_event_name":"SessionStart","source":"startup"}`)
	if exit, ok := bad.Run().(*exec.ExitError); !ok || exit.ExitCode() != 1 {
		t.Fatalf("bad input exit = %v, want 1", exit)
	}
	if _, err := os.Stat(watchRoot); !os.IsNotExist(err) {
		t.Fatalf("bad hook created a watch root: %v", err)
	}
}

func TestWatchTabArgsUseOnlyExplicitRouteContext(t *testing.T) {
	t.Setenv("ZELLIJ_SESSION_NAME", "managed")
	t.Setenv("ZELLIJ_PANE_ID", "7")
	t.Setenv("ZAPHOD_RAIL_URL", "file:/candidate.wasm")
	t.Setenv("ZAPHOD_RECIPIENT_TOKEN", "token")
	t.Setenv("ZAPHOD_ZELLIJ_CONFIG_DIR", "/c")
	t.Setenv("ZAPHOD_ZELLIJ_CONFIG_FILE", "/c/config.kdl")
	t.Setenv("ZAPHOD_ZELLIJ_DATA_DIR", "/d")
	watchDir := "/tmp/zaphod-watch-test"
	t.Setenv("ZAPHOD_WATCH_DIR", watchDir)
	cfg, err := parseWatchTabArgs([]string{"--server", "http://127.0.0.1:8080", "--foreground"}, &bytes.Buffer{})
	if err != nil {
		t.Fatal(err)
	}
	if cfg.PaneID != 7 || cfg.ZellijSession != "managed" || cfg.RailURL != "file:/candidate.wasm" ||
		cfg.RecipientToken != "token" || cfg.SocketRoot != watchDir || !cfg.Foreground {
		t.Fatalf("watch config = %#v", cfg)
	}
}
