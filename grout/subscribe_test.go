// ABOUTME: Covers the private Zaphod subscriber's one tab-bound session path.
// ABOUTME: The loopback source makes data_changed re-list before one targeted pipe.

package main

import (
	"context"
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"net/http"
	"net/http/httptest"
	"os"
	"path/filepath"
	"strings"
	"sync/atomic"
	"syscall"
	"testing"
	"time"
)

func fakeSubscriberZellij(t *testing.T, dir, log, panes string) string {
	t.Helper()
	panesPath := filepath.Join(dir, "panes.json")
	readyPath := filepath.Join(dir, "recipient-ready")
	snapshotPath := filepath.Join(dir, "snapshot.json")
	if err := os.WriteFile(panesPath, []byte(panes), 0o600); err != nil {
		t.Fatal(err)
	}
	return writeScript(t, dir, "zellij", "#!/bin/sh\n"+
		"for arg in \"$@\"; do\n"+
		"  if [ \"$arg\" = list-panes ]; then cat "+panesPath+"; exit 0; fi\n"+
		"  if [ \"$arg\" = pipe ]; then\n"+
		"    case \"$*\" in *agent-event-ready*) : > "+readyPath+"; echo ready; exit 0 ;; esac\n"+
		"    [ -f "+readyPath+" ] || exit 70\n"+
		"    { echo \"$#\"; for value in \"$@\"; do printf '%s\\n' \"$value\"; done; } >> "+log+"\n"+
		"    case \"$*\" in *agent-snapshot*) cat > "+snapshotPath+"; echo accepted; exit 0 ;; esac\n"+
		"    echo accepted\n"+
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
  {"id":7,"tab_id":73,"is_plugin":false,"is_selectable":true,"is_suppressed":false,"pane_cwd":null},
  {"id":9,"tab_id":81,"is_plugin":false,"is_selectable":true,"is_suppressed":false,"pane_cwd":"/work/foreign"}
	]`
	zellij := fakeSubscriberZellij(t, dir, argvLog, panes)
	startupReader, startupWriter, err := os.Pipe()
	if err != nil {
		t.Fatal(err)
	}
	defer startupReader.Close()
	startupFD, err := syscall.Dup(int(startupWriter.Fd()))
	if err != nil {
		t.Fatal(err)
	}
	if err := startupWriter.Close(); err != nil {
		t.Fatal(err)
	}
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
			lists.Add(1)
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
			CheckoutCWD:       "/work/managed",
			StartupFD:         startupFD,
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
	snapshotPayload, err := os.ReadFile(filepath.Join(dir, "snapshot.json"))
	if err != nil {
		t.Fatalf("initial snapshot stdin was not captured: %v", err)
	}
	var snapshot []SessionRow
	if err := json.Unmarshal(snapshotPayload, &snapshot); err != nil {
		t.Fatalf("initial snapshot stdin is not valid row JSON: %v", err)
	}
	if len(snapshot) != 1 || snapshot[0].ID != "session-1" || snapshot[0].Cwd != "/work/managed" {
		t.Fatalf("initial snapshot = %#v, want exact managed session row", snapshot)
	}

	invs := readInvocations(t, argvLog)
	if len(invs) != 2 {
		t.Fatalf("zellij pipe invocations = %d, want initial plus data_changed row", len(invs))
	}
	argv := invs[len(invs)-1]
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
		CheckoutCWD:       "/work/managed",
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

func TestSubscribeRejectsInvalidStreamBeforeReadiness(t *testing.T) {
	for _, tc := range []struct {
		name        string
		contentType string
		body        string
	}{
		{name: "wrong content type", contentType: "application/json"},
		{name: "immediate eof", contentType: "text/event-stream"},
		{name: "heartbeat then eof", contentType: "text/event-stream", body: "event: heartbeat\ndata: {}\n\n"},
	} {
		t.Run(tc.name, func(t *testing.T) {
			dir := t.TempDir()
			zellij := fakeSubscriberZellij(t, dir, filepath.Join(dir, "argv.log"), `[
  {"id":50,"tab_id":73,"is_plugin":true,"plugin_url":"file:/candidate/zellij-sidebar.wasm","is_floating":false,"is_suppressed":false},
  {"id":7,"tab_id":73,"is_plugin":false,"is_selectable":true,"is_suppressed":false,"pane_cwd":"/work/managed"}
]`)
			source := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
				switch r.URL.Path {
				case "/api/v1/sessions":
					fmt.Fprint(w, `{"sessions":[]}`)
				case "/api/v1/events":
					w.Header().Set("Content-Type", tc.contentType)
					fmt.Fprint(w, tc.body)
				default:
					http.NotFound(w, r)
				}
			}))
			defer source.Close()

			reader, writer, err := os.Pipe()
			if err != nil {
				t.Fatal(err)
			}
			err = runSubscribe(context.Background(), SubscribeConfig{
				ServerURL:         source.URL,
				ZellijBin:         zellij,
				ZellijConfigDir:   "/isolated/config",
				ZellijConfigFile:  "/isolated/config/config.kdl",
				ZellijDataDir:     "/isolated/data",
				ZellijSession:     "WORK",
				TabID:             "73",
				RailURL:           "file:/candidate/zellij-sidebar.wasm",
				CheckoutCWD:       "/work/managed",
				StartupFD:         int(writer.Fd()),
				PipeTimeout:       time.Second,
				SummaryClampBytes: 512,
			}, nil)
			_ = writer.Close()
			payload, readErr := io.ReadAll(reader)
			_ = reader.Close()
			if err == nil {
				t.Fatal("invalid stream unexpectedly succeeded")
			}
			if readErr != nil {
				t.Fatal(readErr)
			}
			if len(payload) != 0 {
				t.Fatalf("invalid stream signaled readiness: %q", payload)
			}
		})
	}
}

func TestSubscribeTimesOutStalledInitialRefresh(t *testing.T) {
	dir := t.TempDir()
	zellij := fakeSubscriberZellij(t, dir, filepath.Join(dir, "argv.log"), `[
  {"id":50,"tab_id":73,"is_plugin":true,"plugin_url":"file:/candidate/zellij-sidebar.wasm","is_floating":false,"is_suppressed":false},
  {"id":7,"tab_id":73,"is_plugin":false,"is_selectable":true,"is_suppressed":false}
]`)
	source := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		switch r.URL.Path {
		case "/api/v1/events":
			w.Header().Set("Content-Type", "text/event-stream")
			w.(http.Flusher).Flush()
			<-r.Context().Done()
		case "/api/v1/sessions":
			<-r.Context().Done()
		}
	}))
	defer source.Close()
	reader, writer, err := os.Pipe()
	if err != nil {
		t.Fatal(err)
	}
	err = runSubscribe(context.Background(), SubscribeConfig{
		ServerURL:         source.URL,
		ZellijBin:         zellij,
		ZellijConfigDir:   "/isolated/config",
		ZellijConfigFile:  "/isolated/config/config.kdl",
		ZellijDataDir:     "/isolated/data",
		ZellijSession:     "WORK",
		TabID:             "73",
		RailURL:           "file:/candidate/zellij-sidebar.wasm",
		CheckoutCWD:       "/work/managed",
		StartupFD:         int(writer.Fd()),
		SourceTimeout:     100 * time.Millisecond,
		PipeTimeout:       time.Second,
		SummaryClampBytes: 512,
	}, nil)
	_ = writer.Close()
	payload, readErr := io.ReadAll(reader)
	_ = reader.Close()
	if err == nil || !strings.Contains(err.Error(), "context deadline exceeded") {
		t.Fatalf("stalled refresh error = %v, want finite deadline", err)
	}
	if readErr != nil || len(payload) != 0 {
		t.Fatalf("stalled initial refresh signaled readiness: %q, %v", payload, readErr)
	}
}

func TestTabDeliveryRetriesUntilExactRecipientAcknowledges(t *testing.T) {
	dir := t.TempDir()
	countPath := filepath.Join(dir, "count")
	zellij := writeScript(t, dir, "zellij", "#!/bin/sh\n"+
		"count=0\n"+
		"[ ! -f "+countPath+" ] || count=$(cat "+countPath+")\n"+
		"count=$((count + 1))\n"+
		"echo $count > "+countPath+"\n"+
		"[ $count -lt 2 ] || echo accepted\n")
	err := EmitRowForTab(context.Background(), Config{
		ZellijBin:   zellij,
		PipeName:    "agent-event",
		PipeTimeout: time.Second,
	}, "session", SessionRow{Kind: "session", ID: "retry"}, "73", nil)
	if err != nil {
		t.Fatalf("acknowledged retry failed: %v", err)
	}
	count, err := os.ReadFile(countPath)
	if err != nil || strings.TrimSpace(string(count)) != "2" {
		t.Fatalf("delivery attempts = %q, %v; want one dropped attempt plus acknowledged retry", count, err)
	}
}

func TestSubscribeRejectsEOFDuringHandshake(t *testing.T) {
	for _, tc := range []struct {
		name          string
		delayReady    bool
		stallSessions bool
	}{
		{name: "recipient wait", delayReady: true},
		{name: "initial fetch", stallSessions: true},
	} {
		t.Run(tc.name, func(t *testing.T) {
			dir := t.TempDir()
			panesPath := filepath.Join(dir, "panes.json")
			if err := os.WriteFile(panesPath, []byte(`[
  {"id":50,"tab_id":73,"is_plugin":true,"plugin_url":"file:/candidate/zellij-sidebar.wasm","is_floating":false,"is_suppressed":false},
  {"id":7,"tab_id":73,"is_plugin":false,"is_selectable":true,"is_suppressed":false}
]`), 0o600); err != nil {
				t.Fatal(err)
			}
			readyDelay := ""
			if tc.delayReady {
				readyDelay = "sleep 2\n"
			}
			zellij := writeScript(t, dir, "zellij", "#!/bin/sh\n"+
				"for arg in \"$@\"; do\n"+
				"  if [ \"$arg\" = list-panes ]; then cat "+panesPath+"; exit 0; fi\n"+
				"done\n"+
				readyDelay+
				"echo ready\n")
			fetchStarted := make(chan struct{})
			source := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
				switch r.URL.Path {
				case "/api/v1/events":
					w.Header().Set("Content-Type", "text/event-stream")
					w.(http.Flusher).Flush()
					if tc.stallSessions {
						<-fetchStarted
					} else {
						time.Sleep(175 * time.Millisecond)
					}
				case "/api/v1/sessions":
					if !tc.stallSessions {
						t.Errorf("unexpected sessions request while recipient was unavailable")
						return
					}
					close(fetchStarted)
					<-r.Context().Done()
				default:
					http.NotFound(w, r)
				}
			}))
			defer source.Close()

			reader, writer, err := os.Pipe()
			if err != nil {
				t.Fatal(err)
			}
			err = runSubscribe(context.Background(), SubscribeConfig{
				ServerURL: source.URL, ZellijBin: zellij,
				ZellijConfigDir: "/isolated/config", ZellijConfigFile: "/isolated/config/config.kdl",
				ZellijDataDir: "/isolated/data", ZellijSession: "WORK", TabID: "73",
				RailURL: "file:/candidate/zellij-sidebar.wasm", CheckoutCWD: "/work/managed",
				StartupFD: int(writer.Fd()), SourceTimeout: time.Second, PipeTimeout: time.Second,
			}, nil)
			_ = writer.Close()
			payload, readErr := io.ReadAll(reader)
			_ = reader.Close()
			if !errors.Is(err, ErrSourceEOF) {
				t.Fatalf("handshake closure error = %v, want source EOF", err)
			}
			if readErr != nil || len(payload) != 0 {
				t.Fatalf("EOF during handshake signaled readiness: %q, %v", payload, readErr)
			}
		})
	}
}

func TestSubscribeSignalsReadyAfterAcknowledgedMultiRowSnapshot(t *testing.T) {
	dir := t.TempDir()
	panesPath := filepath.Join(dir, "panes.json")
	deliveryStarted := filepath.Join(dir, "delivery-started")
	releaseDelivery := filepath.Join(dir, "release-delivery")
	catchupDelivered := filepath.Join(dir, "catchup-delivered")
	if err := os.WriteFile(panesPath, []byte(`[
  {"id":50,"tab_id":73,"is_plugin":true,"plugin_url":"file:/candidate/zellij-sidebar.wasm","is_floating":false,"is_suppressed":false},
  {"id":7,"tab_id":73,"is_plugin":false,"is_selectable":true,"is_suppressed":false}
]`), 0o600); err != nil {
		t.Fatal(err)
	}
	zellij := writeScript(t, dir, "zellij", "#!/bin/sh\n"+
		"case \"$*\" in\n"+
		"  *list-panes*) cat "+panesPath+" ;;\n"+
		"  *agent-event-ready*) echo ready ;;\n"+
		"  *agent-snapshot*) : > "+deliveryStarted+"; while [ ! -f "+releaseDelivery+" ]; do sleep 0.01; done; echo accepted ;;\n"+
		"  *agent-event*) : > "+catchupDelivered+"; echo accepted ;;\n"+
		"esac\n")
	change := make(chan struct{})
	changeSent := make(chan struct{})
	var lists atomic.Int32
	source := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		switch r.URL.Path {
		case "/api/v1/events":
			w.Header().Set("Content-Type", "text/event-stream")
			w.(http.Flusher).Flush()
			<-change
			fmt.Fprint(w, "event: data_changed\ndata: {}\n\n")
			w.(http.Flusher).Flush()
			close(changeSent)
			<-r.Context().Done()
		case "/api/v1/sessions":
			if lists.Add(1) == 1 {
				fmt.Fprint(w, `{"sessions":[
 {"id":"one","cwd":"/work/managed","agent":"codex","created_at":"2026-07-13T00:00:00Z"},
 {"id":"two","cwd":"/work/managed","agent":"codex","created_at":"2026-07-13T00:00:00Z"}
]}`)
				return
			}
			fmt.Fprint(w, `{"sessions":[{"id":"catchup","cwd":"/work/managed","agent":"codex","created_at":"2026-07-13T00:00:00Z"}]}`)
		default:
			http.NotFound(w, r)
		}
	}))
	defer source.Close()
	reader, writer, err := os.Pipe()
	if err != nil {
		t.Fatal(err)
	}
	ctx, cancel := context.WithCancel(context.Background())
	errCh := make(chan error, 1)
	go func() {
		errCh <- runSubscribe(ctx, SubscribeConfig{
			ServerURL: source.URL, ZellijBin: zellij,
			ZellijConfigDir: "/isolated/config", ZellijConfigFile: "/isolated/config/config.kdl",
			ZellijDataDir: "/isolated/data", ZellijSession: "WORK", TabID: "73",
			RailURL: "file:/candidate/zellij-sidebar.wasm", CheckoutCWD: "/work/managed",
			StartupFD: int(writer.Fd()), SourceTimeout: time.Second, PipeTimeout: 2 * time.Second,
		}, nil)
	}()
	ready := make(chan string, 1)
	go func() {
		payload := make([]byte, 6)
		n, _ := reader.Read(payload)
		ready <- string(payload[:n])
	}()
	for attempt := 0; attempt < 100; attempt++ {
		if _, err := os.Stat(deliveryStarted); err == nil {
			break
		}
		time.Sleep(10 * time.Millisecond)
	}
	if _, err := os.Stat(deliveryStarted); err != nil {
		t.Fatalf("initial snapshot delivery never began: %v", err)
	}
	close(change)
	select {
	case <-changeSent:
	case <-time.After(time.Second):
		t.Fatal("data_changed was not emitted during snapshot delivery")
	}
	select {
	case payload := <-ready:
		t.Fatalf("startup signaled before snapshot acknowledgment: %q", payload)
	case <-time.After(100 * time.Millisecond):
	}
	if err := os.WriteFile(releaseDelivery, nil, 0o600); err != nil {
		t.Fatal(err)
	}
	select {
	case payload := <-ready:
		if payload != "ready\n" {
			t.Fatalf("startup payload = %q", payload)
		}
	case <-time.After(time.Second):
		t.Fatal("acknowledged multi-row snapshot did not release startup readiness")
	}
	if _, err := os.Stat(catchupDelivered); err != nil {
		t.Fatalf("queued data_changed was not acknowledged before readiness: %v", err)
	}
	cancel()
	_ = writer.Close()
	_ = reader.Close()
	if err := <-errCh; err != nil {
		t.Fatalf("subscriber cancellation = %v", err)
	}
}

func TestSubscribeDoesNotSignalWhenQueuedCatchupFails(t *testing.T) {
	dir := t.TempDir()
	panesPath := filepath.Join(dir, "panes.json")
	snapshotStarted := filepath.Join(dir, "snapshot-started")
	releaseSnapshot := filepath.Join(dir, "release-snapshot")
	if err := os.WriteFile(panesPath, []byte(`[
  {"id":50,"tab_id":73,"is_plugin":true,"plugin_url":"file:/candidate/zellij-sidebar.wasm","is_floating":false,"is_suppressed":false},
  {"id":7,"tab_id":73,"is_plugin":false,"is_selectable":true,"is_suppressed":false}
]`), 0o600); err != nil {
		t.Fatal(err)
	}
	zellij := writeScript(t, dir, "zellij", "#!/bin/sh\n"+
		"case \"$*\" in\n"+
		"  *list-panes*) cat "+panesPath+" ;;\n"+
		"  *agent-event-ready*) echo ready ;;\n"+
		"  *agent-snapshot*) : > "+snapshotStarted+"; while [ ! -f "+releaseSnapshot+" ]; do sleep 0.01; done; echo accepted ;;\n"+
		"  *agent-event*) echo accepted ;;\n"+
		"esac\n")
	change := make(chan struct{})
	changeSent := make(chan struct{})
	var lists atomic.Int32
	source := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		switch r.URL.Path {
		case "/api/v1/events":
			w.Header().Set("Content-Type", "text/event-stream")
			w.(http.Flusher).Flush()
			<-change
			fmt.Fprint(w, "event: data_changed\ndata: {}\n\n")
			w.(http.Flusher).Flush()
			close(changeSent)
			<-r.Context().Done()
		case "/api/v1/sessions":
			if lists.Add(1) == 1 {
				fmt.Fprint(w, `{"sessions":[]}`)
				return
			}
			http.Error(w, "catchup failed", http.StatusServiceUnavailable)
		default:
			http.NotFound(w, r)
		}
	}))
	defer source.Close()
	reader, writer, err := os.Pipe()
	if err != nil {
		t.Fatal(err)
	}
	errCh := make(chan error, 1)
	go func() {
		errCh <- runSubscribe(context.Background(), SubscribeConfig{
			ServerURL: source.URL, ZellijBin: zellij,
			ZellijConfigDir: "/isolated/config", ZellijConfigFile: "/isolated/config/config.kdl",
			ZellijDataDir: "/isolated/data", ZellijSession: "WORK", TabID: "73",
			RailURL: "file:/candidate/zellij-sidebar.wasm", CheckoutCWD: "/work/managed",
			StartupFD: int(writer.Fd()), SourceTimeout: time.Second, PipeTimeout: 2 * time.Second,
		}, nil)
	}()
	for attempt := 0; attempt < 100; attempt++ {
		if _, err := os.Stat(snapshotStarted); err == nil {
			break
		}
		time.Sleep(10 * time.Millisecond)
	}
	if _, err := os.Stat(snapshotStarted); err != nil {
		t.Fatalf("initial snapshot delivery never began: %v", err)
	}
	close(change)
	select {
	case <-changeSent:
	case <-time.After(time.Second):
		t.Fatal("queued change was not emitted")
	}
	time.Sleep(50 * time.Millisecond)
	if err := os.WriteFile(releaseSnapshot, nil, 0o600); err != nil {
		t.Fatal(err)
	}
	err = <-errCh
	_ = writer.Close()
	payload, readErr := io.ReadAll(reader)
	_ = reader.Close()
	if err == nil || !strings.Contains(err.Error(), "503") {
		t.Fatalf("queued catchup error = %v, want source failure", err)
	}
	if readErr != nil || len(payload) != 0 {
		t.Fatalf("failed queued catchup signaled readiness: %q, %v", payload, readErr)
	}
}
