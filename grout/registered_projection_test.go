// ABOUTME: Proves the sidecar projects only exact registered pane/session pairs.
// ABOUTME: CWD, list order, history, and child-like adjacent sessions never admit rows.

package main

import (
	"bytes"
	"context"
	"fmt"
	"net/http"
	"net/http/httptest"
	"os"
	"path/filepath"
	"reflect"
	"sync"
	"testing"
	"time"
)

func registrationForTest(t *testing.T, sessionID, zellijSession, paneID string) AgentPaneRegistrationV1 {
	t.Helper()
	registration, err := decodeCodexRegistration(validHook(sessionID), zellijSession, paneID, 42, time.Unix(123, 0))
	if err != nil {
		t.Fatal(err)
	}
	return registration
}

func TestRegisteredSessionsForTabFetchesOnlyExactLiveMembers(t *testing.T) {
	dir := t.TempDir()
	registryDir := filepath.Join(dir, "registry")
	store := agentRegistryStore{root: registryDir}
	const ownID = "019f5f94-a596-7d92-9928-398653669161"
	const foreignID = "019f5f95-bbfd-7993-8620-0d698008217f"
	if err := store.upsert(registrationForTest(t, ownID, "managed", "7")); err != nil {
		t.Fatal(err)
	}
	if err := store.upsert(registrationForTest(t, foreignID, "managed", "8")); err != nil {
		t.Fatal(err)
	}
	panes := `[
      {"id":50,"tab_id":73,"is_plugin":true,"plugin_url":"file:/candidate/zellij-sidebar.wasm","is_floating":false,"is_suppressed":false},
      {"id":7,"tab_id":73,"is_plugin":false,"is_selectable":true,"is_suppressed":false},
      {"id":8,"tab_id":74,"is_plugin":false,"is_selectable":true,"is_suppressed":false}
    ]`
	zellij := fakeSubscriberZellij(t, dir, filepath.Join(dir, "pipe.log"), panes)

	var lock sync.Mutex
	requests := []string{}
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		lock.Lock()
		requests = append(requests, r.URL.RequestURI())
		lock.Unlock()
		switch r.URL.Path {
		case "/api/v1/sessions/codex:" + ownID:
			fmt.Fprintf(w, `{"id":"codex:%s","cwd":"/deliberately/wrong","agent":"codex","first_message":"own"}`, ownID)
		case "/api/v1/sessions/codex:" + foreignID:
			fmt.Fprintf(w, `{"id":"codex:%s","cwd":"/same/cwd","agent":"codex","first_message":"foreign"}`, foreignID)
		case "/api/v1/sessions":
			t.Fatal("global session list must not be consulted")
		default:
			http.NotFound(w, r)
		}
	}))
	defer server.Close()

	got, err := registeredSessionsForTab(context.Background(), server.Client(), SubscribeConfig{
		ServerURL: server.URL, ZellijBin: zellij,
		ZellijConfigDir: "/c", ZellijConfigFile: "/c/config.kdl", ZellijDataDir: "/d",
		ZellijSession: "managed", TabID: "73", RailURL: "file:/candidate/zellij-sidebar.wasm",
		CheckoutCWD: "/same/cwd", RecipientToken: "token", RegistryDir: registryDir,
		SourceTimeout: time.Second, PipeTimeout: time.Second,
	}, 73)
	if err != nil {
		t.Fatal(err)
	}
	if len(got) != 1 || got[0].PaneID != 7 || got[0].Session.ID != "codex:"+ownID {
		t.Fatalf("registered projection = %#v", got)
	}
	lock.Lock()
	defer lock.Unlock()
	wantRequests := []string{"/api/v1/sessions/codex:" + ownID}
	if !reflect.DeepEqual(requests, wantRequests) {
		t.Fatalf("requests = %#v, want %#v", requests, wantRequests)
	}
}

func TestFetchExactSessionRejectsMismatchedReturnedIdentity(t *testing.T) {
	const requested = "codex:019f5f94-a596-7d92-9928-398653669161"
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		fmt.Fprint(w, `{"id":"codex:019f5f95-bbfd-7993-8620-0d698008217f","cwd":"/same/cwd","agent":"codex"}`)
	}))
	defer server.Close()
	if _, err := fetchExactSession(context.Background(), server.Client(), server.URL, requested, time.Second); err == nil {
		t.Fatal("mismatched exact response unexpectedly accepted")
	}
}

func TestDeliverRegisteredSnapshotRechecksPaneMembershipAfterFetch(t *testing.T) {
	dir := t.TempDir()
	panesPath := filepath.Join(dir, "panes.json")
	panes := `[
      {"id":50,"tab_id":73,"is_plugin":true,"plugin_url":"file:/candidate/zellij-sidebar.wasm","is_floating":false,"is_suppressed":false},
      {"id":7,"tab_id":74,"is_plugin":false,"is_selectable":true,"is_suppressed":false}
    ]`
	if err := os.WriteFile(panesPath, []byte(panes), 0o600); err != nil {
		t.Fatal(err)
	}
	snapshotPath := filepath.Join(dir, "snapshot.json")
	zellij := writeScript(t, dir, "zellij", "#!/bin/sh\n"+
		"case \"$*\" in\n"+
		"  *list-panes*) cat "+panesPath+" ;;\n"+
		"  *zaphod-agent-v1-*-snapshot*) cat > "+snapshotPath+"; echo accepted ;;\n"+
		"esac\n")
	const id = "codex:019f5f94-a596-7d92-9928-398653669161"
	err := deliverRegisteredSnapshot(context.Background(), SubscribeConfig{
		ZellijBin: zellij, ZellijConfigDir: "/c", ZellijConfigFile: "/c/config.kdl", ZellijDataDir: "/d",
		ZellijSession: "managed", TabID: "73", RailURL: "file:/candidate/zellij-sidebar.wasm",
		CheckoutCWD: "/same/cwd", RecipientToken: "token", RegistryDir: filepath.Join(dir, "registry"),
		PipeTimeout: time.Second,
	}, 73, []registeredSession{{PaneID: 7, Session: sessionInfo{ID: id, Agent: "codex", FirstMessage: "stale"}}}, &bytes.Buffer{})
	if err != nil {
		t.Fatal(err)
	}
	payload, err := os.ReadFile(snapshotPath)
	if err != nil {
		t.Fatal(err)
	}
	if string(payload) != "[]" {
		t.Fatalf("moved pane produced stale snapshot %s, want []", payload)
	}
}
