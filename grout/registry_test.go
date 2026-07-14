// ABOUTME: Proves Codex SessionStart registration is exact, atomic, and single-owner.
// ABOUTME: Exercises process-safe replacement, conflict suppression, and private modes.

package main

import (
	"errors"
	"os"
	"path/filepath"
	"strings"
	"sync"
	"syscall"
	"testing"
	"time"
)

func validHook(sessionID string) []byte {
	return []byte(`{"session_id":"` + sessionID + `","transcript_path":null,"cwd":"/same/cwd","hook_event_name":"SessionStart","model":"gpt-5.6","permission_mode":"default","source":"startup"}`)
}

func TestDecodeCodexRegistrationRequiresCompleteStartupOrResumeRecord(t *testing.T) {
	const id = "019f5f94-a596-7d92-9928-398653669161"
	registration, err := decodeCodexRegistration(validHook(id), "managed", "7", 42, time.Unix(123, 0))
	if err != nil {
		t.Fatal(err)
	}
	if registration.AgentSessionID != id || registration.AgentsViewSessionID != "codex:"+id || registration.PaneID != 7 || registration.ZellijSession != "managed" {
		t.Fatalf("registration = %#v", registration)
	}

	for _, tc := range []struct {
		name    string
		payload string
		session string
		pane    string
	}{
		{"malformed", `{`, "managed", "7"},
		{"missing id", `{"hook_event_name":"SessionStart","source":"startup"}`, "managed", "7"},
		{"non canonical id", `{"session_id":"newest","hook_event_name":"SessionStart","source":"startup"}`, "managed", "7"},
		{"subagent", `{"session_id":"` + id + `","hook_event_name":"SubagentStart","source":"startup"}`, "managed", "7"},
		{"clear", `{"session_id":"` + id + `","hook_event_name":"SessionStart","source":"clear"}`, "managed", "7"},
		{"missing zellij session", string(validHook(id)), "", "7"},
		{"missing pane", string(validHook(id)), "managed", ""},
		{"non canonical pane", string(validHook(id)), "managed", "07"},
	} {
		t.Run(tc.name, func(t *testing.T) {
			if _, err := decodeCodexRegistration([]byte(tc.payload), tc.session, tc.pane, 42, time.Now()); err == nil {
				t.Fatal("invalid registration unexpectedly accepted")
			}
		})
	}
}

func TestRegistryUpsertIsIdempotentReplacesPaneAndSuppressesLiveConflict(t *testing.T) {
	root := t.TempDir()
	store := agentRegistryStore{root: root}
	const firstID = "019f5f94-a596-7d92-9928-398653669161"
	first, err := decodeCodexRegistration(validHook(firstID), "managed", "7", 42, time.Unix(123, 0))
	if err != nil {
		t.Fatal(err)
	}
	if err := store.upsert(first); err != nil {
		t.Fatal(err)
	}
	if err := store.upsert(first); err != nil {
		t.Fatal(err)
	}

	const nextID = "019f5f95-bbfd-7993-8620-0d698008217f"
	next, _ := decodeCodexRegistration(validHook(nextID), "managed", "7", 43, time.Unix(124, 0))
	if err := store.upsert(next); err != nil {
		t.Fatal(err)
	}
	conflict, _ := decodeCodexRegistration(validHook(nextID), "managed", "8", 44, time.Unix(125, 0))
	if err := store.upsert(conflict); err != nil {
		t.Fatal(err)
	}

	snapshot, err := store.read("managed")
	if err != nil {
		t.Fatal(err)
	}
	if len(snapshot.Registrations) != 2 {
		t.Fatalf("stored registrations = %#v, want two conflicting claims", snapshot.Registrations)
	}
	if got := deliverableRegistrations(snapshot.Registrations, map[uint32]uint64{7: 1, 8: 2}); len(got) != 0 {
		t.Fatalf("conflicting live claims became deliverable: %#v", got)
	}
	if got := deliverableRegistrations(snapshot.Registrations, map[uint32]uint64{7: 1}); len(got) != 1 || got[0].PaneID != 7 {
		t.Fatalf("stale claim cleanup projection = %#v, want pane 7", got)
	}
}

func TestRegistryConcurrentWritersRemainAtomicAndPrivate(t *testing.T) {
	root := filepath.Join(t.TempDir(), "private")
	store := agentRegistryStore{root: root}
	ids := []string{
		"019f5f94-a596-7d92-9928-398653669161",
		"019f5f95-bbfd-7993-8620-0d698008217f",
	}
	var wg sync.WaitGroup
	for index, id := range ids {
		wg.Add(1)
		go func(index int, id string) {
			defer wg.Done()
			registration, _ := decodeCodexRegistration(validHook(id), "managed", string(rune('7'+index)), 42+index, time.Now())
			if err := store.upsert(registration); err != nil {
				t.Errorf("upsert: %v", err)
			}
		}(index, id)
	}
	wg.Wait()

	snapshot, err := store.read("managed")
	if err != nil || len(snapshot.Registrations) != 2 {
		t.Fatalf("snapshot = %#v, err = %v", snapshot, err)
	}
	rootInfo, err := os.Stat(root)
	if err != nil {
		t.Fatal(err)
	}
	if rootInfo.Mode().Perm() != 0o700 {
		t.Fatalf("registry root mode = %o, want 700", rootInfo.Mode().Perm())
	}
	registryInfo, err := os.Stat(store.registryPath("managed"))
	if err != nil {
		t.Fatal(err)
	}
	if registryInfo.Mode().Perm() != 0o600 {
		t.Fatalf("registry mode = %o, want 600", registryInfo.Mode().Perm())
	}
}

func TestRegistryCrashBeforeRenamePreservesCompletePriorGeneration(t *testing.T) {
	store := agentRegistryStore{root: t.TempDir()}
	const firstID = "019f5f94-a596-7d92-9928-398653669161"
	first, _ := decodeCodexRegistration(validHook(firstID), "managed", "7", 42, time.Unix(123, 0))
	if err := store.upsert(first); err != nil {
		t.Fatal(err)
	}
	store.beforeRename = func() error { return errors.New("injected crash") }
	const nextID = "019f5f95-bbfd-7993-8620-0d698008217f"
	next, _ := decodeCodexRegistration(validHook(nextID), "managed", "8", 43, time.Unix(124, 0))
	if err := store.upsert(next); err == nil {
		t.Fatal("injected crash unexpectedly succeeded")
	}
	store.beforeRename = nil
	snapshot, err := store.read("managed")
	if err != nil {
		t.Fatal(err)
	}
	if len(snapshot.Registrations) != 1 || snapshot.Registrations[0].AgentSessionID != firstID {
		t.Fatalf("prior generation was not preserved: %#v", snapshot)
	}
}

func TestRegistryLockContentionHasAnIndependentDeadline(t *testing.T) {
	store := agentRegistryStore{root: t.TempDir(), lockTimeout: 50 * time.Millisecond}
	if err := store.prepareRoot(); err != nil {
		t.Fatal(err)
	}
	lock, err := os.OpenFile(store.lockPath("managed"), os.O_CREATE|os.O_RDWR, 0o600)
	if err != nil {
		t.Fatal(err)
	}
	defer lock.Close()
	if err := syscall.Flock(int(lock.Fd()), syscall.LOCK_EX); err != nil {
		t.Fatal(err)
	}
	defer syscall.Flock(int(lock.Fd()), syscall.LOCK_UN) //nolint:errcheck
	go func() {
		time.Sleep(200 * time.Millisecond)
		_ = syscall.Flock(int(lock.Fd()), syscall.LOCK_UN)
	}()
	registration := registrationForTest(t, "019f5f94-a596-7d92-9928-398653669161", "managed", "7")
	started := time.Now()
	err = store.upsert(registration)
	if err == nil || !strings.Contains(err.Error(), "lock timeout") {
		t.Fatalf("contended upsert error = %v, want lock timeout", err)
	}
	if elapsed := time.Since(started); elapsed > 150*time.Millisecond {
		t.Fatalf("contended upsert exceeded independent deadline: %s", elapsed)
	}
}

func TestRegistryPruneStaleRemovesOnlyAbsentPanes(t *testing.T) {
	store := agentRegistryStore{root: t.TempDir()}
	first := registrationForTest(t, "019f5f94-a596-7d92-9928-398653669161", "managed", "7")
	second := registrationForTest(t, "019f5f95-bbfd-7993-8620-0d698008217f", "managed", "8")
	if err := store.upsert(first); err != nil {
		t.Fatal(err)
	}
	if err := store.upsert(second); err != nil {
		t.Fatal(err)
	}
	if err := store.pruneStale("managed", map[uint32]uint64{7: 73}); err != nil {
		t.Fatal(err)
	}
	snapshot, err := store.read("managed")
	if err != nil {
		t.Fatal(err)
	}
	if len(snapshot.Registrations) != 1 || snapshot.Registrations[0].PaneID != 7 {
		t.Fatalf("pruned snapshot = %#v, want only live pane 7", snapshot)
	}
}
