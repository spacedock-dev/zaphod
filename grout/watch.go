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
	"path/filepath"
)

const watchProtocol = "zaphod-watch-tab-v1"
const maxWatchHookBytes = 1 << 20
const maxWatchEnvelopeBytes = maxWatchHookBytes + 1024

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
