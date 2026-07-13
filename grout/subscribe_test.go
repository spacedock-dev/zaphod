// ABOUTME: Covers the private Zaphod subscriber's one tab-bound session path.
// ABOUTME: The loopback source makes data_changed re-list before one targeted pipe.

package main

import (
	"context"
	"errors"
	"fmt"
	"io"
	"net/http"
	"net/http/httptest"
	"os"
	"path/filepath"
	"strings"
	"sync/atomic"
	"testing"
	"time"
)

func fakeSubscriberZellij(t *testing.T, dir, log, panes string) string {
	t.Helper()
	panesPath := filepath.Join(dir, "panes.json")
	if err := os.WriteFile(panesPath, []byte(panes), 0o600); err != nil {
		t.Fatal(err)
	}
	return writeScript(t, dir, "zellij", "#!/bin/sh\n"+
		"for arg in \"$@\"; do\n"+
		"  if [ \"$arg\" = list-panes ]; then cat "+panesPath+"; exit 0; fi\n"+
		"  if [ \"$arg\" = pipe ]; then\n"+
		"    { echo \"$#\"; for value in \"$@\"; do printf '%s\\n' \"$value\"; done; } >> "+log+"\n"+
		"    exit 0\n"+
		"  fi\n"+
		"done\n"+
		"echo unexpected zellij invocation >&2\nexit 64\n")
}

func TestSubscribeRefreshesOnDataChangedAndTargetsStableTab(t *testing.T) {
	dir := t.TempDir()
	argvLog := filepath.Join(dir, "zellij-argv.log")
	const railURL = "file:/candidate/zellij-sidebar.wasm"
	const panes = `[
  {"id":50,"tab_id":73,"is_plugin":true,"plugin_url":"file:/candidate/zellij-sidebar.wasm","is_floating":false,"is_suppressed":false},
  {"id":7,"tab_id":73,"is_plugin":false,"is_selectable":true,"is_suppressed":false,"pane_cwd":"/work/managed"},
  {"id":9,"tab_id":81,"is_plugin":false,"is_selectable":true,"is_suppressed":false,"pane_cwd":"/work/foreign"}
	]`
	zellij := fakeSubscriberZellij(t, dir, argvLog, panes)
	startupReader, startupWriter, err := os.Pipe()
	if err != nil {
		t.Fatal(err)
	}
	defer startupReader.Close()
	defer startupWriter.Close()
	ready := make(chan string, 1)
	go func() {
		payload, _ := io.ReadAll(startupReader)
		ready <- string(payload)
	}()

	changed := make(chan struct{})
	connected := make(chan struct{})
	var lists atomic.Int32
	source := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		switch r.URL.Path {
		case "/api/v1/sessions":
			w.Header().Set("Content-Type", "application/json")
			if lists.Add(1) == 1 {
				fmt.Fprint(w, `{"sessions":[]}`)
				return
			}
			fmt.Fprint(w, `{"sessions":[{"id":"session-1","cwd":"/work/managed","agent":"codex","termination_status":"awaiting_user","first_message":"needs review","created_at":"2026-07-13T00:00:00Z"}]}`)
		case "/api/v1/events":
			w.Header().Set("Content-Type", "text/event-stream")
			flusher, ok := w.(http.Flusher)
			if !ok {
				t.Fatal("loopback response cannot stream")
			}
			close(connected)
			flusher.Flush()
			<-changed
			fmt.Fprint(w, "event: data_changed\ndata: {\"scope\":\"sessions\"}\n\n")
			flusher.Flush()
		default:
			t.Errorf("unexpected source path %q", r.URL.Path)
			http.NotFound(w, r)
		}
	}))
	defer source.Close()

	ctx, cancel := context.WithTimeout(context.Background(), 3*time.Second)
	defer cancel()
	errCh := make(chan error, 1)
	go func() {
		errCh <- runSubscribe(ctx, SubscribeConfig{
			ServerURL:         source.URL,
			ZellijBin:         zellij,
			ZellijConfigDir:   "/isolated/config",
			ZellijConfigFile:  "/isolated/config/config.kdl",
			ZellijDataDir:     "/isolated/data",
			ZellijSession:     "WORK",
			TabID:             "73",
			RailURL:           railURL,
			StartupFD:         int(startupWriter.Fd()),
			PipeTimeout:       time.Second,
			SummaryClampBytes: 512,
		}, nil)
	}()

	select {
	case <-connected:
	case <-ctx.Done():
		t.Fatal("subscriber never opened its one SSE connection")
	}
	select {
	case payload := <-ready:
		if payload != "ready\n" {
			t.Fatalf("stream-ready payload = %q, want ready newline", payload)
		}
	case <-ctx.Done():
		close(changed)
		t.Fatal("subscriber never signaled stream readiness")
	}
	close(changed)
	if err := <-errCh; !errors.Is(err, ErrSourceEOF) {
		t.Fatalf("runSubscribe error = %v, want source EOF", err)
	}
	if got := lists.Load(); got != 2 {
		t.Fatalf("source list count = %d, want initial plus data_changed refresh", got)
	}

	invs := readInvocations(t, argvLog)
	if len(invs) != 1 {
		t.Fatalf("zellij pipe invocations = %d, want one target-local session row", len(invs))
	}
	argv := invs[0]
	wantPrefix := []string{
		"--config-dir", "/isolated/config",
		"--config", "/isolated/config/config.kdl",
		"--data-dir", "/isolated/data",
		"--session", "WORK",
		"pipe", "--name", "agent-event",
		"--args", "recipient-tab-id=73",
		"--",
	}
	if len(argv) != len(wantPrefix)+1 || strings.Join(argv[:len(wantPrefix)], "\x00") != strings.Join(wantPrefix, "\x00") {
		t.Fatalf("pipe argv = %q, want prefix %q plus JSON", argv, wantPrefix)
	}
	if strings.Contains(strings.Join(argv, "\x00"), "--plugin") {
		t.Fatalf("pipe argv unexpectedly names a plugin: %q", argv)
	}
	if !strings.Contains(argv[len(argv)-1], `"cwd":"/work/managed"`) || !strings.Contains(argv[len(argv)-1], `"kind":"session"`) {
		t.Fatalf("pipe payload = %s, want source session row", argv[len(argv)-1])
	}
}

func TestSubscribeFailsClosedWhenItsVerifiedTargetIsGone(t *testing.T) {
	dir := t.TempDir()
	argvLog := filepath.Join(dir, "zellij-argv.log")
	zellij := fakeSubscriberZellij(t, dir, argvLog, `[]`)
	source := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		t.Errorf("source request %q happened after target loss", r.URL.Path)
		http.Error(w, "target should be checked first", http.StatusInternalServerError)
	}))
	defer source.Close()

	err := runSubscribe(context.Background(), SubscribeConfig{
		ServerURL:         source.URL,
		ZellijBin:         zellij,
		ZellijConfigDir:   "/isolated/config",
		ZellijConfigFile:  "/isolated/config/config.kdl",
		ZellijDataDir:     "/isolated/data",
		ZellijSession:     "WORK",
		TabID:             "73",
		RailURL:           "file:/candidate/zellij-sidebar.wasm",
		StartupFD:         -1,
		PipeTimeout:       time.Second,
		SummaryClampBytes: 512,
	}, nil)
	if !errors.Is(err, ErrTargetLost) {
		t.Fatalf("runSubscribe error = %v, want target-lost", err)
	}
	if _, err := os.Stat(argvLog); !errors.Is(err, os.ErrNotExist) {
		t.Fatalf("target loss must not pipe or leave a pipe argv log: %v", err)
	}
}
