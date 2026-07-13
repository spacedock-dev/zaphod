// ABOUTME: Private per-managed-tab AgentsView subscriber for the Zaphod rail.
// ABOUTME: Re-lists on SSE data_changed and emits only stable-tab-addressed session rows.

package main

import (
	"bufio"
	"context"
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"net/http"
	"net/url"
	"os/exec"
	"strconv"
	"strings"
	"time"
)

var (
	// ErrTargetLost means the originally verified server tab no longer has
	// exactly one tiled rail with the canonical candidate URL. It is terminal:
	// this sidecar never follows a replacement.
	ErrTargetLost = errors.New("target-lost")
	// ErrSourceEOF means the one AgentsView stream ended. Reconnect policy is
	// deliberately out of this walking skeleton.
	ErrSourceEOF = errors.New("source-eof")
)

// SubscribeConfig is entirely derived by the direct managed-tab entry after
// it has created and verified a fresh tab. It has no discovery fallback:
// TabID, profile, and canonical rail URL are all mandatory.
type SubscribeConfig struct {
	ServerURL         string
	ZellijBin         string
	ZellijConfigDir   string
	ZellijConfigFile  string
	ZellijDataDir     string
	ZellijSession     string
	TabID             string
	RailURL           string
	StartupFD         int
	PipeTimeout       time.Duration
	SummaryClampBytes int
}

type zellijPane struct {
	ID           uint64  `json:"id"`
	TabID        uint64  `json:"tab_id"`
	IsPlugin     bool    `json:"is_plugin"`
	PluginURL    *string `json:"plugin_url"`
	IsFloating   bool    `json:"is_floating"`
	IsSuppressed bool    `json:"is_suppressed"`
	IsSelectable bool    `json:"is_selectable"`
	PaneCwd      *string `json:"pane_cwd"`
}

type targetSnapshot struct {
	cwds map[string]struct{}
}

func canonicalTabID(value string) (uint64, error) {
	if value == "" {
		return 0, fmt.Errorf("missing tab id")
	}
	for _, c := range value {
		if c < '0' || c > '9' {
			return 0, fmt.Errorf("tab id %q is not unsigned decimal", value)
		}
	}
	if len(value) > 1 && value[0] == '0' {
		return 0, fmt.Errorf("tab id %q is not canonical unsigned decimal", value)
	}
	id, err := strconv.ParseUint(value, 10, 64)
	if err != nil {
		return 0, fmt.Errorf("tab id %q: %w", value, err)
	}
	return id, nil
}

func (cfg SubscribeConfig) validate() (uint64, error) {
	if cfg.ServerURL == "" || cfg.ZellijBin == "" || cfg.ZellijConfigDir == "" ||
		cfg.ZellijConfigFile == "" || cfg.ZellijDataDir == "" || cfg.ZellijSession == "" ||
		cfg.RailURL == "" {
		return 0, fmt.Errorf("subscribe requires server, Zellij profile, session, tab id, and rail URL")
	}
	server, err := url.Parse(cfg.ServerURL)
	if err != nil || (server.Scheme != "http" && server.Scheme != "https") || server.Host == "" {
		return 0, fmt.Errorf("invalid AgentsView server URL %q", cfg.ServerURL)
	}
	return canonicalTabID(cfg.TabID)
}

func (cfg SubscribeConfig) emitConfig() Config {
	return Config{
		ZellijBin:         cfg.ZellijBin,
		ZellijConfigDir:   cfg.ZellijConfigDir,
		ZellijConfigFile:  cfg.ZellijConfigFile,
		ZellijDataDir:     cfg.ZellijDataDir,
		ZellijSession:     cfg.ZellijSession,
		PipeName:          "agent-event",
		PipeTimeout:       cfg.PipeTimeout,
		SummaryClampBytes: cfg.SummaryClampBytes,
	}
}

func (cfg SubscribeConfig) zellijArgs(command ...string) []string {
	args := zellijProfileArgs(cfg.emitConfig())
	return append(args, command...)
}

// probeTarget checks the only target identity that the sidecar may use: its
// original stable server tab ID plus the exact canonical rail URL. A pane ID,
// title, CWD, display position, or URL-only match cannot substitute for it.
func probeTarget(ctx context.Context, cfg SubscribeConfig, stableTabID uint64) (targetSnapshot, error) {
	if err := ctx.Err(); err != nil {
		return targetSnapshot{}, err
	}
	args := cfg.zellijArgs("action", "list-panes", "--json", "--all", "--command", "--geometry", "--state", "--tab")
	command := exec.CommandContext(ctx, cfg.ZellijBin, args...)
	output, err := command.Output()
	if err != nil {
		if ctx.Err() != nil {
			return targetSnapshot{}, ctx.Err()
		}
		return targetSnapshot{}, fmt.Errorf("%w: native list-panes: %v", ErrTargetLost, err)
	}
	var panes []zellijPane
	if err := json.Unmarshal(output, &panes); err != nil {
		return targetSnapshot{}, fmt.Errorf("%w: malformed native pane state: %v", ErrTargetLost, err)
	}
	resident := 0
	cwds := make(map[string]struct{})
	for _, pane := range panes {
		if pane.TabID != stableTabID {
			continue
		}
		if pane.IsPlugin && pane.PluginURL != nil && *pane.PluginURL == cfg.RailURL &&
			!pane.IsFloating && !pane.IsSuppressed {
			resident++
		}
		if !pane.IsPlugin && pane.IsSelectable && !pane.IsSuppressed && pane.PaneCwd != nil && *pane.PaneCwd != "" {
			cwds[*pane.PaneCwd] = struct{}{}
		}
	}
	if resident != 1 {
		return targetSnapshot{}, fmt.Errorf("%w: expected one resident rail in stable tab %d, found %d", ErrTargetLost, stableTabID, resident)
	}
	return targetSnapshot{cwds: cwds}, nil
}

func serverEndpoint(serverURL, suffix string) (string, error) {
	base, err := url.Parse(serverURL)
	if err != nil {
		return "", err
	}
	base.Path = strings.TrimRight(base.Path, "/") + suffix
	base.RawQuery = ""
	return base.String(), nil
}

func fetchSessions(ctx context.Context, client *http.Client, serverURL string) ([]sessionInfo, error) {
	endpoint, err := serverEndpoint(serverURL, "/api/v1/sessions")
	if err != nil {
		return nil, fmt.Errorf("source endpoint: %w", err)
	}
	req, err := http.NewRequestWithContext(ctx, http.MethodGet, endpoint, nil)
	if err != nil {
		return nil, err
	}
	query := req.URL.Query()
	query.Set("include_one_shot", "true")
	query.Set("include_children", "true")
	query.Set("limit", "1000")
	req.URL.RawQuery = query.Encode()
	response, err := client.Do(req)
	if err != nil {
		return nil, fmt.Errorf("source list: %w", err)
	}
	defer response.Body.Close()
	if response.StatusCode != http.StatusOK {
		return nil, fmt.Errorf("source list: unexpected HTTP status %s", response.Status)
	}
	var page struct {
		Sessions []sessionInfo `json:"sessions"`
	}
	if err := json.NewDecoder(response.Body).Decode(&page); err != nil {
		return nil, fmt.Errorf("source list: decode: %w", err)
	}
	return page.Sessions, nil
}

func refreshSessions(
	ctx context.Context,
	client *http.Client,
	cfg SubscribeConfig,
	stableTabID uint64,
	stderr io.Writer,
) error {
	// Probing before the source call makes a vanished session/tab terminal even
	// when the next list happens to be empty. A later probe before each pipe
	// closes the race between list and delivery.
	target, err := probeTarget(ctx, cfg, stableTabID)
	if err != nil {
		return err
	}
	sessions, err := fetchSessions(ctx, client, cfg.ServerURL)
	if err != nil {
		return err
	}
	for _, session := range sessions {
		if _, local := target.cwds[session.Cwd]; !local {
			continue
		}
		current, err := probeTarget(ctx, cfg, stableTabID)
		if err != nil {
			return err
		}
		if _, stillLocal := current.cwds[session.Cwd]; !stillLocal {
			continue
		}
		row := BuildSessionRow(session, time.Now(), cfg.SummaryClampBytes)
		if err := EmitRowForTab(cfg.emitConfig(), row.Kind, row, cfg.TabID, stderr); err != nil {
			return err
		}
	}
	return nil
}

func streamEvents(
	ctx context.Context,
	client *http.Client,
	cfg SubscribeConfig,
	stableTabID uint64,
	stderr io.Writer,
) error {
	endpoint, err := serverEndpoint(cfg.ServerURL, "/api/v1/events")
	if err != nil {
		return err
	}
	req, err := http.NewRequestWithContext(ctx, http.MethodGet, endpoint, nil)
	if err != nil {
		return err
	}
	req.Header.Set("Accept", "text/event-stream")
	response, err := client.Do(req)
	if err != nil {
		return fmt.Errorf("source stream: %w", err)
	}
	defer response.Body.Close()
	if response.StatusCode != http.StatusOK {
		return fmt.Errorf("source stream: unexpected HTTP status %s", response.Status)
	}

	scanner := bufio.NewScanner(response.Body)
	// An event is only a trigger today, but allow enough room for a server
	// diagnostic without silently tokenizing it.
	scanner.Buffer(make([]byte, 1024), 1024*1024)
	eventName := ""
	for scanner.Scan() {
		line := scanner.Text()
		if line == "" {
			if eventName == "data_changed" {
				if err := refreshSessions(ctx, client, cfg, stableTabID, stderr); err != nil {
					return err
				}
			} else if eventName == "heartbeat" {
				if _, err := probeTarget(ctx, cfg, stableTabID); err != nil {
					return err
				}
			}
			eventName = ""
			continue
		}
		if strings.HasPrefix(line, "event:") {
			eventName = strings.TrimSpace(strings.TrimPrefix(line, "event:"))
		}
	}
	if err := scanner.Err(); err != nil {
		if ctx.Err() != nil {
			return nil
		}
		return fmt.Errorf("source stream: %w", err)
	}
	if ctx.Err() != nil {
		return nil
	}
	return ErrSourceEOF
}

// runSubscribe has exactly one source connection. Initial state is listed
// before connecting, then data_changed causes a fresh list. EOF, source
// failure, or target loss stops the sidecar; there is no retry, pool, lease,
// retarget, or external cleanup in this slice.
func runSubscribe(ctx context.Context, cfg SubscribeConfig, stderr io.Writer) error {
	stableTabID, err := cfg.validate()
	if err != nil {
		return err
	}
	if cfg.PipeTimeout <= 0 {
		cfg.PipeTimeout = 5 * time.Second
	}
	if cfg.SummaryClampBytes <= 0 {
		cfg.SummaryClampBytes = 512
	}
	client := &http.Client{}
	if err := refreshSessions(ctx, client, cfg, stableTabID, stderr); err != nil {
		if ctx.Err() != nil {
			return nil
		}
		return err
	}
	err = streamEvents(ctx, client, cfg, stableTabID, stderr)
	if ctx.Err() != nil {
		return nil
	}
	return err
}
