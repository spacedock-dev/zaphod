// ABOUTME: Small source and recipient seams used by the tab-local watcher.
// ABOUTME: No persistent registry or automatic subscriber authority lives here.

package main

import (
	"bytes"
	"context"
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"net/http"
	"net/url"
	"os/exec"
	"strings"
	"time"
)

var (
	ErrTargetLost           = errors.New("target-lost")
	ErrSourceEOF            = errors.New("source-eof")
	ErrExactSessionNotFound = errors.New("exact-session-not-found")
)

type WatchRoute struct {
	ServerURL            string
	ZellijBin            string
	ZellijConfigDir      string
	ZellijConfigFile     string
	ZellijDataDir        string
	ZellijSession        string
	TabID                string
	RailURL              string
	RecipientToken       string
	SourceTimeout        time.Duration
	PipeTimeout          time.Duration
	SummaryClampBytes    int
	recipientWaitTimeout time.Duration
}

type zellijPane struct {
	ID           uint64  `json:"id"`
	TabID        uint64  `json:"tab_id"`
	IsPlugin     bool    `json:"is_plugin"`
	PluginURL    *string `json:"plugin_url"`
	IsFloating   bool    `json:"is_floating"`
	IsSuppressed bool    `json:"is_suppressed"`
	IsSelectable bool    `json:"is_selectable"`
}

func (cfg WatchRoute) emitConfig() Config {
	return Config{
		ZellijBin: cfg.ZellijBin, ZellijConfigDir: cfg.ZellijConfigDir,
		ZellijConfigFile: cfg.ZellijConfigFile, ZellijDataDir: cfg.ZellijDataDir,
		ZellijSession: cfg.ZellijSession, PipeName: "agent-event",
		PipeTimeout: cfg.PipeTimeout, SummaryClampBytes: cfg.SummaryClampBytes,
	}
}

func (cfg WatchRoute) zellijArgs(command ...string) []string {
	return append(zellijProfileArgs(cfg.emitConfig()), command...)
}

func serverEndpoint(serverURL, suffix string) (string, error) {
	base, err := url.Parse(serverURL)
	if err != nil || (base.Scheme != "http" && base.Scheme != "https") || base.Host == "" {
		return "", fmt.Errorf("invalid AgentsView server URL %q", serverURL)
	}
	base.Path = strings.TrimRight(base.Path, "/") + suffix
	base.RawQuery = ""
	return base.String(), nil
}

func fetchExactSession(ctx context.Context, client *http.Client, serverURL, sessionID string, timeout time.Duration) (sessionInfo, error) {
	requestCtx, cancel := context.WithTimeout(ctx, timeout)
	defer cancel()
	endpoint, err := serverEndpoint(serverURL, "/api/v1/sessions/"+url.PathEscape(sessionID))
	if err != nil {
		return sessionInfo{}, fmt.Errorf("source endpoint: %w", err)
	}
	req, err := http.NewRequestWithContext(requestCtx, http.MethodGet, endpoint, nil)
	if err != nil {
		return sessionInfo{}, err
	}
	response, err := client.Do(req)
	if err != nil {
		return sessionInfo{}, fmt.Errorf("source exact session %q: %w", sessionID, err)
	}
	defer response.Body.Close()
	if response.StatusCode == http.StatusNotFound {
		return sessionInfo{}, fmt.Errorf("%w: %q", ErrExactSessionNotFound, sessionID)
	}
	if response.StatusCode != http.StatusOK {
		return sessionInfo{}, fmt.Errorf("source exact session %q: unexpected HTTP status %s", sessionID, response.Status)
	}
	const maxSessionRecordBytes = 1 << 20
	payload, err := io.ReadAll(io.LimitReader(response.Body, maxSessionRecordBytes+1))
	if err != nil {
		return sessionInfo{}, fmt.Errorf("source exact session %q: read: %w", sessionID, err)
	}
	if len(payload) > maxSessionRecordBytes {
		return sessionInfo{}, fmt.Errorf("source exact session %q exceeds %d bytes", sessionID, maxSessionRecordBytes)
	}
	decoder := json.NewDecoder(bytes.NewReader(payload))
	var session sessionInfo
	if err := decoder.Decode(&session); err != nil {
		return sessionInfo{}, fmt.Errorf("source exact session %q: decode: %w", sessionID, err)
	}
	if err := requireJSONEOF(decoder); err != nil {
		return sessionInfo{}, fmt.Errorf("source exact session %q: %w", sessionID, err)
	}
	if session.ID != sessionID {
		return sessionInfo{}, fmt.Errorf("source exact session identity mismatch: requested %q, received %q", sessionID, session.ID)
	}
	return session, nil
}

func waitForRecipient(ctx context.Context, cfg WatchRoute) error {
	waitTimeout := cfg.recipientWaitTimeout
	if waitTimeout <= 0 {
		waitTimeout = 18 * time.Second
	}
	waitCtx, cancelWait := context.WithTimeout(ctx, waitTimeout)
	defer cancelWait()
	for {
		probeCtx, cancel := context.WithTimeout(waitCtx, cfg.PipeTimeout)
		args := cfg.zellijArgs("pipe", "--name", privateAgentPipeName(cfg.RecipientToken, "ready"),
			"--args", "recipient-tab-id="+cfg.TabID+",recipient-token="+cfg.RecipientToken)
		command := exec.CommandContext(probeCtx, cfg.ZellijBin, args...)
		command.WaitDelay = 50 * time.Millisecond
		command.Stdin = strings.NewReader("probe")
		output, err := command.Output()
		cancel()
		if err == nil && strings.TrimSpace(string(output)) == "ready" {
			return nil
		}
		select {
		case <-ctx.Done():
			return ctx.Err()
		case <-waitCtx.Done():
			if err := ctx.Err(); err != nil {
				return err
			}
			return fmt.Errorf("recipient-ready timeout for stable tab %s", cfg.TabID)
		case <-time.After(50 * time.Millisecond):
		}
	}
}
