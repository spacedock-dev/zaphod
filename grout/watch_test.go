// ABOUTME: Specifies tab-local watcher socket identity and atomic hook admission.
// ABOUTME: The private transport carries one exact SessionStart and no durable record.

package main

import (
	"context"
	"encoding/json"
	"errors"
	"fmt"
	"net"
	"os"
	"path/filepath"
	"strings"
	"syscall"
	"testing"
	"time"
)

func validWatchEnvelope() []byte {
	return []byte(`{"protocol":"zaphod-watch-tab-v1","zellij_session":"managed","pane_id":7,"provider":"codex","hook":{"session_id":"019f60ff-1111-7222-8333-444455556666","hook_event_name":"SessionStart","source":"startup"}}`)
}

func shortWatchRoot(t *testing.T) string {
	t.Helper()
	root, err := os.MkdirTemp("/tmp", "zwt.")
	if err != nil {
		t.Fatal(err)
	}
	t.Cleanup(func() { _ = os.RemoveAll(root) })
	return root
}

func TestWatchSocketPathIsExactPrivatePaneIdentity(t *testing.T) {
	root, err := os.MkdirTemp("/tmp", "zwt.")
	if err != nil {
		t.Fatal(err)
	}
	t.Cleanup(func() { _ = os.RemoveAll(root) })
	one, err := watchSocketPath(root, "managed", "7")
	if err != nil {
		t.Fatal(err)
	}
	again, err := watchSocketPath(root, "managed", "7")
	if err != nil {
		t.Fatal(err)
	}
	otherPane, err := watchSocketPath(root, "managed", "8")
	if err != nil {
		t.Fatal(err)
	}
	otherSession, err := watchSocketPath(root, "other", "7")
	if err != nil {
		t.Fatal(err)
	}
	if one != again || one == otherPane || one == otherSession {
		t.Fatalf("socket identity one=%q again=%q pane=%q session=%q", one, again, otherPane, otherSession)
	}
	if filepath.Dir(one) != root || !strings.HasSuffix(one, ".sock") || len(one) > 103 {
		t.Fatalf("socket path is not a short child of its private root: %q", one)
	}
}

func TestWatchSocketPathFitsShippedDefaultRoot(t *testing.T) {
	root := filepath.Join("/tmp", fmt.Sprintf("zaphod-watch-tab-v1-%d", os.Getuid()))
	path, err := watchSocketPath(root, "kj-live-1784045584", "2")
	if err != nil {
		t.Fatalf("shipped default root %q: %v", root, err)
	}
	if len(path) > 103 {
		t.Fatalf("socket path length = %d, want at most 103: %q", len(path), path)
	}
}

func TestListenWatchSocketReclaimsProvenStaleSocket(t *testing.T) {
	root := shortWatchRoot(t)
	listener, socket, err := listenWatchSocket(root, "managed", "7")
	if err != nil {
		t.Fatal(err)
	}
	listener.SetUnlinkOnClose(false)
	if err := listener.Close(); err != nil {
		t.Fatal(err)
	}
	if info, err := os.Lstat(socket); err != nil || info.Mode()&os.ModeSocket == 0 {
		t.Fatalf("stale socket missing before retry: info=%v err=%v", info, err)
	}

	restarted, restartedPath, err := listenWatchSocket(root, "managed", "7")
	if err != nil {
		t.Fatalf("retry refused proven stale socket: %v", err)
	}
	t.Cleanup(func() {
		_ = restarted.Close()
		_ = os.Remove(restartedPath)
	})
	if restartedPath != socket {
		t.Fatalf("restart socket = %q, want %q", restartedPath, socket)
	}
}

func TestListenWatchSocketDoesNotEvictLiveWatcher(t *testing.T) {
	root := shortWatchRoot(t)
	listener, socket, err := listenWatchSocket(root, "managed", "7")
	if err != nil {
		t.Fatal(err)
	}
	t.Cleanup(func() {
		_ = listener.Close()
		_ = os.Remove(socket)
	})
	ctx, cancel := context.WithTimeout(context.Background(), time.Second)
	defer cancel()
	registrations := make(chan WatchRegistration, 1)
	acceptErrors := make(chan error, 1)
	go acceptWatchHooks(ctx, listener, "managed", "7", registrations, acceptErrors)

	if replacement, _, err := listenWatchSocket(root, "managed", "7"); err == nil {
		_ = replacement.Close()
		t.Fatal("second watcher displaced a live listener")
	}
	hook := []byte(`{"session_id":"019f60ff-1111-7222-8333-444455556666","hook_event_name":"SessionStart","source":"startup"}`)
	if err := sendWatchHook(ctx, root, "managed", "7", hook); err != nil {
		t.Fatalf("live watcher was no longer reachable: %v", err)
	}
	select {
	case registration := <-registrations:
		if registration.PaneID != 7 {
			t.Fatalf("registration after rejected replacement = %#v", registration)
		}
	case err := <-acceptErrors:
		t.Fatalf("replacement probe disrupted live watcher: %v", err)
	case <-ctx.Done():
		t.Fatal(ctx.Err())
	}
}

func TestListenWatchSocketFailsClosedForAmbiguousEndpoint(t *testing.T) {
	t.Run("regular file", func(t *testing.T) {
		root := shortWatchRoot(t)
		path, err := watchSocketPath(root, "managed", "7")
		if err != nil {
			t.Fatal(err)
		}
		if err := os.WriteFile(path, []byte("not a socket"), 0o600); err != nil {
			t.Fatal(err)
		}
		if listener, _, err := listenWatchSocket(root, "managed", "7"); err == nil {
			_ = listener.Close()
			t.Fatal("ambiguous regular file was replaced")
		}
		payload, err := os.ReadFile(path)
		if err != nil || string(payload) != "not a socket" {
			t.Fatalf("ambiguous regular file changed: payload=%q err=%v", payload, err)
		}
	})

	t.Run("non-private socket", func(t *testing.T) {
		root := shortWatchRoot(t)
		listener, path, err := listenWatchSocket(root, "managed", "7")
		if err != nil {
			t.Fatal(err)
		}
		listener.SetUnlinkOnClose(false)
		if err := listener.Close(); err != nil {
			t.Fatal(err)
		}
		if err := os.Chmod(path, 0o660); err != nil {
			t.Fatal(err)
		}
		if replacement, _, err := listenWatchSocket(root, "managed", "7"); err == nil {
			_ = replacement.Close()
			t.Fatal("ambiguous non-private socket was replaced")
		}
		info, err := os.Lstat(path)
		if err != nil || info.Mode()&os.ModeSocket == 0 || info.Mode().Perm() != 0o660 {
			t.Fatalf("ambiguous socket changed: info=%v err=%v", info, err)
		}
	})

	t.Run("foreign owner", func(t *testing.T) {
		root := shortWatchRoot(t)
		listener, path, err := listenWatchSocket(root, "managed", "7")
		if err != nil {
			t.Fatal(err)
		}
		defer listener.Close()
		info, err := os.Lstat(path)
		if err != nil {
			t.Fatal(err)
		}
		if err := validateExistingWatchSocket(info, os.Getuid()+1); err == nil || !strings.Contains(err.Error(), "ownership") {
			t.Fatalf("foreign-owned endpoint was not ambiguous: %v", err)
		}
	})
}

func TestListenWatchSocketWaitsForConcurrentDifferentPaneAdmission(t *testing.T) {
	root := shortWatchRoot(t)
	rootHandle, err := os.Open(root)
	if err != nil {
		t.Fatal(err)
	}
	defer rootHandle.Close()
	if err := syscall.Flock(int(rootHandle.Fd()), syscall.LOCK_EX|syscall.LOCK_NB); err != nil {
		t.Fatal(err)
	}
	type result struct {
		listener *net.UnixListener
		path     string
		err      error
	}
	completed := make(chan result, 1)
	go func() {
		listener, path, err := listenWatchSocket(root, "managed", "8")
		completed <- result{listener: listener, path: path, err: err}
	}()
	select {
	case got := <-completed:
		t.Fatalf("different-pane admission did not wait for the shared critical section: %v", got.err)
	case <-time.After(20 * time.Millisecond):
	}
	if err := syscall.Flock(int(rootHandle.Fd()), syscall.LOCK_UN); err != nil {
		t.Fatal(err)
	}
	select {
	case got := <-completed:
		if got.err != nil {
			t.Fatalf("different-pane admission failed after lock release: %v", got.err)
		}
		_ = got.listener.Close()
		_ = os.Remove(got.path)
	case <-time.After(time.Second):
		t.Fatal("different-pane admission did not complete after lock release")
	}
}

func TestWatchNativePanesNamesAuthorityDeadline(t *testing.T) {
	zellij := writeScript(t, t.TempDir(), "zellij", "#!/bin/sh\nsleep 1\n")
	ctx, cancel := context.WithTimeout(context.Background(), 20*time.Millisecond)
	defer cancel()
	_, err := watchNativePanes(ctx, WatchRoute{ZellijBin: zellij})
	if !errors.Is(err, ErrTargetLost) || !errors.Is(err, context.DeadlineExceeded) {
		t.Fatalf("native deadline error = %v, want target-lost plus context deadline", err)
	}
	if !strings.Contains(err.Error(), "native pane-state probe deadline") {
		t.Fatalf("native deadline error did not name the failing boundary: %v", err)
	}
}

func TestWatchEnvelopeIsValidatedAtomically(t *testing.T) {
	wantID := "codex:019f60ff-1111-7222-8333-444455556666"
	registration, err := decodeWatchEnvelope(validWatchEnvelope(), "managed", "7")
	if err != nil {
		t.Fatal(err)
	}
	if registration.PaneID != 7 || registration.AgentsViewSessionID != wantID {
		t.Fatalf("registration = %#v", registration)
	}

	var base map[string]any
	if err := json.Unmarshal(validWatchEnvelope(), &base); err != nil {
		t.Fatal(err)
	}
	cases := map[string][]byte{
		"wrong session":   []byte(`{"protocol":"zaphod-watch-tab-v1","zellij_session":"other","pane_id":7,"provider":"codex","hook":{"session_id":"019f60ff-1111-7222-8333-444455556666","hook_event_name":"SessionStart","source":"startup"}}`),
		"wrong pane":      []byte(`{"protocol":"zaphod-watch-tab-v1","zellij_session":"managed","pane_id":8,"provider":"codex","hook":{"session_id":"019f60ff-1111-7222-8333-444455556666","hook_event_name":"SessionStart","source":"startup"}}`),
		"child":           []byte(`{"protocol":"zaphod-watch-tab-v1","zellij_session":"managed","pane_id":7,"provider":"codex","hook":{"session_id":"019f60ff-1111-7222-8333-444455556666","hook_event_name":"SubagentStart","source":"startup"}}`),
		"unknown field":   []byte(`{"protocol":"zaphod-watch-tab-v1","zellij_session":"managed","pane_id":7,"provider":"codex","extra":true,"hook":{"session_id":"019f60ff-1111-7222-8333-444455556666","hook_event_name":"SessionStart","source":"startup"}}`),
		"duplicate field": []byte(`{"protocol":"zaphod-watch-tab-v1","protocol":"zaphod-watch-tab-v1","zellij_session":"managed","pane_id":7,"provider":"codex","hook":{"session_id":"019f60ff-1111-7222-8333-444455556666","hook_event_name":"SessionStart","source":"startup"}}`),
		"trailing value":  append(validWatchEnvelope(), []byte(` {}`)...),
	}
	for name, payload := range cases {
		t.Run(name, func(t *testing.T) {
			if got, err := decodeWatchEnvelope(payload, "managed", "7"); err == nil {
				t.Fatalf("accepted %#v", got)
			}
		})
	}
}

func TestWatchSocketCarriesOneBoundedInMemoryRegistration(t *testing.T) {
	root, err := os.MkdirTemp("/tmp", "zwt.")
	if err != nil {
		t.Fatal(err)
	}
	t.Cleanup(func() { _ = os.RemoveAll(root) })
	listener, socket, err := listenWatchSocket(root, "managed", "7")
	if err != nil {
		t.Fatal(err)
	}
	t.Cleanup(func() { _ = listener.Close() })
	rootInfo, err := os.Stat(root)
	if err != nil {
		t.Fatal(err)
	}
	socketInfo, err := os.Lstat(socket)
	if err != nil {
		t.Fatal(err)
	}
	if rootInfo.Mode().Perm() != 0o700 || socketInfo.Mode().Perm() != 0o600 || socketInfo.Mode()&os.ModeSocket == 0 {
		t.Fatalf("private modes root=%#o socket=%v", rootInfo.Mode().Perm(), socketInfo.Mode())
	}

	ctx, cancel := context.WithTimeout(context.Background(), time.Second)
	defer cancel()
	accepted := make(chan WatchRegistration, 1)
	errs := make(chan error, 1)
	go func() {
		registration, err := acceptWatchRegistration(ctx, listener, "managed", "7")
		if err != nil {
			errs <- err
			return
		}
		accepted <- registration
	}()
	hook := []byte(`{"session_id":"019f60ff-1111-7222-8333-444455556666","hook_event_name":"SessionStart","source":"startup"}`)
	if err := sendWatchHook(ctx, root, "managed", "7", hook); err != nil {
		t.Fatal(err)
	}
	select {
	case registration := <-accepted:
		if registration.AgentsViewSessionID != "codex:019f60ff-1111-7222-8333-444455556666" {
			t.Fatalf("registration = %#v", registration)
		}
	case err := <-errs:
		t.Fatal(err)
	case <-ctx.Done():
		t.Fatal(ctx.Err())
	}
	entries, err := os.ReadDir(root)
	if err != nil {
		t.Fatal(err)
	}
	if len(entries) != 1 || entries[0].Name() != filepath.Base(socket) {
		t.Fatalf("watcher wrote durable authority files: %v", entries)
	}

	missingRoot, err := os.MkdirTemp("/tmp", "zwt.")
	if err != nil {
		t.Fatal(err)
	}
	t.Cleanup(func() { _ = os.RemoveAll(missingRoot) })
	missingCtx, missingCancel := context.WithTimeout(context.Background(), 200*time.Millisecond)
	defer missingCancel()
	if err := sendWatchHook(missingCtx, missingRoot, "managed", "7", hook); err == nil {
		t.Fatal("missing watcher accepted a hook")
	}
	tooLarge := append(hook, make([]byte, maxWatchHookBytes-len(hook)+1)...)
	if err := sendWatchHook(ctx, root, "managed", "7", tooLarge); err == nil {
		t.Fatal("over-limit hook reached the watcher")
	}
}

func TestWatcherContinuouslyProvesOriginalTerminalTabAndRail(t *testing.T) {
	dir := t.TempDir()
	panesPath := filepath.Join(dir, "panes.json")
	argsPath := filepath.Join(dir, "args")
	writePanes := func(value string) {
		t.Helper()
		if err := os.WriteFile(panesPath, []byte(value), 0o600); err != nil {
			t.Fatal(err)
		}
	}
	writePanes(`[
{"id":50,"tab_id":73,"is_plugin":true,"plugin_url":"file:/candidate/sidebar.wasm","is_floating":false,"is_suppressed":false},
{"id":7,"tab_id":73,"is_plugin":false,"is_selectable":true,"is_suppressed":false},
{"id":60,"tab_id":74,"is_plugin":true,"plugin_url":"file:/candidate/sidebar.wasm","is_floating":false,"is_suppressed":false},
{"id":8,"tab_id":74,"is_plugin":false,"is_selectable":true,"is_suppressed":false}
]`)
	zellij := writeScript(t, dir, "zellij", "#!/bin/sh\nprintf '%s\\n' \"$*\" >> "+argsPath+"\ncat "+panesPath+"\n")
	cfg := WatchRoute{
		ZellijBin: zellij, ZellijConfigDir: "/c", ZellijConfigFile: "/c/config.kdl",
		ZellijDataDir: "/d", ZellijSession: "managed", RailURL: "file:/candidate/sidebar.wasm",
	}
	target, err := resolveWatchTarget(context.Background(), cfg, 7)
	if err != nil {
		t.Fatal(err)
	}
	if target.TabID != 73 || target.TerminalPaneID != 7 || target.RailPaneID != 50 {
		t.Fatalf("target = %#v", target)
	}
	if err := probeWatchTarget(context.Background(), cfg, target); err != nil {
		t.Fatal(err)
	}

	// Retained per-client runtimes are not extra panes, but a second matching
	// native rail in the stable tab must still revoke authority.
	writePanes(`[
{"id":50,"tab_id":73,"is_plugin":true,"plugin_url":"file:/candidate/sidebar.wasm","is_floating":false,"is_suppressed":false},
{"id":51,"tab_id":73,"is_plugin":true,"plugin_url":"file:/candidate/sidebar.wasm","is_floating":false,"is_suppressed":false},
{"id":7,"tab_id":73,"is_plugin":false,"is_selectable":true,"is_suppressed":false}
]`)
	if err := probeWatchTarget(context.Background(), cfg, target); err == nil {
		t.Fatal("duplicate matching native rail preserved authority")
	}

	// A same-WASM rail in another tab is not the original authority.
	writePanes(`[
{"id":50,"tab_id":73,"is_plugin":true,"plugin_url":"file:/candidate/sidebar.wasm","is_floating":false,"is_suppressed":false},
{"id":7,"tab_id":73,"is_plugin":false,"is_selectable":true,"is_suppressed":false}
]`)
	if err := probeWatchTarget(context.Background(), cfg, target); err != nil {
		t.Fatalf("bystander removal revoked target: %v", err)
	}

	// Removing the exact original rail fails closed even though an otherwise
	// identical rail remains visible in another tab.
	writePanes(`[
{"id":60,"tab_id":74,"is_plugin":true,"plugin_url":"file:/candidate/sidebar.wasm","is_floating":false,"is_suppressed":false},
{"id":7,"tab_id":73,"is_plugin":false,"is_selectable":true,"is_suppressed":false}
]`)
	if err := probeWatchTarget(context.Background(), cfg, target); err == nil {
		t.Fatal("replacement same-WASM rail preserved authority")
	}
	args, err := os.ReadFile(argsPath)
	if err != nil {
		t.Fatal(err)
	}
	for _, line := range strings.Split(strings.TrimSpace(string(args)), "\n") {
		for _, required := range []string{"--all", "--state", "--tab"} {
			if !strings.Contains(line, required) {
				t.Fatalf("native authority probe omitted %s: %s", required, line)
			}
		}
		for _, forbidden := range []string{"--command", "--geometry", "--cwd"} {
			if strings.Contains(line, forbidden) {
				t.Fatalf("native authority probe requested %s: %s", forbidden, line)
			}
		}
	}
}
