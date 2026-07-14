// ABOUTME: Private tab-local watcher identity and hook protocol.
// ABOUTME: Session authority lives in memory behind one pane-derived Unix socket.

package main

import (
	"bytes"
	"crypto/sha256"
	"encoding/hex"
	"encoding/json"
	"fmt"
	"path/filepath"
)

const watchProtocol = "zaphod-watch-tab-v1"

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
