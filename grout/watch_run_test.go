// ABOUTME: Exercises the manual watcher from one startup inventory through leased delivery.
// ABOUTME: Post-ready lifecycle events never trigger native pane inventory or cleanup probes.

package main

import (
	"bytes"
	"context"
	"fmt"
	"net/http"
	"net/http/httptest"
	"os"
	"path/filepath"
	"strconv"
	"strings"
	"sync/atomic"
	"testing"
	"time"
)

func TestWatchTabUsesNativeInventoryOnlyBeforeReady(t *testing.T) {
	dir := t.TempDir()
	runtimeRoot, err := os.MkdirTemp("/tmp", "zwt.")
	if err != nil {
		t.Fatal(err)
	}
	t.Cleanup(func() { _ = os.RemoveAll(runtimeRoot) })
	panesPath := filepath.Join(dir, "panes.json")
	snapshotPath := filepath.Join(dir, "snapshot.json")
	argvPath := filepath.Join(dir, "snapshot.args")
	nativePath := filepath.Join(dir, "native.calls")
	heartbeatPath := filepath.Join(dir, "heartbeat.calls")
	writePanes := func(value string) {
		t.Helper()
		if err := os.WriteFile(panesPath, []byte(value), 0o600); err != nil {
			t.Fatal(err)
		}
	}
	writePanes(`[
      {"id":50,"tab_id":73,"is_plugin":true,"plugin_url":"file:/candidate/sidebar.wasm","is_floating":false,"is_suppressed":false},
      {"id":7,"tab_id":73,"is_plugin":false,"is_selectable":true,"is_suppressed":false},
      {"id":60,"tab_id":74,"is_plugin":true,"plugin_url":"file:/candidate/sidebar.wasm","is_floating":false,"is_suppressed":false}
    ]`)
	zellij := writeScript(t, dir, "zellij", "#!/bin/sh\n"+
		"case \" $* \" in\n"+
		"  *' list-panes '*) printf 'inventory\\n' >> "+nativePath+"; cat "+panesPath+" ;;\n"+
		"  *' pipe '*)\n"+
		"    case \"$*\" in\n"+
		"      *-ready*) printf readyready ;;\n"+
		"      *-heartbeat*) printf 'heartbeat\\n' >> "+heartbeatPath+" ;;\n"+
		"      *-snapshot*) printf '%s\\n' \"$*\" >> "+argvPath+"; cat > "+snapshotPath+"; printf acceptedaccepted ;;\n"+
		"    esac ;;\n"+
		"esac\n")

	sessionID := "codex:019f60ff-1111-7222-8333-444455556666"
	var exactRequests atomic.Int64
	dataChanged := make(chan struct{}, 8)
	source := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		switch r.URL.Path {
		case "/api/v1/events":
			w.Header().Set("Content-Type", "text/event-stream")
			flusher := w.(http.Flusher)
			ticker := time.NewTicker(50 * time.Millisecond)
			defer ticker.Stop()
			fmt.Fprint(w, "event: heartbeat\ndata: {}\n\n")
			flusher.Flush()
			for {
				select {
				case <-r.Context().Done():
					return
				case <-dataChanged:
					fmt.Fprint(w, "event: data_changed\ndata: {}\n\n")
				case <-ticker.C:
					fmt.Fprint(w, "event: heartbeat\ndata: {}\n\n")
				}
				flusher.Flush()
			}
		case "/api/v1/sessions/" + sessionID:
			exactRequests.Add(1)
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
				recipientWaitTimeout: 500 * time.Millisecond,
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
	assertNativeCount := func(want int) {
		t.Helper()
		calls, _ := os.ReadFile(nativePath)
		if got := bytes.Count(calls, []byte("\n")); got != want {
			t.Fatalf("native inventory calls = %d, want %d: %s", got, want, calls)
		}
	}
	assertNativeCount(1)
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
	assertNativeCount(1)

	lifecycle := []struct {
		name  string
		panes string
	}{
		{"terminal-close", `[{"id":50,"tab_id":73,"is_plugin":true,"plugin_url":"file:/candidate/sidebar.wasm","is_floating":false,"is_suppressed":false}]`},
		{"terminal-move", `[{"id":50,"tab_id":73,"is_plugin":true,"plugin_url":"file:/candidate/sidebar.wasm","is_floating":false,"is_suppressed":false},{"id":7,"tab_id":74,"is_plugin":false,"is_selectable":true,"is_suppressed":false}]`},
		{"terminal-suppress", `[{"id":50,"tab_id":73,"is_plugin":true,"plugin_url":"file:/candidate/sidebar.wasm","is_floating":false,"is_suppressed":false},{"id":7,"tab_id":73,"is_plugin":false,"is_selectable":true,"is_suppressed":true}]`},
		{"original-rail-loss", `[{"id":7,"tab_id":73,"is_plugin":false,"is_selectable":true,"is_suppressed":false},{"id":60,"tab_id":74,"is_plugin":true,"plugin_url":"file:/candidate/sidebar.wasm","is_floating":false,"is_suppressed":false}]`},
		{"bystander-rail-loss", `[{"id":50,"tab_id":73,"is_plugin":true,"plugin_url":"file:/candidate/sidebar.wasm","is_floating":false,"is_suppressed":false},{"id":7,"tab_id":73,"is_plugin":false,"is_selectable":true,"is_suppressed":false}]`},
	}
	for i, event := range lifecycle {
		writePanes(event.panes)
		dataChanged <- struct{}{}
		wantRequests := int64(i + 2)
		for deadline := time.Now().Add(2 * time.Second); exactRequests.Load() < wantRequests && time.Now().Before(deadline); {
			time.Sleep(10 * time.Millisecond)
		}
		if got := exactRequests.Load(); got != wantRequests {
			t.Fatalf("%s exact requests = %d, want %d", event.name, got, wantRequests)
		}
		select {
		case err := <-errCh:
			t.Fatalf("%s stopped startup-authorized watcher: %v", event.name, err)
		default:
		}
		assertNativeCount(1)
	}
	time.Sleep(250 * time.Millisecond)
	if got := exactRequests.Load(); got != int64(len(lifecycle)+1) {
		t.Fatalf("heartbeat multiplied exact source fetches: got %d, want %d", got, len(lifecycle)+1)
	}
	if heartbeats, _ := os.ReadFile(heartbeatPath); len(heartbeats) == 0 {
		t.Fatal("watcher did not exercise post-ready heartbeats")
	}
	args, err := os.ReadFile(argvPath)
	if err != nil {
		t.Fatal(err)
	}
	for _, want := range []string{"watch-generation=", "lease-ms=500", "recipient-tab-id=73", "recipient-rail-id=50", "recipient-token=token"} {
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

	cancel()
	select {
	case err := <-errCh:
		if err != nil {
			t.Fatalf("watcher cancellation = %v", err)
		}
	case <-time.After(time.Second):
		t.Fatal("watcher cleanup did not finish")
	}
	assertNativeCount(1)
	if _, err := os.Lstat(started.SocketPath); !os.IsNotExist(err) {
		t.Fatalf("watch socket survived explicit cleanup: %v", err)
	}
}

func TestWatcherReadinessIncludesSessionStartAcceptedBeforeBoundary(t *testing.T) {
	dir := t.TempDir()
	runtimeRoot, err := os.MkdirTemp("/tmp", "zwt.")
	if err != nil {
		t.Fatal(err)
	}
	t.Cleanup(func() { _ = os.RemoveAll(runtimeRoot) })
	readyGate := filepath.Join(dir, "recipient-ready")
	snapshotPath := filepath.Join(dir, "snapshot.json")
	zellij := writeScript(t, dir, "zellij", "#!/bin/sh\n"+
		"case \" $* \" in\n"+
		"  *' list-panes '*) printf '%s\\n' '[{\"id\":50,\"tab_id\":73,\"is_plugin\":true,\"plugin_url\":\"file:/candidate/sidebar.wasm\",\"is_floating\":false,\"is_suppressed\":false},{\"id\":7,\"tab_id\":73,\"is_plugin\":false,\"is_selectable\":true,\"is_suppressed\":false}]' ;;\n"+
		"  *' pipe '*) case \"$*\" in *-ready*) [ -e "+readyGate+" ] && echo ready ;; *-snapshot*) cat > "+snapshotPath+"; echo accepted ;; esac ;;\n"+
		"esac\n")

	const agentID = "019f60ff-1111-7222-8333-444455556666"
	source := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		switch r.URL.Path {
		case "/api/v1/events":
			w.Header().Set("Content-Type", "text/event-stream")
			w.(http.Flusher).Flush()
			<-r.Context().Done()
		case "/api/v1/sessions/codex:" + agentID:
			fmt.Fprintf(w, `{"id":"codex:%s","agent":"codex","first_message":"PRE_READY_ROW"}`, agentID)
		default:
			http.NotFound(w, r)
		}
	}))
	defer source.Close()

	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()
	ready := make(chan WatchReady, 1)
	errs := make(chan error, 1)
	go func() {
		errs <- runWatchTab(ctx, WatchConfig{WatchRoute: WatchRoute{
			ServerURL: source.URL, ZellijBin: zellij, ZellijConfigDir: "/c",
			ZellijConfigFile: "/c/config.kdl", ZellijDataDir: "/d", ZellijSession: "managed",
			RailURL: "file:/candidate/sidebar.wasm", RecipientToken: "token",
			PipeTimeout: time.Second, SourceTimeout: time.Second, SummaryClampBytes: 512,
		}, PaneID: 7, SocketRoot: runtimeRoot, Lease: 500 * time.Millisecond,
			Heartbeat: 100 * time.Millisecond, Ready: ready}, &bytes.Buffer{})
	}()
	socket, err := watchSocketPath(runtimeRoot, "managed", "7")
	if err != nil {
		t.Fatal(err)
	}
	for deadline := time.Now().Add(2 * time.Second); time.Now().Before(deadline); time.Sleep(10 * time.Millisecond) {
		if _, err := os.Lstat(socket); err == nil {
			break
		}
	}
	hook := []byte(`{"session_id":"` + agentID + `","hook_event_name":"SessionStart","source":"startup"}`)
	if err := sendWatchHook(ctx, runtimeRoot, "managed", "7", hook); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(readyGate, nil, 0o600); err != nil {
		t.Fatal(err)
	}
	select {
	case <-ready:
		payload, _ := os.ReadFile(snapshotPath)
		if !strings.Contains(string(payload), "PRE_READY_ROW") {
			t.Fatalf("readiness preceded accepted SessionStart projection: %s", payload)
		}
	case err := <-errs:
		t.Fatalf("watcher failed before ready: %v", err)
	case <-time.After(3 * time.Second):
		t.Fatal("watcher readiness timed out")
	}
}

func TestWatcherCleanupUsesNoNativeInventoryOrSnapshot(t *testing.T) {
	dir := t.TempDir()
	runtimeRoot, err := os.MkdirTemp("/tmp", "zwt.")
	if err != nil {
		t.Fatal(err)
	}
	t.Cleanup(func() { _ = os.RemoveAll(runtimeRoot) })
	nativeLog := filepath.Join(dir, "native.log")
	snapshotLog := filepath.Join(dir, "snapshot.log")
	zellij := writeScript(t, dir, "zellij", "#!/bin/sh\n"+
		"case \" $* \" in\n"+
		"  *' list-panes '*) printf 'inventory\\n' >> "+nativeLog+"; printf '%s\\n' '[{\"id\":50,\"tab_id\":73,\"is_plugin\":true,\"plugin_url\":\"file:/candidate/sidebar.wasm\",\"is_floating\":false,\"is_suppressed\":false},{\"id\":7,\"tab_id\":73,\"is_plugin\":false,\"is_selectable\":true,\"is_suppressed\":false}]' ;;\n"+
		"  *' pipe '*) case \"$*\" in *-ready*) echo ready ;; *-snapshot*) cat >/dev/null; printf 'snapshot\\n' >> "+snapshotLog+"; echo accepted ;; esac ;;\n"+
		"esac\n")
	source := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.URL.Path != "/api/v1/events" {
			http.NotFound(w, r)
			return
		}
		w.Header().Set("Content-Type", "text/event-stream")
		w.(http.Flusher).Flush()
		<-r.Context().Done()
	}))
	defer source.Close()

	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()
	ready := make(chan WatchReady, 1)
	errs := make(chan error, 1)
	go func() {
		errs <- runWatchTab(ctx, WatchConfig{WatchRoute: WatchRoute{
			ServerURL: source.URL, ZellijBin: zellij, ZellijConfigDir: "/c",
			ZellijConfigFile: "/c/config.kdl", ZellijDataDir: "/d", ZellijSession: "managed",
			RailURL: "file:/candidate/sidebar.wasm", RecipientToken: "token",
			PipeTimeout: time.Second, SourceTimeout: time.Second, SummaryClampBytes: 512,
		}, PaneID: 7, SocketRoot: runtimeRoot, Lease: time.Second, Heartbeat: time.Hour,
			Ready: ready}, &bytes.Buffer{})
	}()
	select {
	case <-ready:
	case err := <-errs:
		t.Fatalf("watcher failed before ready: %v", err)
	case <-time.After(3 * time.Second):
		t.Fatal("watcher readiness timed out")
	}
	countLines := func(path string) int {
		contents, _ := os.ReadFile(path)
		return bytes.Count(contents, []byte("\n"))
	}
	if got := countLines(nativeLog); got != 1 {
		t.Fatalf("pre-ready native inventory calls = %d, want 1", got)
	}
	if got := countLines(snapshotLog); got != 1 {
		t.Fatalf("pre-ready snapshots = %d, want 1", got)
	}
	cancel()
	select {
	case err := <-errs:
		if err != nil {
			t.Fatalf("watcher cancellation = %v", err)
		}
	case <-time.After(time.Second):
		t.Fatal("watcher cleanup did not finish")
	}
	if got := countLines(nativeLog); got != 1 {
		t.Fatalf("cleanup native inventory calls changed count to %d, want 1", got)
	}
	if got := countLines(snapshotLog); got != 1 {
		t.Fatalf("cleanup emitted snapshot count %d, want 1", got)
	}
}

func TestTwoWatchersUseNativeInventoryOnlyForStartup(t *testing.T) {
	dir := t.TempDir()
	runtimeRoot, err := os.MkdirTemp("/tmp", "zwt.")
	if err != nil {
		t.Fatal(err)
	}
	t.Cleanup(func() { _ = os.RemoveAll(runtimeRoot) })
	panesPath := filepath.Join(dir, "panes.json")
	panes := `[
      {"id":50,"tab_id":73,"is_plugin":true,"plugin_url":"file:/candidate/sidebar.wasm","is_floating":false,"is_suppressed":false},
      {"id":7,"tab_id":73,"is_plugin":false,"is_selectable":true,"is_suppressed":false},
      {"id":60,"tab_id":81,"is_plugin":true,"plugin_url":"file:/candidate/sidebar.wasm","is_floating":false,"is_suppressed":false},
      {"id":8,"tab_id":81,"is_plugin":false,"is_selectable":true,"is_suppressed":false}
    ]`
	if err := os.WriteFile(panesPath, []byte(panes), 0o600); err != nil {
		t.Fatal(err)
	}
	lockPath := filepath.Join(dir, "native.lock")
	heartbeatLog := filepath.Join(dir, "heartbeats.log")
	nativeLog := filepath.Join(dir, "native.log")
	snapshotLog := filepath.Join(dir, "snapshots.log")
	zellij := writeScript(t, dir, "zellij", "#!/bin/sh\n"+
		"case \" $* \" in\n"+
		"  *' list-panes '*)\n"+
		"    while ! mkdir "+lockPath+" 2>/dev/null; do sleep 0.01; done\n"+
		"    trap 'rmdir "+lockPath+"' EXIT\n"+
		"    sleep 1.1\n"+
		"    printf 'probe\\n' >> "+nativeLog+"\n"+
		"    cat "+panesPath+" ;;\n"+
		"  *' pipe '*)\n"+
		"    case \"$*\" in\n"+
		"      *-ready*) echo ready ;;\n"+
		"      *-heartbeat*) printf '%s\\n' \"$*\" >> "+heartbeatLog+" ;;\n"+
		"      *-snapshot*) cat >/dev/null; printf '%s\\n' \"$*\" >> "+snapshotLog+"; echo accepted ;;\n"+
		"    esac ;;\n"+
		"esac\n")

	source := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		switch r.URL.Path {
		case "/api/v1/events":
			w.Header().Set("Content-Type", "text/event-stream")
			fmt.Fprint(w, "event: heartbeat\ndata: {}\n\n")
			w.(http.Flusher).Flush()
			<-r.Context().Done()
		case "/api/v1/sessions/codex:019f60ff-1111-7222-8333-444455556666",
			"/api/v1/sessions/codex:019f60ff-1111-7222-8333-444455556667":
			fmt.Fprintf(w, `{"id":%q,"agent":"codex","first_message":"SERIALIZED_NATIVE_ROW"}`, strings.TrimPrefix(r.URL.Path, "/api/v1/sessions/"))
		default:
			http.NotFound(w, r)
		}
	}))
	t.Cleanup(source.Close)

	type runningWatcher struct {
		pane   uint32
		cancel context.CancelFunc
		errs   chan error
	}
	start := func(pane uint32, token string) runningWatcher {
		t.Helper()
		ctx, cancel := context.WithCancel(context.Background())
		ready := make(chan WatchReady, 1)
		errs := make(chan error, 1)
		go func() {
			errs <- runWatchTab(ctx, WatchConfig{WatchRoute: WatchRoute{
				ServerURL: source.URL, ZellijBin: zellij, ZellijConfigDir: "/c",
				ZellijConfigFile: "/c/config.kdl", ZellijDataDir: "/d", ZellijSession: "managed",
				RailURL: "file:/candidate/sidebar.wasm", RecipientToken: token,
				PipeTimeout: time.Second, SourceTimeout: time.Second, SummaryClampBytes: 512,
			}, PaneID: pane, SocketRoot: runtimeRoot, Lease: 2500 * time.Millisecond,
				Heartbeat: time.Hour, Ready: ready}, &bytes.Buffer{})
		}()
		select {
		case <-ready:
		case err := <-errs:
			cancel()
			t.Fatalf("watcher %d failed before ready: %v", pane, err)
		case <-time.After(8 * time.Second):
			cancel()
			t.Fatalf("watcher %d readiness timed out", pane)
		}
		return runningWatcher{pane: pane, cancel: cancel, errs: errs}
	}
	a := start(7, "token-a")
	b := start(8, "token-b")
	watchers := []runningWatcher{a, b}
	t.Cleanup(func() {
		for _, watcher := range watchers {
			watcher.cancel()
		}
	})

	hooks := []struct {
		pane uint32
		id   string
	}{
		{7, "019f60ff-1111-7222-8333-444455556666"},
		{8, "019f60ff-1111-7222-8333-444455556667"},
	}
	for deadline := time.Now().Add(2 * time.Second); time.Now().Before(deadline); time.Sleep(10 * time.Millisecond) {
		heartbeats, _ := os.ReadFile(heartbeatLog)
		if bytes.Count(heartbeats, []byte("\n")) >= 2 {
			break
		}
	}
	if err := os.Remove(heartbeatLog); err != nil {
		t.Fatalf("reset startup heartbeat evidence: %v", err)
	}
	hookErrs := make(chan error, len(hooks))
	refreshStarted := time.Now()
	for _, hook := range hooks {
		hook := hook
		go func() {
			payload := []byte(`{"session_id":"` + hook.id + `","hook_event_name":"SessionStart","source":"startup"}`)
			hookErrs <- sendWatchHook(context.Background(), runtimeRoot, "managed", strconv.FormatUint(uint64(hook.pane), 10), payload)
		}()
	}
	for range hooks {
		if err := <-hookErrs; err != nil {
			t.Fatalf("send concurrent hook: %v", err)
		}
	}

	deadline := time.Now().Add(8 * time.Second)
	for time.Now().Before(deadline) {
		for _, watcher := range watchers {
			select {
			case err := <-watcher.errs:
				watcher.cancel()
				t.Fatalf("watcher %d died after startup inventory: %v", watcher.pane, err)
			default:
			}
		}
		contents, _ := os.ReadFile(snapshotLog)
		if bytes.Count(contents, []byte("\n")) >= 4 {
			break
		}
		time.Sleep(20 * time.Millisecond)
	}
	contents, _ := os.ReadFile(snapshotLog)
	if bytes.Count(contents, []byte("\n")) < 4 {
		t.Fatalf("two watchers did not complete their post-hook snapshots: %s", contents)
	}
	if elapsed := time.Since(refreshStarted); elapsed >= 2500*time.Millisecond {
		t.Fatalf("serialized post-hook refresh took %s, want less than the renewed 2.5s lease", elapsed)
	}
	heartbeats, _ := os.ReadFile(heartbeatLog)
	if got := bytes.Count(heartbeats, []byte("\n")); got != 2 {
		t.Fatalf("pre-refresh lease renewals = %d, want one per watcher", got)
	}
	nativeCalls, _ := os.ReadFile(nativeLog)
	if got := bytes.Count(nativeCalls, []byte("\n")); got != 2 {
		t.Fatalf("native inventory calls = %d, want one startup call per watcher", got)
	}
	for _, watcher := range watchers {
		watcher.cancel()
		select {
		case err := <-watcher.errs:
			if err != nil {
				t.Fatalf("watcher %d cleanup = %v", watcher.pane, err)
			}
		case <-time.After(time.Second):
			t.Fatalf("watcher %d cleanup timed out", watcher.pane)
		}
	}
	nativeCalls, _ = os.ReadFile(nativeLog)
	if got := bytes.Count(nativeCalls, []byte("\n")); got != 2 {
		t.Fatalf("cleanup changed native inventory count to %d, want 2", got)
	}
}
