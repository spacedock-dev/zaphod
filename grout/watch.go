// ABOUTME: Private tab-local watcher identity and hook protocol.
// ABOUTME: Session authority lives in memory behind one pane-derived Unix socket.

package main

import (
	"bytes"
	"context"
	"crypto/sha256"
	"encoding/hex"
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"net"
	"os"
	"os/exec"
	"path/filepath"
	"regexp"
	"strconv"
	"strings"
)

const watchProtocol = "zaphod-watch-tab-v1"
const maxWatchHookBytes = 1 << 20
const maxWatchEnvelopeBytes = maxWatchHookBytes + 1024

var codexSessionIDPattern = regexp.MustCompile(`^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$`)

type codexSessionStartHook struct {
	SessionID      string          `json:"session_id"`
	TranscriptPath json.RawMessage `json:"transcript_path"`
	Cwd            string          `json:"cwd"`
	HookEventName  string          `json:"hook_event_name"`
	Model          string          `json:"model"`
	PermissionMode string          `json:"permission_mode"`
	Source         string          `json:"source"`
}

type watchEnvelopeV1 struct {
	Protocol      string                `json:"protocol"`
	ZellijSession string                `json:"zellij_session"`
	PaneID        uint32                `json:"pane_id"`
	Provider      string                `json:"provider"`
	Hook          codexSessionStartHook `json:"hook"`
}

type WatchRegistration struct {
	PaneID              uint32
	AgentSessionID      string
	AgentsViewSessionID string
}

type WatchTarget struct {
	TabID          uint64
	TerminalPaneID uint32
	RailPaneID     uint64
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

func requireJSONEOF(decoder *json.Decoder) error {
	var trailing any
	if err := decoder.Decode(&trailing); errors.Is(err, io.EOF) {
		return nil
	} else if err != nil {
		return fmt.Errorf("decode trailing JSON data: %w", err)
	}
	return fmt.Errorf("input contains more than one JSON value")
}

func watchSocketPath(root, zellijSession, paneValue string) (string, error) {
	if !filepath.IsAbs(root) {
		return "", fmt.Errorf("watch root must be absolute")
	}
	if err := validateZellijSession(zellijSession); err != nil {
		return "", err
	}
	if _, err := canonicalPaneID(paneValue); err != nil {
		return "", err
	}
	digest := sha256.Sum256([]byte(zellijSession + "\x00" + paneValue))
	path := filepath.Join(root, "watch-"+hex.EncodeToString(digest[:])+".sock")
	// Darwin's sockaddr_un.sun_path is 104 bytes including its terminating NUL.
	if len(path) > 103 {
		return "", fmt.Errorf("watch socket path exceeds 103 bytes")
	}
	return path, nil
}

func decodeWatchEnvelope(payload []byte, expectedSession, expectedPane string) (WatchRegistration, error) {
	if err := validateUniqueJSON(payload); err != nil {
		return WatchRegistration{}, err
	}
	decoder := json.NewDecoder(bytes.NewReader(payload))
	decoder.DisallowUnknownFields()
	var envelope watchEnvelopeV1
	if err := decoder.Decode(&envelope); err != nil {
		return WatchRegistration{}, fmt.Errorf("decode watch envelope: %w", err)
	}
	if err := requireJSONEOF(decoder); err != nil {
		return WatchRegistration{}, fmt.Errorf("decode watch envelope: %w", err)
	}
	if envelope.Protocol != watchProtocol || envelope.Provider != "codex" {
		return WatchRegistration{}, fmt.Errorf("unsupported watch envelope identity")
	}
	if envelope.ZellijSession != expectedSession {
		return WatchRegistration{}, fmt.Errorf("watch envelope Zellij session mismatch")
	}
	paneID, err := canonicalPaneID(expectedPane)
	if err != nil {
		return WatchRegistration{}, err
	}
	if envelope.PaneID != paneID {
		return WatchRegistration{}, fmt.Errorf("watch envelope pane mismatch")
	}
	if envelope.Hook.HookEventName != "SessionStart" {
		return WatchRegistration{}, fmt.Errorf("unsupported Codex hook event %q", envelope.Hook.HookEventName)
	}
	if envelope.Hook.Source != "startup" && envelope.Hook.Source != "resume" {
		return WatchRegistration{}, fmt.Errorf("unsupported SessionStart source %q", envelope.Hook.Source)
	}
	if !codexSessionIDPattern.MatchString(envelope.Hook.SessionID) {
		return WatchRegistration{}, fmt.Errorf("invalid Codex session id")
	}
	return WatchRegistration{
		PaneID:              paneID,
		AgentSessionID:      envelope.Hook.SessionID,
		AgentsViewSessionID: "codex:" + envelope.Hook.SessionID,
	}, nil
}

func listenWatchSocket(root, zellijSession, paneValue string) (*net.UnixListener, string, error) {
	if err := os.MkdirAll(root, 0o700); err != nil {
		return nil, "", fmt.Errorf("create watch root: %w", err)
	}
	if err := os.Chmod(root, 0o700); err != nil {
		return nil, "", fmt.Errorf("secure watch root: %w", err)
	}
	info, err := os.Lstat(root)
	if err != nil {
		return nil, "", fmt.Errorf("stat watch root: %w", err)
	}
	if !info.IsDir() || info.Mode()&os.ModeSymlink != 0 || info.Mode().Perm() != 0o700 {
		return nil, "", fmt.Errorf("watch root is not a private directory")
	}
	path, err := watchSocketPath(root, zellijSession, paneValue)
	if err != nil {
		return nil, "", err
	}
	if _, err := os.Lstat(path); err == nil {
		return nil, "", fmt.Errorf("watch socket already exists")
	} else if !errors.Is(err, os.ErrNotExist) {
		return nil, "", fmt.Errorf("stat watch socket: %w", err)
	}
	listener, err := net.ListenUnix("unix", &net.UnixAddr{Name: path, Net: "unix"})
	if err != nil {
		return nil, "", fmt.Errorf("listen watch socket: %w", err)
	}
	if err := os.Chmod(path, 0o600); err != nil {
		_ = listener.Close()
		_ = os.Remove(path)
		return nil, "", fmt.Errorf("secure watch socket: %w", err)
	}
	return listener, path, nil
}

func acceptWatchRegistration(ctx context.Context, listener *net.UnixListener, expectedSession, expectedPane string) (WatchRegistration, error) {
	return acceptWatchRegistrationWithAdmission(ctx, listener, expectedSession, expectedPane, nil)
}

func acceptWatchRegistrationWithAdmission(
	ctx context.Context,
	listener *net.UnixListener,
	expectedSession, expectedPane string,
	admit func(WatchRegistration) error,
) (WatchRegistration, error) {
	if deadline, ok := ctx.Deadline(); ok {
		if err := listener.SetDeadline(deadline); err != nil {
			return WatchRegistration{}, err
		}
	}
	connection, err := listener.AcceptUnix()
	if err != nil {
		if ctx.Err() != nil {
			return WatchRegistration{}, ctx.Err()
		}
		return WatchRegistration{}, fmt.Errorf("accept watch hook: %w", err)
	}
	defer connection.Close()
	if deadline, ok := ctx.Deadline(); ok {
		_ = connection.SetDeadline(deadline)
	}
	payload, err := io.ReadAll(io.LimitReader(connection, maxWatchEnvelopeBytes+1))
	if err != nil {
		return WatchRegistration{}, fmt.Errorf("read watch hook: %w", err)
	}
	if len(payload) > maxWatchEnvelopeBytes {
		return WatchRegistration{}, fmt.Errorf("watch envelope exceeds %d bytes", maxWatchEnvelopeBytes)
	}
	registration, err := decodeWatchEnvelope(payload, expectedSession, expectedPane)
	if err != nil {
		return WatchRegistration{}, err
	}
	if admit != nil {
		if err := admit(registration); err != nil {
			return WatchRegistration{}, err
		}
	}
	if _, err := io.WriteString(connection, "accepted\n"); err != nil {
		return WatchRegistration{}, fmt.Errorf("acknowledge watch hook: %w", err)
	}
	return registration, nil
}

func sendWatchHook(ctx context.Context, root, zellijSession, paneValue string, hookPayload []byte) error {
	if len(hookPayload) > maxWatchHookBytes {
		return fmt.Errorf("Codex hook exceeds %d bytes", maxWatchHookBytes)
	}
	hook, err := decodeCodexHook(hookPayload)
	if err != nil {
		return err
	}
	paneID, err := canonicalPaneID(paneValue)
	if err != nil {
		return err
	}
	path, err := watchSocketPath(root, zellijSession, paneValue)
	if err != nil {
		return err
	}
	payload, err := json.Marshal(watchEnvelopeV1{
		Protocol: watchProtocol, ZellijSession: zellijSession, PaneID: paneID,
		Provider: "codex", Hook: hook,
	})
	if err != nil {
		return fmt.Errorf("encode watch hook: %w", err)
	}
	dialer := net.Dialer{}
	connection, err := dialer.DialContext(ctx, "unix", path)
	if err != nil {
		return fmt.Errorf("connect watch socket: %w", err)
	}
	defer connection.Close()
	if deadline, ok := ctx.Deadline(); ok {
		_ = connection.SetDeadline(deadline)
	}
	if _, err := connection.Write(payload); err != nil {
		return fmt.Errorf("send watch hook: %w", err)
	}
	if unixConnection, ok := connection.(*net.UnixConn); ok {
		if err := unixConnection.CloseWrite(); err != nil {
			return fmt.Errorf("finish watch hook: %w", err)
		}
	}
	ack, err := io.ReadAll(io.LimitReader(connection, 64))
	if err != nil {
		return fmt.Errorf("read watch hook acknowledgment: %w", err)
	}
	if string(ack) != "accepted\n" {
		return fmt.Errorf("watch hook was not acknowledged")
	}
	return nil
}

func decodeCodexHook(payload []byte) (codexSessionStartHook, error) {
	if err := validateUniqueJSON(payload); err != nil {
		return codexSessionStartHook{}, err
	}
	decoder := json.NewDecoder(bytes.NewReader(payload))
	decoder.DisallowUnknownFields()
	var hook codexSessionStartHook
	if err := decoder.Decode(&hook); err != nil {
		return codexSessionStartHook{}, fmt.Errorf("decode Codex hook: %w", err)
	}
	if err := requireJSONEOF(decoder); err != nil {
		return codexSessionStartHook{}, err
	}
	if hook.HookEventName != "SessionStart" {
		return codexSessionStartHook{}, fmt.Errorf("unsupported Codex hook event %q", hook.HookEventName)
	}
	if hook.Source != "startup" && hook.Source != "resume" {
		return codexSessionStartHook{}, fmt.Errorf("unsupported SessionStart source %q", hook.Source)
	}
	if !codexSessionIDPattern.MatchString(hook.SessionID) {
		return codexSessionStartHook{}, fmt.Errorf("invalid Codex session id")
	}
	return hook, nil
}

// validateUniqueJSON walks the complete JSON value before typed decoding so
// duplicate keys cannot be laundered by encoding/json's last-value behavior.
func validateUniqueJSON(payload []byte) error {
	decoder := json.NewDecoder(bytes.NewReader(payload))
	if err := validateUniqueJSONValue(decoder); err != nil {
		return fmt.Errorf("validate watch envelope: %w", err)
	}
	if err := requireJSONEOF(decoder); err != nil {
		return fmt.Errorf("validate watch envelope: %w", err)
	}
	return nil
}

func validateUniqueJSONValue(decoder *json.Decoder) error {
	token, err := decoder.Token()
	if err != nil {
		return err
	}
	delim, composite := token.(json.Delim)
	if !composite {
		return nil
	}
	switch delim {
	case '{':
		seen := make(map[string]struct{})
		for decoder.More() {
			keyToken, err := decoder.Token()
			if err != nil {
				return err
			}
			key, ok := keyToken.(string)
			if !ok {
				return fmt.Errorf("object key is not a string")
			}
			if _, duplicate := seen[key]; duplicate {
				return fmt.Errorf("duplicate JSON field %q", key)
			}
			seen[key] = struct{}{}
			if err := validateUniqueJSONValue(decoder); err != nil {
				return err
			}
		}
		end, err := decoder.Token()
		if err != nil {
			return err
		}
		if end != json.Delim('}') {
			return fmt.Errorf("unterminated object")
		}
	case '[':
		for decoder.More() {
			if err := validateUniqueJSONValue(decoder); err != nil {
				return err
			}
		}
		end, err := decoder.Token()
		if err != nil {
			return err
		}
		if end != json.Delim(']') {
			return fmt.Errorf("unterminated array")
		}
	default:
		return fmt.Errorf("unexpected JSON delimiter %q", delim)
	}
	return nil
}

func resolveWatchTarget(ctx context.Context, cfg WatchRoute, watchedPane uint32) (WatchTarget, error) {
	panes, err := watchNativePanes(ctx, cfg)
	if err != nil {
		return WatchTarget{}, err
	}
	terminalCount := 0
	var tabID uint64
	for _, pane := range panes {
		if !pane.IsPlugin && pane.IsSelectable && !pane.IsSuppressed && pane.ID == uint64(watchedPane) {
			terminalCount++
			tabID = pane.TabID
		}
	}
	if terminalCount != 1 {
		return WatchTarget{}, fmt.Errorf("%w: watched terminal pane %d count is %d", ErrTargetLost, watchedPane, terminalCount)
	}
	railCount := 0
	var railID uint64
	for _, pane := range panes {
		if pane.TabID == tabID && pane.IsPlugin && pane.PluginURL != nil && *pane.PluginURL == cfg.RailURL &&
			!pane.IsFloating && !pane.IsSuppressed {
			railCount++
			railID = pane.ID
		}
	}
	if railCount != 1 {
		return WatchTarget{}, fmt.Errorf("%w: original rail count in stable tab %d is %d", ErrTargetLost, tabID, railCount)
	}
	return WatchTarget{TabID: tabID, TerminalPaneID: watchedPane, RailPaneID: railID}, nil
}

func probeWatchTarget(ctx context.Context, cfg WatchRoute, target WatchTarget) error {
	panes, err := watchNativePanes(ctx, cfg)
	if err != nil {
		return err
	}
	terminals := 0
	rails := 0
	for _, pane := range panes {
		if !pane.IsPlugin && pane.IsSelectable && !pane.IsSuppressed &&
			pane.ID == uint64(target.TerminalPaneID) && pane.TabID == target.TabID {
			terminals++
		}
		if pane.IsPlugin && pane.ID == target.RailPaneID && pane.TabID == target.TabID &&
			pane.PluginURL != nil && *pane.PluginURL == cfg.RailURL && !pane.IsFloating && !pane.IsSuppressed {
			rails++
		}
	}
	if terminals != 1 || rails != 1 {
		return fmt.Errorf("%w: expected terminal %d and original rail %d once in stable tab %d, found %d/%d",
			ErrTargetLost, target.TerminalPaneID, target.RailPaneID, target.TabID, terminals, rails)
	}
	return nil
}

func watchNativePanes(ctx context.Context, cfg WatchRoute) ([]zellijPane, error) {
	args := cfg.zellijArgs("action", "list-panes", "--json", "--all", "--state", "--tab")
	command := exec.CommandContext(ctx, cfg.ZellijBin, args...)
	var stderr strings.Builder
	command.Stderr = &stderr
	output, err := command.Output()
	if err != nil {
		if ctx.Err() != nil {
			return nil, ctx.Err()
		}
		return nil, fmt.Errorf("%w: native list-panes: %v: %s", ErrTargetLost, err, strings.TrimSpace(stderr.String()))
	}
	const maxPaneReplyBytes = 1 << 20
	if len(output) > maxPaneReplyBytes {
		return nil, fmt.Errorf("%w: native pane state exceeds %d bytes", ErrTargetLost, maxPaneReplyBytes)
	}
	decoder := json.NewDecoder(bytes.NewReader(output))
	var panes []zellijPane
	if err := decoder.Decode(&panes); err != nil {
		return nil, fmt.Errorf("%w: malformed native pane state: %v", ErrTargetLost, err)
	}
	if err := requireJSONEOF(decoder); err != nil {
		return nil, fmt.Errorf("%w: malformed native pane state: %v", ErrTargetLost, err)
	}
	return panes, nil
}
