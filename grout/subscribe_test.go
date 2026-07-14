// ABOUTME: Covers the private Zaphod subscriber's one tab-bound session path.
// ABOUTME: The loopback source makes data_changed rebuild one exact registered snapshot.

package main

import (
	"bufio"
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
	"sync"
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
		"    case \"$*\" in *zaphod-agent-v1-*-ready*) : > "+readyPath+"; echo ready; exit 0 ;; esac\n"+
		"    [ -f "+readyPath+" ] || exit 70\n"+
		"    { echo \"$#\"; for value in \"$@\"; do printf '%s\\n' \"$value\"; done; } >> "+log+"\n"+
		"    case \"$*\" in *zaphod-agent-v1-*-snapshot*) cat > "+snapshotPath+"; echo accepted; exit 0 ;; esac\n"+
		"    echo accepted\n"+
		"    exit 0\n"+
		"  fi\n"+
		"done\n"+
		"echo unexpected zellij invocation >&2\nexit 64\n")
}

func TestStartupStreamQuietRequiresEveryScannedLineConsumed(t *testing.T) {
	if startupStreamQuiet(1, 0, "") {
		t.Fatal("a scanned line queued for consumption was treated as quiet")
	}
	if startupStreamQuiet(1, 1, "data_changed") {
		t.Fatal("an incomplete SSE event was treated as quiet")
	}
	if !startupStreamQuiet(1, 1, "") {
		t.Fatal("a fully consumed event boundary was not treated as quiet")
	}
}

type failingReader struct{}

func (failingReader) Read([]byte) (int, error) { return 0, errors.New("transport failed") }

func TestReadinessReaderPublishesFailureBeforeReturning(t *testing.T) {
	var boundary sync.Mutex
	var activity atomic.Uint64
	var ended atomic.Bool
	var pending atomic.Bool
	observed := false
	reader := readinessReader{
		reader: failingReader{}, boundary: &boundary, activity: &activity, ended: &ended, pending: &pending,
		afterRead: func(_ int, err error) { observed = err != nil && ended.Load() },
	}
	if _, err := reader.Read(make([]byte, 1)); err == nil {
		t.Fatal("failing transport unexpectedly succeeded")
	}
	if !observed {
		t.Fatal("transport failure was returned before readiness state published it")
	}
}

func TestScanLinesReportsTrailingFragmentAfterCompleteToken(t *testing.T) {
	data := []byte(": keepalive\nevent: data_")
	advance, token, err := bufio.ScanLines(data, false)
	if err != nil || string(token) != ": keepalive" || len(data) <= advance {
		t.Fatalf("advance=%d token=%q err=%v; want complete token plus retained fragment", advance, token, err)
	}
}

func TestRecipientProbeAllowsMoreThanQuarterSecond(t *testing.T) {
	dir := t.TempDir()
	zellij := writeScript(t, dir, "zellij", "#!/bin/sh\nsleep 0.4\necho ready\n")
	err := waitForRecipient(context.Background(), SubscribeConfig{
		ZellijBin: zellij, ZellijConfigDir: "/c", ZellijConfigFile: "/c/config.kdl",
		ZellijDataDir: "/d", ZellijSession: "s", TabID: "73", RecipientToken: "token",
		PipeTimeout: time.Second,
	})
	if err != nil {
		t.Fatalf("delayed healthy recipient failed: %v", err)
	}
}

func TestRecipientProbeUsesPrivateVersionedPipe(t *testing.T) {
	dir := t.TempDir()
	argsPath := filepath.Join(dir, "args")
	zellij := writeScript(t, dir, "zellij", "#!/bin/sh\n"+
		"printf '%s\\n' \"$*\" > "+argsPath+"\n"+
		"case \"$*\" in *'--name zaphod-agent-v1-test-token-ready'*) echo ready ;; esac\n")
	err := waitForRecipient(context.Background(), SubscribeConfig{
		ZellijBin: zellij, ZellijConfigDir: "/c", ZellijConfigFile: "/c/config.kdl",
		ZellijDataDir: "/d", ZellijSession: "s", TabID: "73", RecipientToken: "test-token",
		PipeTimeout: time.Second, recipientWaitTimeout: time.Second,
	})
	if err != nil {
		args, _ := os.ReadFile(argsPath)
		t.Fatalf("private ready pipe was not used: %v; argv=%q", err, args)
	}
}

func TestRecipientProbeHonorsOverallWaitDeadline(t *testing.T) {
	dir := t.TempDir()
	zellij := writeScript(t, dir, "zellij", "#!/bin/sh\nsleep 1 &\nwait\n")
	started := time.Now()
	err := waitForRecipient(context.Background(), SubscribeConfig{
		ZellijBin: zellij, ZellijConfigDir: "/c", ZellijConfigFile: "/c/config.kdl",
		ZellijDataDir: "/d", ZellijSession: "s", TabID: "73", RecipientToken: "token",
		PipeTimeout: time.Second, recipientWaitTimeout: 100 * time.Millisecond,
	})
	if err == nil || !strings.Contains(err.Error(), "recipient-ready timeout") {
		t.Fatalf("hanging recipient error = %v; want overall wait timeout", err)
	}
	if elapsed := time.Since(started); elapsed > 300*time.Millisecond {
		t.Fatalf("hanging recipient exceeded overall wait deadline: %s", elapsed)
	}
}

func TestSubscribeDoesNotSignalWithScannedLineQueuedAtSettle(t *testing.T) {
	dir := t.TempDir()
	panesPath := filepath.Join(dir, "panes.json")
	if err := os.WriteFile(panesPath, []byte(`[
  {"id":50,"tab_id":73,"is_plugin":true,"plugin_url":"file:/candidate/zellij-sidebar.wasm","is_floating":false,"is_suppressed":false},
  {"id":7,"tab_id":73,"is_plugin":false,"is_selectable":true,"is_suppressed":false}
]`), 0o600); err != nil {
		t.Fatal(err)
	}
	zellij := writeScript(t, dir, "zellij", "#!/bin/sh\n"+
		"case \"$*\" in\n"+
		"  *list-panes*) cat "+panesPath+" ;;\n"+
		"  *zaphod-agent-v1-*-ready*) echo ready ;;\n"+
		"  *zaphod-agent-v1-*-snapshot*) cat >/dev/null; echo accepted ;;\n"+
		"esac\n")
	emitPartial := make(chan struct{})
	finishEvent := make(chan struct{})
	source := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		switch r.URL.Path {
		case "/api/v1/events":
			w.Header().Set("Content-Type", "text/event-stream")
			w.(http.Flusher).Flush()
			select {
			case <-emitPartial:
			case <-r.Context().Done():
				return
			}
			fmt.Fprint(w, ": keepalive\n")
			w.(http.Flusher).Flush()
			select {
			case <-finishEvent:
			case <-r.Context().Done():
				return
			}
			fmt.Fprint(w, "event: data_changed\ndata: {}\n\n")
			w.(http.Flusher).Flush()
			<-r.Context().Done()
		case "/api/v1/sessions":
			fmt.Fprint(w, `{"sessions":[]}`)
		default:
			http.NotFound(w, r)
		}
	}))
	defer source.Close()
	checkStarted := make(chan struct{})
	releaseCheck := make(chan struct{})
	var checkOnce sync.Once
	fragmentRead := make(chan struct{})
	var readNotified atomic.Bool
	splitStarted := make(chan struct{})
	releaseSplit := make(chan struct{})
	var splitOnce sync.Once
	keepaliveConsumed := make(chan struct{})
	releaseNextScan := make(chan struct{})
	var tokenOnce sync.Once
	reader, writer, err := os.Pipe()
	if err != nil {
		t.Fatal(err)
	}
	defer reader.Close()
	defer writer.Close()
	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()
	errCh := make(chan error, 1)
	go func() {
		errCh <- runSubscribe(ctx, SubscribeConfig{
			ServerURL: source.URL, ZellijBin: zellij,
			ZellijConfigDir: "/isolated/config", ZellijConfigFile: "/isolated/config/config.kdl",
			ZellijDataDir: "/isolated/data", ZellijSession: "WORK", TabID: "73",
			RailURL: "file:/candidate/zellij-sidebar.wasm", CheckoutCWD: "/work/managed", RecipientToken: "test-token",
			StartupFD: int(writer.Fd()), SourceTimeout: time.Second, PipeTimeout: time.Second,
			afterRead: func(n int, _ error) {
				if n > 0 && readNotified.CompareAndSwap(false, true) {
					close(fragmentRead)
				}
			},
			beforeReadinessCheck: func() {
				checkOnce.Do(func() {
					close(checkStarted)
					select {
					case <-releaseCheck:
					case <-ctx.Done():
					}
				})
			},
			beforeSplit: func() {
				splitOnce.Do(func() {
					close(splitStarted)
					select {
					case <-releaseSplit:
					case <-ctx.Done():
					}
				})
			},
			beforeLineSend: func() {
				tokenOnce.Do(func() {
					close(keepaliveConsumed)
					select {
					case <-releaseNextScan:
					case <-ctx.Done():
					}
				})
			},
		}, nil)
	}()
	ready := make(chan string, 1)
	go func() {
		payload := make([]byte, 6)
		n, _ := reader.Read(payload)
		ready <- string(payload[:n])
	}()
	select {
	case <-checkStarted:
	case <-time.After(time.Second):
		t.Fatal("readiness check did not reach the settle boundary")
	}
	close(emitPartial)
	select {
	case <-fragmentRead:
	case <-time.After(time.Second):
		t.Fatal("fragmented event bytes were not read while readiness was paused")
	}
	close(releaseCheck)
	select {
	case <-splitStarted:
	case <-time.After(time.Second):
		t.Fatal("scanner did not pause before accounting for the read")
	}
	select {
	case payload := <-ready:
		t.Fatalf("unprocessed transport read allowed readiness: %q", payload)
	case <-time.After(75 * time.Millisecond):
	}
	close(releaseSplit)
	select {
	case <-keepaliveConsumed:
	case <-time.After(time.Second):
		t.Fatal("complete keepalive was not consumed before the next split")
	}
	select {
	case payload := <-ready:
		t.Fatalf("queued scanned line allowed readiness: %q", payload)
	case <-time.After(75 * time.Millisecond):
	}
	close(releaseNextScan)
	close(finishEvent)
	select {
	case payload := <-ready:
		if payload != "ready\n" {
			t.Fatalf("startup payload = %q", payload)
		}
	case <-time.After(time.Second):
		t.Fatal("consumed event and catch-up did not release readiness")
	}
	cancel()
	_ = writer.Close()
	_ = reader.Close()
	if err := <-errCh; err != nil {
		t.Fatalf("subscriber cancellation = %v", err)
	}
}

func TestSubscribeDoesNotSignalBetweenTokenPendingAndActivityPublication(t *testing.T) {
	dir := t.TempDir()
	panesPath := filepath.Join(dir, "panes.json")
	if err := os.WriteFile(panesPath, []byte(`[
  {"id":50,"tab_id":73,"is_plugin":true,"plugin_url":"file:/candidate/zellij-sidebar.wasm","is_floating":false,"is_suppressed":false},
  {"id":7,"tab_id":73,"is_plugin":false,"is_selectable":true,"is_suppressed":false}
]`), 0o600); err != nil {
		t.Fatal(err)
	}
	zellij := writeScript(t, dir, "zellij", "#!/bin/sh\n"+
		"case \"$*\" in\n"+
		"  *list-panes*) cat "+panesPath+" ;;\n"+
		"  *zaphod-agent-v1-*-ready*) echo ready ;;\n"+
		"  *zaphod-agent-v1-*-snapshot*) cat >/dev/null; echo accepted ;;\n"+
		"esac\n")
	emitToken := make(chan struct{})
	source := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		switch r.URL.Path {
		case "/api/v1/events":
			w.Header().Set("Content-Type", "text/event-stream")
			w.(http.Flusher).Flush()
			select {
			case <-emitToken:
			case <-r.Context().Done():
				return
			}
			fmt.Fprint(w, ": keepalive\n")
			w.(http.Flusher).Flush()
			<-r.Context().Done()
		case "/api/v1/sessions":
			fmt.Fprint(w, `{"sessions":[]}`)
		default:
			http.NotFound(w, r)
		}
	}))
	defer source.Close()
	checkStarted := make(chan struct{})
	releaseCheck := make(chan struct{})
	var checkOnce sync.Once
	gapStarted := make(chan struct{})
	releaseGap := make(chan struct{})
	var gapOnce sync.Once
	reader, writer, err := os.Pipe()
	if err != nil {
		t.Fatal(err)
	}
	defer reader.Close()
	defer writer.Close()
	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()
	errCh := make(chan error, 1)
	go func() {
		errCh <- runSubscribe(ctx, SubscribeConfig{
			ServerURL: source.URL, ZellijBin: zellij,
			ZellijConfigDir: "/isolated/config", ZellijConfigFile: "/isolated/config/config.kdl",
			ZellijDataDir: "/isolated/data", ZellijSession: "WORK", TabID: "73",
			RailURL: "file:/candidate/zellij-sidebar.wasm", CheckoutCWD: "/work/managed", RecipientToken: "test-token",
			StartupFD: int(writer.Fd()), SourceTimeout: time.Second, PipeTimeout: time.Second,
			beforeReadinessCheck: func() {
				checkOnce.Do(func() {
					close(checkStarted)
					select {
					case <-releaseCheck:
					case <-ctx.Done():
					}
				})
			},
			afterTokenPendingClear: func() {
				gapOnce.Do(func() {
					close(gapStarted)
					select {
					case <-releaseGap:
					case <-ctx.Done():
					}
				})
			},
		}, nil)
	}()
	ready := make(chan string, 1)
	go func() {
		payload := make([]byte, 6)
		n, _ := reader.Read(payload)
		ready <- string(payload[:n])
	}()
	select {
	case <-checkStarted:
	case <-time.After(time.Second):
		t.Fatal("readiness check did not reach the settle boundary")
	}
	close(emitToken)
	select {
	case <-gapStarted:
	case <-time.After(time.Second):
		t.Fatal("scanner did not pause in the token publication gap")
	}
	close(releaseCheck)
	select {
	case payload := <-ready:
		t.Fatalf("token publication gap allowed readiness: %q", payload)
	case <-time.After(75 * time.Millisecond):
	}
	close(releaseGap)
	select {
	case payload := <-ready:
		if payload != "ready\n" {
			t.Fatalf("startup payload = %q", payload)
		}
	case <-time.After(time.Second):
		t.Fatal("published keepalive activity did not release readiness")
	}
	cancel()
	_ = writer.Close()
	_ = reader.Close()
	if err := <-errCh; err != nil {
		t.Fatalf("subscriber cancellation = %v", err)
	}
}

func TestSubscribeRefreshesAfterTransientLayoutReplyAndStaysAlive(t *testing.T) {
	dir := t.TempDir()
	argvLog := filepath.Join(dir, "zellij-argv.log")
	const railURL = "file:/candidate/zellij-sidebar.wasm"
	const panes = `[
  {"id":50,"tab_id":73,"is_plugin":true,"plugin_url":"file:/candidate/zellij-sidebar.wasm","is_floating":false,"is_suppressed":false},
  {"id":7,"tab_id":73,"is_plugin":false,"is_selectable":true,"is_suppressed":false,"pane_cwd":null},
  {"id":9,"tab_id":81,"is_plugin":false,"is_selectable":true,"is_suppressed":false,"pane_cwd":"/work/foreign"}
	]`
	panesPath := filepath.Join(dir, "panes.json")
	readyPath := filepath.Join(dir, "recipient-ready")
	snapshotPath := filepath.Join(dir, "snapshot.json")
	injectMalformed := filepath.Join(dir, "inject-malformed")
	injected := filepath.Join(dir, "malformed-injected")
	if err := os.WriteFile(panesPath, []byte(panes), 0o600); err != nil {
		t.Fatal(err)
	}
	zellij := writeScript(t, dir, "zellij", "#!/bin/sh\n"+
		"for arg in \"$@\"; do\n"+
		"  if [ \"$arg\" = list-panes ]; then\n"+
		"    if [ -e "+injectMalformed+" ] && mkdir "+injected+" 2>/dev/null; then printf 'layout { pane; }\\n'; exit 0; fi\n"+
		"    cat "+panesPath+"; exit 0\n"+
		"  fi\n"+
		"  if [ \"$arg\" = pipe ]; then\n"+
		"    case \"$*\" in *zaphod-agent-v1-*-ready*) : > "+readyPath+"; echo ready; exit 0 ;; esac\n"+
		"    [ -f "+readyPath+" ] || exit 70\n"+
		"    { echo \"$#\"; for value in \"$@\"; do printf '%s\\n' \"$value\"; done; } >> "+argvLog+"\n"+
		"    case \"$*\" in *zaphod-agent-v1-*-snapshot*) cat > "+snapshotPath+"; echo accepted; exit 0 ;; esac\n"+
		"    echo accepted; exit 0\n"+
		"  fi\n"+
		"done\nexit 64\n")
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
	registryDir := filepath.Join(dir, "registry")
	const registeredID = "019f5f94-a596-7d92-9928-398653669161"
	if err := (agentRegistryStore{root: registryDir}).upsert(registrationForTest(t, registeredID, "WORK", "7")); err != nil {
		t.Fatal(err)
	}
	source := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		switch r.URL.Path {
		case "/api/v1/sessions/codex:" + registeredID:
			w.Header().Set("Content-Type", "application/json")
			if lists.Add(1) == 1 {
				fmt.Fprintf(w, `{"id":"codex:%s","cwd":"/wrong/cwd","agent":"codex","termination_status":"awaiting_user","first_message":"initial marker","created_at":"2026-07-13T00:00:00Z"}`, registeredID)
			} else {
				fmt.Fprintf(w, `{"id":"codex:%s","cwd":"/wrong/cwd","agent":"codex","termination_status":"awaiting_user","first_message":"fq-second-marker","created_at":"2026-07-14T00:00:00Z"}`, registeredID)
			}
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
			<-r.Context().Done()
		default:
			t.Errorf("unexpected source path %q", r.URL.Path)
			http.NotFound(w, r)
		}
	}))
	defer source.Close()

	ctx, cancel := context.WithTimeout(context.Background(), 3*time.Second)
	defer cancel()
	var diagnostics strings.Builder
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
			RecipientToken:    "test-token",
			RegistryDir:       registryDir,
			StartupFD:         startupFD,
			PipeTimeout:       time.Second,
			SummaryClampBytes: 512,
		}, &diagnostics)
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
	if err := os.WriteFile(injectMalformed, nil, 0o600); err != nil {
		t.Fatal(err)
	}
	close(changed)
	var eventLog []byte
	var refreshedPayload []byte
	for attempt := 0; attempt < 100; attempt++ {
		select {
		case err := <-errCh:
			t.Fatalf("subscriber exited before second marked session: %v", err)
		default:
		}
		eventLog, _ = os.ReadFile(argvLog)
		refreshedPayload, _ = os.ReadFile(snapshotPath)
		if strings.Contains(string(refreshedPayload), "fq-second-marker") {
			break
		}
		time.Sleep(10 * time.Millisecond)
	}
	if !strings.Contains(string(refreshedPayload), "fq-second-marker") {
		t.Fatalf("second marked session was not delivered: argv=%q snapshot=%q", eventLog, refreshedPayload)
	}
	select {
	case err := <-errCh:
		t.Fatalf("subscriber exited after second marked session: %v", err)
	default:
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
	if len(snapshot) != 1 || snapshot[0].ID != "codex:"+registeredID || snapshot[0].PaneID == nil || *snapshot[0].PaneID != 7 || snapshot[0].Summary != "fq-second-marker" {
		t.Fatalf("refreshed snapshot = %#v, want exact registered session row", snapshot)
	}

	invs := readInvocations(t, argvLog)
	if len(invs) != 2 {
		t.Fatalf("zellij pipe invocations = %d, want initial plus data_changed row", len(invs))
	}
	if got := strings.Join(invs[0], "\x00"); !strings.Contains(got, "pipe\x00--name\x00zaphod-agent-v1-test-token-snapshot") {
		t.Fatalf("snapshot argv = %q, want private versioned pipe", invs[0])
	}
	argv := invs[len(invs)-1]
	wantPrefix := []string{
		"--config-dir", "/isolated/config",
		"--config", "/isolated/config/config.kdl",
		"--data-dir", "/isolated/data",
		"--session", "WORK",
		"pipe", "--name", "zaphod-agent-v1-test-token-snapshot",
		"--args", "recipient-tab-id=73,recipient-token=test-token",
	}
	if len(argv) != len(wantPrefix) || strings.Join(argv, "\x00") != strings.Join(wantPrefix, "\x00") {
		t.Fatalf("pipe argv = %q, want snapshot argv %q", argv, wantPrefix)
	}
	if strings.Contains(strings.Join(argv, "\x00"), "--plugin") {
		t.Fatalf("pipe argv unexpectedly names a plugin: %q", argv)
	}
	cancel()
	if err := <-errCh; err != nil {
		t.Fatalf("subscriber cancellation = %v", err)
	}
	diagnosticLog := diagnostics.String()
	for _, want := range []string{
		"transient-native-pane-reply", "command=list-panes", "attempt=1/3",
		`stdout_prefix="layout { pane; }\n"`,
	} {
		if !strings.Contains(diagnosticLog, want) {
			t.Fatalf("surviving transient diagnostic = %q, want %q", diagnosticLog, want)
		}
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
		RecipientToken:    "test-token",
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

func TestProbeTargetRetriesOneTransientEmptyNativeReply(t *testing.T) {
	dir := t.TempDir()
	seen := filepath.Join(dir, "seen")
	panes := filepath.Join(dir, "panes.json")
	if err := os.WriteFile(panes, []byte(`[
  {"id":50,"tab_id":73,"is_plugin":true,"plugin_url":"file:/candidate/zellij-sidebar.wasm","is_floating":false,"is_suppressed":false},
  {"id":7,"tab_id":73,"is_plugin":false,"is_selectable":true,"is_suppressed":false}
]`), 0o600); err != nil {
		t.Fatal(err)
	}
	zellij := writeScript(t, dir, "zellij", "#!/bin/sh\n"+
		"if [ ! -e "+seen+" ]; then : > "+seen+"; exit 0; fi\n"+
		"cat "+panes+"\n")
	got, err := probeTarget(context.Background(), SubscribeConfig{
		ZellijBin: zellij, ZellijConfigDir: "/c", ZellijConfigFile: "/c/config.kdl",
		ZellijDataDir: "/d", ZellijSession: "WORK", RailURL: "file:/candidate/zellij-sidebar.wasm",
		CheckoutCWD: "/work/managed",
	}, 73)
	if err != nil {
		t.Fatalf("transient empty list-panes reply was terminal: %v", err)
	}
	if got.paneTabs[7] != 73 {
		t.Fatalf("target pane map = %#v, want pane 7 in tab 73", got.paneTabs)
	}
}

func TestProbeTargetRetriesOneTransientNativeEmptyInventory(t *testing.T) {
	dir := t.TempDir()
	seen := filepath.Join(dir, "seen")
	panes := filepath.Join(dir, "panes.json")
	if err := os.WriteFile(panes, []byte(`[
  {"id":50,"tab_id":73,"is_plugin":true,"plugin_url":"file:/candidate/zellij-sidebar.wasm","is_floating":false,"is_suppressed":false},
  {"id":7,"tab_id":73,"is_plugin":false,"is_selectable":true,"is_suppressed":false}
]`), 0o600); err != nil {
		t.Fatal(err)
	}
	zellij := writeScript(t, dir, "zellij", "#!/bin/sh\n"+
		"if [ ! -e "+seen+" ]; then : > "+seen+"; printf '[]\\n'; exit 0; fi\n"+
		"cat "+panes+"\n")
	got, err := probeTarget(context.Background(), SubscribeConfig{
		ZellijBin: zellij, ZellijConfigDir: "/c", ZellijConfigFile: "/c/config.kdl",
		ZellijDataDir: "/d", ZellijSession: "WORK", RailURL: "file:/candidate/zellij-sidebar.wasm",
		CheckoutCWD: "/work/managed",
	}, 73)
	if err != nil {
		t.Fatalf("transient native [] list-panes reply was terminal: %v", err)
	}
	if got.paneTabs[7] != 73 {
		t.Fatalf("target pane map = %#v, want pane 7 in tab 73", got.paneTabs)
	}
}

func TestProbeTargetMalformedErrorPreservesBoundedNativeReplyProvenance(t *testing.T) {
	dir := t.TempDir()
	stdout := "layout { pane; }\n" + strings.Repeat("x", 300)
	stderr := "route saturated\n"
	zellij := writeScript(t, dir, "zellij", "#!/bin/sh\n"+
		"printf '%s' '"+stdout+"'\n"+
		"printf '%s' '"+stderr+"' >&2\n")
	_, err := probeTarget(context.Background(), SubscribeConfig{
		ZellijBin: zellij, ZellijConfigDir: "/c", ZellijConfigFile: "/c/config.kdl",
		ZellijDataDir: "/d", ZellijSession: "WORK", RailURL: "file:/candidate/zellij-sidebar.wasm",
		CheckoutCWD: "/work/managed",
	}, 73)
	if err == nil {
		t.Fatal("malformed native pane state unexpectedly succeeded")
	}
	message := err.Error()
	for _, want := range []string{
		"command=list-panes", "attempt=1/3", "stdout_len=317", "stderr_len=16",
		`stdout_prefix="layout { pane; }\n`, `stderr_prefix="route saturated\n"`,
	} {
		if !strings.Contains(message, want) {
			t.Fatalf("malformed error = %q, want provenance %q", message, want)
		}
	}
	if strings.Contains(message, strings.Repeat("x", 257)) {
		t.Fatalf("malformed error leaked unbounded stdout: %q", message)
	}
}

func TestProbeTargetPersistentLayoutReplyFailsAfterBoundedRetries(t *testing.T) {
	dir := t.TempDir()
	attempts := filepath.Join(dir, "attempts")
	zellij := writeScript(t, dir, "zellij", "#!/bin/sh\n"+
		"printf x >> "+attempts+"\n"+
		"printf 'layout { pane; }\\n'\n")
	_, err := probeTarget(context.Background(), SubscribeConfig{
		ZellijBin: zellij, ZellijConfigDir: "/c", ZellijConfigFile: "/c/config.kdl",
		ZellijDataDir: "/d", ZellijSession: "WORK", RailURL: "file:/candidate/zellij-sidebar.wasm",
		CheckoutCWD: "/work/managed",
	}, 73)
	if !errors.Is(err, ErrTargetLost) {
		t.Fatalf("persistent layout reply error = %v, want target-lost", err)
	}
	for _, want := range []string{
		"malformed native pane state", "command=list-panes", "attempt=3/3",
		`stdout_prefix="layout { pane; }\n"`,
	} {
		if !strings.Contains(err.Error(), want) {
			t.Fatalf("persistent layout reply error = %q, want %q", err, want)
		}
	}
	got, readErr := os.ReadFile(attempts)
	if readErr != nil {
		t.Fatal(readErr)
	}
	if string(got) != "xxx" {
		t.Fatalf("list-panes attempts = %q, want three bounded attempts", got)
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
				RecipientToken:    "test-token",
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
	registryDir := filepath.Join(dir, "registry")
	const registeredID = "019f5f94-a596-7d92-9928-398653669161"
	if err := (agentRegistryStore{root: registryDir}).upsert(registrationForTest(t, registeredID, "WORK", "7")); err != nil {
		t.Fatal(err)
	}
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
		case "/api/v1/sessions/codex:" + registeredID:
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
		RecipientToken:    "test-token",
		RegistryDir:       registryDir,
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
	}, "session", SessionRow{Kind: "session", ID: "retry"}, "73", "test-token", nil)
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
			registryDir := filepath.Join(dir, "registry")
			const registeredID = "019f5f94-a596-7d92-9928-398653669161"
			if tc.stallSessions {
				if err := (agentRegistryStore{root: registryDir}).upsert(registrationForTest(t, registeredID, "WORK", "7")); err != nil {
					t.Fatal(err)
				}
			}
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
				case "/api/v1/sessions/codex:" + registeredID:
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
				RailURL: "file:/candidate/zellij-sidebar.wasm", CheckoutCWD: "/work/managed", RecipientToken: "test-token",
				RegistryDir: registryDir, StartupFD: int(writer.Fd()), SourceTimeout: time.Second, PipeTimeout: time.Second,
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
	  {"id":7,"tab_id":73,"is_plugin":false,"is_selectable":true,"is_suppressed":false},
	  {"id":8,"tab_id":73,"is_plugin":false,"is_selectable":true,"is_suppressed":false}
]`), 0o600); err != nil {
		t.Fatal(err)
	}
	registryDir := filepath.Join(dir, "registry")
	const firstID = "019f5f94-a596-7d92-9928-398653669161"
	const secondID = "019f5f95-bbfd-7993-8620-0d698008217f"
	if err := (agentRegistryStore{root: registryDir}).upsert(registrationForTest(t, firstID, "WORK", "7")); err != nil {
		t.Fatal(err)
	}
	if err := (agentRegistryStore{root: registryDir}).upsert(registrationForTest(t, secondID, "WORK", "8")); err != nil {
		t.Fatal(err)
	}
	zellij := writeScript(t, dir, "zellij", "#!/bin/sh\n"+
		"case \"$*\" in\n"+
		"  *list-panes*) cat "+panesPath+" ;;\n"+
		"  *zaphod-agent-v1-*-ready*) echo ready ;;\n"+
		"  *zaphod-agent-v1-*-snapshot*) cat >/dev/null; if [ -f "+deliveryStarted+" ]; then : > "+catchupDelivered+"; else : > "+deliveryStarted+"; fi; while [ ! -f "+releaseDelivery+" ]; do sleep 0.01; done; echo accepted ;;\n"+
		"esac\n")
	change := make(chan struct{})
	changeSent := make(chan struct{})
	source := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		switch r.URL.Path {
		case "/api/v1/events":
			w.Header().Set("Content-Type", "text/event-stream")
			w.(http.Flusher).Flush()
			select {
			case <-change:
			case <-r.Context().Done():
				return
			}
			fmt.Fprint(w, "event: data_changed\ndata: {}\n\n")
			w.(http.Flusher).Flush()
			close(changeSent)
			<-r.Context().Done()
		case "/api/v1/sessions/codex:" + firstID:
			fmt.Fprintf(w, `{"id":"codex:%s","cwd":"/wrong","agent":"codex","created_at":"2026-07-13T00:00:00Z"}`, firstID)
		case "/api/v1/sessions/codex:" + secondID:
			fmt.Fprintf(w, `{"id":"codex:%s","cwd":"/wrong","agent":"codex","created_at":"2026-07-13T00:00:00Z"}`, secondID)
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
	defer cancel()
	errCh := make(chan error, 1)
	go func() {
		errCh <- runSubscribe(ctx, SubscribeConfig{
			ServerURL: source.URL, ZellijBin: zellij,
			ZellijConfigDir: "/isolated/config", ZellijConfigFile: "/isolated/config/config.kdl",
			ZellijDataDir: "/isolated/data", ZellijSession: "WORK", TabID: "73",
			RailURL: "file:/candidate/zellij-sidebar.wasm", CheckoutCWD: "/work/managed", RecipientToken: "test-token",
			RegistryDir: registryDir, StartupFD: int(writer.Fd()), SourceTimeout: time.Second, PipeTimeout: 2 * time.Second,
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
	registryDir := filepath.Join(dir, "registry")
	const registeredID = "019f5f94-a596-7d92-9928-398653669161"
	if err := (agentRegistryStore{root: registryDir}).upsert(registrationForTest(t, registeredID, "WORK", "7")); err != nil {
		t.Fatal(err)
	}
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
		"  *zaphod-agent-v1-*-ready*) echo ready ;;\n"+
		"  *zaphod-agent-v1-*-snapshot*) : > "+snapshotStarted+"; while [ ! -f "+releaseSnapshot+" ]; do sleep 0.01; done; echo accepted ;;\n"+
		"  *zaphod-agent-v1-*-event*) echo accepted ;;\n"+
		"esac\n")
	change := make(chan struct{})
	changeStarted := make(chan struct{})
	finishChange := make(chan struct{})
	changeSent := make(chan struct{})
	var lists atomic.Int32
	source := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		switch r.URL.Path {
		case "/api/v1/events":
			w.Header().Set("Content-Type", "text/event-stream")
			w.(http.Flusher).Flush()
			select {
			case <-change:
			case <-r.Context().Done():
				return
			}
			fmt.Fprint(w, "event: data_changed\n")
			w.(http.Flusher).Flush()
			close(changeStarted)
			select {
			case <-finishChange:
			case <-r.Context().Done():
				return
			}
			fmt.Fprint(w, "data: {}\n\n")
			w.(http.Flusher).Flush()
			close(changeSent)
			<-r.Context().Done()
		case "/api/v1/sessions/codex:" + registeredID:
			if lists.Add(1) == 1 {
				fmt.Fprintf(w, `{"id":"codex:%s","cwd":"/wrong","agent":"codex"}`, registeredID)
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
	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()
	errCh := make(chan error, 1)
	go func() {
		errCh <- runSubscribe(ctx, SubscribeConfig{
			ServerURL: source.URL, ZellijBin: zellij,
			ZellijConfigDir: "/isolated/config", ZellijConfigFile: "/isolated/config/config.kdl",
			ZellijDataDir: "/isolated/data", ZellijSession: "WORK", TabID: "73",
			RailURL: "file:/candidate/zellij-sidebar.wasm", CheckoutCWD: "/work/managed", RecipientToken: "test-token",
			RegistryDir: registryDir, StartupFD: int(writer.Fd()), SourceTimeout: time.Second, PipeTimeout: 2 * time.Second,
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
	if err := os.WriteFile(releaseSnapshot, nil, 0o600); err != nil {
		t.Fatal(err)
	}
	close(change)
	select {
	case <-changeStarted:
	case <-time.After(time.Second):
		t.Fatal("partial queued change was not emitted")
	}
	ready := make(chan string, 1)
	go func() {
		payload := make([]byte, 6)
		n, _ := reader.Read(payload)
		ready <- string(payload[:n])
	}()
	select {
	case payload := <-ready:
		t.Fatalf("partial queued change allowed readiness: %q", payload)
	case <-time.After(75 * time.Millisecond):
	}
	close(finishChange)
	select {
	case <-changeSent:
	case <-time.After(time.Second):
		t.Fatal("queued change was not completed")
	}
	err = <-errCh
	_ = writer.Close()
	payload := <-ready
	_ = reader.Close()
	if err == nil || !strings.Contains(err.Error(), "503") {
		t.Fatalf("queued catchup error = %v, want source failure", err)
	}
	if len(payload) != 0 {
		t.Fatalf("failed queued catchup signaled readiness: %q", payload)
	}
}
