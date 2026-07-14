// ABOUTME: Specifies tab-local watcher socket identity and atomic hook admission.
// ABOUTME: The private transport carries one exact SessionStart and no durable record.

package main

import (
	"context"
	"encoding/json"
	"os"
	"path/filepath"
	"strings"
	"testing"
	"time"
)

func validWatchEnvelope() []byte {
	return []byte(`{"protocol":"zaphod-watch-tab-v1","zellij_session":"managed","pane_id":7,"provider":"codex","hook":{"session_id":"019f60ff-1111-7222-8333-444455556666","hook_event_name":"SessionStart","source":"startup"}}`)
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
