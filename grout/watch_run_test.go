// ABOUTME: Exercises the manual watcher from ready handshake through authority loss.
// ABOUTME: Exact SessionStart state stays in memory and every visible row is leased.

package main

import (
	"bytes"
	"context"
	"fmt"
	"net/http"
	"net/http/httptest"
	"os"
	"path/filepath"
	"strings"
	"testing"
	"time"
)

func TestWatchTabProjectsOneLeasedExactSessionAndFailsClosed(t *testing.T) {
	dir := t.TempDir()
	runtimeRoot, err := os.MkdirTemp("/tmp", "zwt.")
	if err != nil {
		t.Fatal(err)
	}
	t.Cleanup(func() { _ = os.RemoveAll(runtimeRoot) })
	panesPath := filepath.Join(dir, "panes.json")
	snapshotPath := filepath.Join(dir, "snapshot.json")
	argvPath := filepath.Join(dir, "snapshot.args")
	writePanes := func(withRail bool) {
		t.Helper()
		rail := ""
		if withRail {
			rail = `{"id":50,"tab_id":73,"is_plugin":true,"plugin_url":"file:/candidate/sidebar.wasm","is_floating":false,"is_suppressed":false},`
		}
		value := "[" + rail + `{"id":7,"tab_id":73,"is_plugin":false,"is_selectable":true,"is_suppressed":false}]`
		if err := os.WriteFile(panesPath, []byte(value), 0o600); err != nil {
			t.Fatal(err)
		}
	}
	writePanes(true)
	zellij := writeScript(t, dir, "zellij", "#!/bin/sh\n"+
		"case \" $* \" in\n"+
		"  *' list-panes '*) cat "+panesPath+" ;;\n"+
		"  *' pipe '*)\n"+
		"    case \"$*\" in *-ready*) echo ready ;; *-snapshot*) printf '%s\\n' \"$*\" >> "+argvPath+"; cat > "+snapshotPath+"; echo accepted ;; esac ;;\n"+
		"esac\n")

	sessionID := "codex:019f60ff-1111-7222-8333-444455556666"
	source := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		switch r.URL.Path {
		case "/api/v1/events":
			w.Header().Set("Content-Type", "text/event-stream")
			w.(http.Flusher).Flush()
			<-r.Context().Done()
		case "/api/v1/sessions/" + sessionID:
			fmt.Fprintf(w, `{"id":%q,"agent":"codex","first_message":"KJ_WATCH_ROW"}`, sessionID)
		default:
			http.NotFound(w, r)
		}
	}))
	defer source.Close()

	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()
	ready := make(chan WatchReady, 1)
	errCh := make(chan error, 1)
	go func() {
		errCh <- runWatchTab(ctx, WatchConfig{
			WatchRoute: WatchRoute{
				ServerURL: source.URL, ZellijBin: zellij,
				ZellijConfigDir: "/c", ZellijConfigFile: "/c/config.kdl", ZellijDataDir: "/d",
				ZellijSession: "managed", RailURL: "file:/candidate/sidebar.wasm", RecipientToken: "token",
				PipeTimeout: time.Second, SourceTimeout: time.Second, SummaryClampBytes: 512,
			},
			PaneID: 7, SocketRoot: runtimeRoot, Lease: 500 * time.Millisecond,
			Heartbeat: 100 * time.Millisecond, Ready: ready,
		}, &bytes.Buffer{})
	}()
	var started WatchReady
	select {
	case started = <-ready:
	case err := <-errCh:
		t.Fatalf("watcher failed before ready: %v", err)
	case <-time.After(3 * time.Second):
		t.Fatal("watcher readiness timed out")
	}
	if started.Target != (WatchTarget{TabID: 73, TerminalPaneID: 7, RailPaneID: 50}) {
		t.Fatalf("ready target = %#v", started.Target)
	}
	hook := []byte(`{"session_id":"019f60ff-1111-7222-8333-444455556666","hook_event_name":"SessionStart","source":"startup"}`)
	if err := sendWatchHook(ctx, runtimeRoot, "managed", "7", hook); err != nil {
		t.Fatal(err)
	}
	for deadline := time.Now().Add(3 * time.Second); time.Now().Before(deadline); time.Sleep(20 * time.Millisecond) {
		payload, _ := os.ReadFile(snapshotPath)
		if strings.Contains(string(payload), "KJ_WATCH_ROW") {
			break
		}
	}
	payload, err := os.ReadFile(snapshotPath)
	if err != nil || !strings.Contains(string(payload), "KJ_WATCH_ROW") {
		t.Fatalf("exact session was not projected: %v %s", err, payload)
	}
	args, err := os.ReadFile(argvPath)
	if err != nil {
		t.Fatal(err)
	}
	for _, want := range []string{"watch-generation=", "lease-ms=500", "recipient-tab-id=73", "recipient-token=token"} {
		if !strings.Contains(string(args), want) {
			t.Fatalf("leased snapshot args omitted %q: %s", want, args)
		}
	}
	entries, err := os.ReadDir(runtimeRoot)
	if err != nil {
		t.Fatal(err)
	}
	if len(entries) != 1 || !strings.HasSuffix(entries[0].Name(), ".sock") {
		t.Fatalf("watcher wrote durable authority: %v", entries)
	}

	writePanes(false)
	select {
	case err := <-errCh:
		if err == nil || !strings.Contains(err.Error(), "target-lost") {
			t.Fatalf("rail loss error = %v", err)
		}
	case <-time.After(3 * time.Second):
		t.Fatal("original rail loss did not stop watcher")
	}
	if _, err := os.Lstat(started.SocketPath); !os.IsNotExist(err) {
		t.Fatalf("watch socket survived authority loss: %v", err)
	}
}
