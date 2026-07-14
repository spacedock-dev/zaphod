// ABOUTME: Native owner of the exact Codex-session-to-Zellij-pane registry.
// ABOUTME: Uses advisory locking and same-directory atomic replacement.

package main

import (
	"bytes"
	"crypto/sha256"
	"encoding/hex"
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"os"
	"path/filepath"
	"regexp"
	"sort"
	"strconv"
	"strings"
	"syscall"
	"time"
)

const agentRegistryVersion = 1

var codexSessionIDPattern = regexp.MustCompile(`^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$`)

type codexSessionStartHook struct {
	SessionID     string `json:"session_id"`
	HookEventName string `json:"hook_event_name"`
	Source        string `json:"source"`
}

type AgentPaneRegistrationV1 struct {
	ZellijSession       string `json:"zellij_session"`
	PaneID              uint32 `json:"pane_id"`
	Agent               string `json:"agent"`
	AgentSessionID      string `json:"agent_session_id"`
	AgentsViewSessionID string `json:"agentsview_session_id"`
	PID                 int    `json:"pid"`
	UpdatedAt           string `json:"updated_at"`
}

type AgentRegistryV1 struct {
	Version       int                       `json:"version"`
	ZellijSession string                    `json:"zellij_session"`
	Registrations []AgentPaneRegistrationV1 `json:"registrations"`
}

type agentRegistryStore struct {
	root         string
	beforeRename func() error
}

func validateZellijSession(value string) error {
	if value == "" || len(value) > 255 || strings.TrimSpace(value) != value {
		return fmt.Errorf("invalid Zellij session name")
	}
	for _, r := range value {
		if r == 0 || r < 0x20 || r == 0x7f {
			return fmt.Errorf("invalid Zellij session name")
		}
	}
	return nil
}

func canonicalPaneID(value string) (uint32, error) {
	if value == "" {
		return 0, fmt.Errorf("missing Zellij pane id")
	}
	if len(value) > 1 && value[0] == '0' {
		return 0, fmt.Errorf("Zellij pane id %q is not canonical", value)
	}
	for _, c := range value {
		if c < '0' || c > '9' {
			return 0, fmt.Errorf("Zellij pane id %q is not unsigned decimal", value)
		}
	}
	id, err := strconv.ParseUint(value, 10, 32)
	if err != nil {
		return 0, fmt.Errorf("Zellij pane id %q: %w", value, err)
	}
	return uint32(id), nil
}

func decodeCodexRegistration(payload []byte, zellijSession, paneValue string, pid int, now time.Time) (AgentPaneRegistrationV1, error) {
	if err := validateZellijSession(zellijSession); err != nil {
		return AgentPaneRegistrationV1{}, err
	}
	paneID, err := canonicalPaneID(paneValue)
	if err != nil {
		return AgentPaneRegistrationV1{}, err
	}
	decoder := json.NewDecoder(bytes.NewReader(payload))
	var hook codexSessionStartHook
	if err := decoder.Decode(&hook); err != nil {
		return AgentPaneRegistrationV1{}, fmt.Errorf("decode Codex hook: %w", err)
	}
	if err := requireJSONEOF(decoder); err != nil {
		return AgentPaneRegistrationV1{}, err
	}
	if hook.HookEventName != "SessionStart" {
		return AgentPaneRegistrationV1{}, fmt.Errorf("unsupported Codex hook event %q", hook.HookEventName)
	}
	if hook.Source != "startup" && hook.Source != "resume" {
		return AgentPaneRegistrationV1{}, fmt.Errorf("unsupported SessionStart source %q", hook.Source)
	}
	if !codexSessionIDPattern.MatchString(hook.SessionID) {
		return AgentPaneRegistrationV1{}, fmt.Errorf("invalid Codex session id")
	}
	return AgentPaneRegistrationV1{
		ZellijSession:       zellijSession,
		PaneID:              paneID,
		Agent:               "codex",
		AgentSessionID:      hook.SessionID,
		AgentsViewSessionID: "codex:" + hook.SessionID,
		PID:                 pid,
		UpdatedAt:           now.UTC().Format(time.RFC3339Nano),
	}, nil
}

func requireJSONEOF(decoder *json.Decoder) error {
	var trailing any
	if err := decoder.Decode(&trailing); errors.Is(err, io.EOF) {
		return nil
	} else if err != nil {
		return fmt.Errorf("decode trailing hook data: %w", err)
	}
	return fmt.Errorf("hook input contains more than one JSON value")
}

func (s agentRegistryStore) registryPath(zellijSession string) string {
	digest := sha256.Sum256([]byte(zellijSession))
	return filepath.Join(s.root, "session-"+hex.EncodeToString(digest[:])+".json")
}

func defaultAgentRegistryDir() string {
	if value := os.Getenv("ZAPHOD_REGISTRY_DIR"); value != "" {
		return value
	}
	if value := os.Getenv("XDG_RUNTIME_DIR"); value != "" {
		return filepath.Join(value, "zaphod", "agent-sessions-v1")
	}
	return filepath.Join(os.TempDir(), fmt.Sprintf("zaphod-agent-sessions-v1-%d", os.Getuid()))
}

func (s agentRegistryStore) lockPath(zellijSession string) string {
	return s.registryPath(zellijSession) + ".lock"
}

func (s agentRegistryStore) prepareRoot() error {
	if s.root == "" {
		return fmt.Errorf("registry root is required")
	}
	if err := os.MkdirAll(s.root, 0o700); err != nil {
		return fmt.Errorf("create registry root: %w", err)
	}
	if err := os.Chmod(s.root, 0o700); err != nil {
		return fmt.Errorf("secure registry root: %w", err)
	}
	info, err := os.Lstat(s.root)
	if err != nil {
		return fmt.Errorf("stat registry root: %w", err)
	}
	if !info.IsDir() || info.Mode()&os.ModeSymlink != 0 {
		return fmt.Errorf("registry root is not a private directory")
	}
	return nil
}

func (s agentRegistryStore) withLock(zellijSession string, exclusive bool, fn func() error) error {
	if err := validateZellijSession(zellijSession); err != nil {
		return err
	}
	if err := s.prepareRoot(); err != nil {
		return err
	}
	lock, err := os.OpenFile(s.lockPath(zellijSession), os.O_CREATE|os.O_RDWR, 0o600)
	if err != nil {
		return fmt.Errorf("open registry lock: %w", err)
	}
	defer lock.Close()
	if err := lock.Chmod(0o600); err != nil {
		return fmt.Errorf("secure registry lock: %w", err)
	}
	operation := syscall.LOCK_SH
	if exclusive {
		operation = syscall.LOCK_EX
	}
	if err := syscall.Flock(int(lock.Fd()), operation); err != nil {
		return fmt.Errorf("lock registry: %w", err)
	}
	defer syscall.Flock(int(lock.Fd()), syscall.LOCK_UN) //nolint:errcheck
	return fn()
}

func (s agentRegistryStore) readUnlocked(zellijSession string) (AgentRegistryV1, error) {
	registry := AgentRegistryV1{Version: agentRegistryVersion, ZellijSession: zellijSession, Registrations: []AgentPaneRegistrationV1{}}
	payload, err := os.ReadFile(s.registryPath(zellijSession))
	if errors.Is(err, os.ErrNotExist) {
		return registry, nil
	}
	if err != nil {
		return AgentRegistryV1{}, fmt.Errorf("read registry: %w", err)
	}
	decoder := json.NewDecoder(bytes.NewReader(payload))
	if err := decoder.Decode(&registry); err != nil {
		return AgentRegistryV1{}, fmt.Errorf("decode registry: %w", err)
	}
	if err := requireJSONEOF(decoder); err != nil {
		return AgentRegistryV1{}, fmt.Errorf("decode registry: %w", err)
	}
	if registry.Version != agentRegistryVersion || registry.ZellijSession != zellijSession {
		return AgentRegistryV1{}, fmt.Errorf("registry identity mismatch")
	}
	for _, registration := range registry.Registrations {
		if registration.ZellijSession != zellijSession || registration.Agent != "codex" ||
			!codexSessionIDPattern.MatchString(registration.AgentSessionID) ||
			registration.AgentsViewSessionID != "codex:"+registration.AgentSessionID {
			return AgentRegistryV1{}, fmt.Errorf("registry contains an invalid registration")
		}
	}
	return registry, nil
}

func (s agentRegistryStore) read(zellijSession string) (AgentRegistryV1, error) {
	var registry AgentRegistryV1
	err := s.withLock(zellijSession, false, func() error {
		var err error
		registry, err = s.readUnlocked(zellijSession)
		return err
	})
	return registry, err
}

func (s agentRegistryStore) upsert(registration AgentPaneRegistrationV1) error {
	return s.withLock(registration.ZellijSession, true, func() error {
		registry, err := s.readUnlocked(registration.ZellijSession)
		if err != nil {
			return err
		}
		replaced := false
		for index := range registry.Registrations {
			if registry.Registrations[index].PaneID == registration.PaneID {
				registry.Registrations[index] = registration
				replaced = true
				break
			}
		}
		if !replaced {
			registry.Registrations = append(registry.Registrations, registration)
		}
		sort.Slice(registry.Registrations, func(i, j int) bool {
			return registry.Registrations[i].PaneID < registry.Registrations[j].PaneID
		})
		return s.writeAtomic(registry)
	})
}

func (s agentRegistryStore) writeAtomic(registry AgentRegistryV1) error {
	payload, err := json.MarshalIndent(registry, "", "  ")
	if err != nil {
		return fmt.Errorf("encode registry: %w", err)
	}
	payload = append(payload, '\n')
	temporary, err := os.CreateTemp(s.root, ".registry-*.tmp")
	if err != nil {
		return fmt.Errorf("create registry temporary: %w", err)
	}
	temporaryPath := temporary.Name()
	committed := false
	defer func() {
		temporary.Close()
		if !committed {
			_ = os.Remove(temporaryPath)
		}
	}()
	if err := temporary.Chmod(0o600); err != nil {
		return fmt.Errorf("secure registry temporary: %w", err)
	}
	if _, err := temporary.Write(payload); err != nil {
		return fmt.Errorf("write registry temporary: %w", err)
	}
	if err := temporary.Sync(); err != nil {
		return fmt.Errorf("sync registry temporary: %w", err)
	}
	if err := temporary.Close(); err != nil {
		return fmt.Errorf("close registry temporary: %w", err)
	}
	if s.beforeRename != nil {
		if err := s.beforeRename(); err != nil {
			return err
		}
	}
	if err := os.Rename(temporaryPath, s.registryPath(registry.ZellijSession)); err != nil {
		return fmt.Errorf("replace registry: %w", err)
	}
	committed = true
	directory, err := os.Open(s.root)
	if err != nil {
		return fmt.Errorf("open registry root for sync: %w", err)
	}
	defer directory.Close()
	if err := directory.Sync(); err != nil {
		return fmt.Errorf("sync registry root: %w", err)
	}
	return nil
}

// deliverableRegistrations filters to exact live panes and suppresses every
// live claimant of a duplicated AgentsView identity. No heuristic chooses a
// winner. The map value is the pane's current stable tab id.
func deliverableRegistrations(registrations []AgentPaneRegistrationV1, livePaneTabs map[uint32]uint64) []AgentPaneRegistrationV1 {
	live := make([]AgentPaneRegistrationV1, 0, len(registrations))
	counts := make(map[string]int)
	for _, registration := range registrations {
		if _, ok := livePaneTabs[registration.PaneID]; !ok {
			continue
		}
		live = append(live, registration)
		counts[registration.AgentsViewSessionID]++
	}
	result := live[:0]
	for _, registration := range live {
		if counts[registration.AgentsViewSessionID] == 1 {
			result = append(result, registration)
		}
	}
	return result
}
