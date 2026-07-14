// ABOUTME: Private per-managed-tab AgentsView subscriber for the Zaphod rail.
// ABOUTME: Re-lists on SSE data_changed and emits only stable-tab-addressed session rows.

package main

import (
	"bufio"
	"bytes"
	"context"
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"mime"
	"net/http"
	"net/url"
	"os/exec"
	"path/filepath"
	"strconv"
	"strings"
	"sync"
	"sync/atomic"
	"time"
	"unicode/utf8"
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
	CheckoutCWD       string
	RecipientToken    string
	RegistryDir       string
	StartupFD         int
	SourceTimeout     time.Duration
	PipeTimeout       time.Duration
	SummaryClampBytes int
	// Package-private deterministic concurrency seams used only by tests.
	afterScan              func()
	beforeReadinessCheck   func()
	afterRead              func(int, error)
	beforeNextScan         func()
	beforeLineSend         func()
	beforeSplit            func()
	afterTokenPendingClear func()
	recipientWaitTimeout   time.Duration
	nativeDiagnostics      io.Writer
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

type targetSnapshot struct {
	paneTabs map[uint32]uint64
}

type registeredSession struct {
	PaneID  uint32
	Session sessionInfo
}

type readinessReader struct {
	reader    io.Reader
	boundary  *sync.Mutex
	activity  *atomic.Uint64
	ended     *atomic.Bool
	afterRead func(int, error)
	pending   *atomic.Bool
}

func (r readinessReader) Read(p []byte) (int, error) {
	n, err := r.reader.Read(p)
	r.boundary.Lock()
	if n > 0 {
		r.activity.Add(1)
		r.pending.Store(true)
	}
	if err != nil {
		r.ended.Store(true)
	}
	r.boundary.Unlock()
	if r.afterRead != nil {
		r.afterRead(n, err)
	}
	return n, err
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
		cfg.RailURL == "" || cfg.CheckoutCWD == "" || cfg.RecipientToken == "" {
		return 0, fmt.Errorf("subscribe requires server, Zellij profile, session, tab id, and rail URL")
	}
	if !filepath.IsAbs(cfg.CheckoutCWD) {
		return 0, fmt.Errorf("subscribe checkout cwd must be absolute")
	}
	if cfg.RegistryDir != "" && !filepath.IsAbs(cfg.RegistryDir) {
		return 0, fmt.Errorf("subscribe registry directory must be absolute")
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

func nativePaneReplyProvenance(attempt int, stdout []byte, stderr string) string {
	const prefixLimit = 256
	stdoutPrefix := stdout
	if len(stdoutPrefix) > prefixLimit {
		stdoutPrefix = stdoutPrefix[:prefixLimit]
	}
	stderrPrefix := []byte(stderr)
	if len(stderrPrefix) > prefixLimit {
		stderrPrefix = stderrPrefix[:prefixLimit]
	}
	return fmt.Sprintf(
		"command=list-panes attempt=%d/3 stdout_len=%d stdout_prefix=%s stderr_len=%d stderr_prefix=%s",
		attempt, len(stdout), strconv.QuoteToASCII(string(stdoutPrefix)), len(stderr),
		strconv.QuoteToASCII(string(stderrPrefix)),
	)
}

// isNativeLayoutReply recognizes the bounded, complete wrong-action record
// observed when Zellij returned dump-layout output to list-panes. The caller
// may retry this one shape, but never accepts it as pane state.
func isNativeLayoutReply(output []byte) bool {
	const maxLayoutReply = 1 << 20
	trimmed := bytes.TrimSpace(output)
	return len(trimmed) <= maxLayoutReply && utf8.Valid(trimmed) &&
		bytes.HasPrefix(trimmed, []byte("layout {")) && bytes.HasSuffix(trimmed, []byte("}")) &&
		!bytes.ContainsRune(trimmed, '\x00')
}

// probeTarget checks the only target identity that the sidecar may use: its
// original stable server tab ID plus the exact canonical rail URL. Native
// list-panes may omit terminal cwd, so the direct entry's absolute checkout
// root supplies the row filter after the tab still proves a terminal exists.
func probeTarget(ctx context.Context, cfg SubscribeConfig, stableTabID uint64) (targetSnapshot, error) {
	if err := ctx.Err(); err != nil {
		return targetSnapshot{}, err
	}
	args := cfg.zellijArgs("action", "list-panes", "--json", "--all", "--command", "--geometry", "--state", "--tab")
	var output []byte
	var panes []zellijPane
	var stderrOutput string
	lastAttempt := 0
	for attempt := 0; attempt < 3; attempt++ {
		command := exec.CommandContext(ctx, cfg.ZellijBin, args...)
		var commandStderr strings.Builder
		command.Stderr = &commandStderr
		var err error
		output, err = command.Output()
		stderrOutput = commandStderr.String()
		lastAttempt = attempt + 1
		if err != nil {
			if ctx.Err() != nil {
				return targetSnapshot{}, ctx.Err()
			}
			return targetSnapshot{}, fmt.Errorf(
				"%w: native list-panes: %v; %s", ErrTargetLost, err,
				nativePaneReplyProvenance(lastAttempt, output, stderrOutput),
			)
		}
		if len(bytes.TrimSpace(output)) > 0 {
			if err := json.Unmarshal(output, &panes); err != nil {
				if isNativeLayoutReply(output) && attempt < 2 {
					if cfg.nativeDiagnostics != nil {
						fmt.Fprintf(cfg.nativeDiagnostics, "transient-native-pane-reply: %s\n",
							nativePaneReplyProvenance(lastAttempt, output, stderrOutput))
					}
					select {
					case <-ctx.Done():
						return targetSnapshot{}, ctx.Err()
					case <-time.After(50 * time.Millisecond):
					}
					continue
				}
				return targetSnapshot{}, fmt.Errorf(
					"%w: malformed native pane state: %v; %s", ErrTargetLost, err,
					nativePaneReplyProvenance(lastAttempt, output, stderrOutput),
				)
			}
			if len(panes) > 0 {
				break
			}
		}
		if attempt < 2 {
			select {
			case <-ctx.Done():
				return targetSnapshot{}, ctx.Err()
			case <-time.After(50 * time.Millisecond):
			}
		}
	}
	if panes == nil {
		if err := json.Unmarshal(output, &panes); err != nil {
			return targetSnapshot{}, fmt.Errorf(
				"%w: malformed native pane state: %v; %s", ErrTargetLost, err,
				nativePaneReplyProvenance(lastAttempt, output, stderrOutput),
			)
		}
	}
	resident := 0
	terminals := 0
	paneTabs := make(map[uint32]uint64)
	for _, pane := range panes {
		if pane.TabID == stableTabID && pane.IsPlugin && pane.PluginURL != nil && *pane.PluginURL == cfg.RailURL &&
			!pane.IsFloating && !pane.IsSuppressed {
			resident++
		}
		if !pane.IsPlugin && pane.IsSelectable && !pane.IsSuppressed {
			if pane.ID > uint64(^uint32(0)) {
				return targetSnapshot{}, fmt.Errorf("%w: terminal pane id %d exceeds protocol range", ErrTargetLost, pane.ID)
			}
			paneID := uint32(pane.ID)
			if _, duplicate := paneTabs[paneID]; duplicate {
				return targetSnapshot{}, fmt.Errorf("%w: duplicate terminal pane id %d", ErrTargetLost, paneID)
			}
			paneTabs[paneID] = pane.TabID
			if pane.TabID == stableTabID {
				terminals++
			}
		}
	}
	if resident != 1 {
		return targetSnapshot{}, fmt.Errorf("%w: expected one resident rail in stable tab %d, found %d", ErrTargetLost, stableTabID, resident)
	}
	if terminals == 0 {
		return targetSnapshot{}, fmt.Errorf("%w: stable tab %d has no selectable terminal", ErrTargetLost, stableTabID)
	}
	return targetSnapshot{paneTabs: paneTabs}, nil
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

func registeredSessionsForTab(
	ctx context.Context,
	client *http.Client,
	cfg SubscribeConfig,
	stableTabID uint64,
) ([]registeredSession, error) {
	target, err := probeTarget(ctx, cfg, stableTabID)
	if err != nil {
		return nil, err
	}
	registryDir := cfg.RegistryDir
	if registryDir == "" {
		registryDir = defaultAgentRegistryDir()
	}
	store := agentRegistryStore{root: registryDir}
	if err := store.pruneStale(cfg.ZellijSession, target.paneTabs); err != nil {
		return nil, fmt.Errorf("prune agent registry: %w", err)
	}
	registry, err := store.read(cfg.ZellijSession)
	if err != nil {
		return nil, fmt.Errorf("read agent registry: %w", err)
	}
	deliverable := deliverableRegistrations(registry.Registrations, target.paneTabs)
	result := make([]registeredSession, 0, len(deliverable))
	for _, registration := range deliverable {
		if target.paneTabs[registration.PaneID] != stableTabID {
			continue
		}
		session, err := fetchExactSession(ctx, client, cfg.ServerURL, registration.AgentsViewSessionID, cfg.SourceTimeout)
		if err != nil {
			return nil, err
		}
		result = append(result, registeredSession{PaneID: registration.PaneID, Session: session})
	}
	return result, nil
}

func snapshotSessions(
	ctx context.Context,
	client *http.Client,
	cfg SubscribeConfig,
	stableTabID uint64,
) ([]registeredSession, error) {
	return registeredSessionsForTab(ctx, client, cfg, stableTabID)
}

func deliverRegisteredSnapshot(
	ctx context.Context,
	cfg SubscribeConfig,
	stableTabID uint64,
	sessions []registeredSession,
	stderr io.Writer,
) error {
	current, err := probeTarget(ctx, cfg, stableTabID)
	if err != nil {
		return err
	}
	registryDir := cfg.RegistryDir
	if registryDir == "" {
		registryDir = defaultAgentRegistryDir()
	}
	registry, err := (agentRegistryStore{root: registryDir}).read(cfg.ZellijSession)
	if err != nil {
		return fmt.Errorf("read agent registry before delivery: %w", err)
	}
	allowed := make(map[string]struct{})
	for _, registration := range deliverableRegistrations(registry.Registrations, current.paneTabs) {
		if current.paneTabs[registration.PaneID] == stableTabID {
			key := fmt.Sprintf("%d\x00%s", registration.PaneID, registration.AgentsViewSessionID)
			allowed[key] = struct{}{}
		}
	}
	rows := make([]SessionRow, 0, len(sessions))
	for _, session := range sessions {
		key := fmt.Sprintf("%d\x00%s", session.PaneID, session.Session.ID)
		if _, stillRegistered := allowed[key]; !stillRegistered {
			continue
		}
		rows = append(rows, BuildRegisteredSessionRow(session.Session, session.PaneID, time.Now(), cfg.SummaryClampBytes))
	}
	return EmitSnapshotForTab(ctx, cfg.emitConfig(), rows, cfg.TabID, cfg.RecipientToken, stderr)
}

func deliverSnapshot(
	ctx context.Context,
	cfg SubscribeConfig,
	stableTabID uint64,
	sessions []registeredSession,
	stderr io.Writer,
) error {
	return deliverRegisteredSnapshot(ctx, cfg, stableTabID, sessions, stderr)
}

func refreshSessions(
	ctx context.Context,
	client *http.Client,
	cfg SubscribeConfig,
	stableTabID uint64,
	stderr io.Writer,
) error {
	sessions, err := snapshotSessions(ctx, client, cfg, stableTabID)
	if err != nil {
		return err
	}
	return deliverRegisteredSnapshot(ctx, cfg, stableTabID, sessions, stderr)
}

func waitForRecipient(ctx context.Context, cfg SubscribeConfig) error {
	// A newly added ReadCliPipes grant may put the ordinary permission prompt
	// in front of the recipient. Leave the attached user time to approve it
	// once while staying inside the entry script's overall startup bound.
	waitTimeout := cfg.recipientWaitTimeout
	if waitTimeout <= 0 {
		waitTimeout = 18 * time.Second
	}
	waitCtx, cancelWait := context.WithTimeout(ctx, waitTimeout)
	defer cancelWait()
	for {
		probeCtx, cancel := context.WithTimeout(waitCtx, cfg.PipeTimeout)
		args := cfg.zellijArgs("pipe", "--name", privateAgentPipeName(cfg.RecipientToken, "ready"), "--args", "recipient-tab-id="+cfg.TabID+",recipient-token="+cfg.RecipientToken)
		command := exec.CommandContext(probeCtx, cfg.ZellijBin, args...)
		// A shell killed at the probe deadline can leave descendants holding its
		// stdout pipe open. Do not let those inherited descriptors extend the
		// recipient startup bound.
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

func startupStreamQuiet(scanned, consumed uint64, eventName string) bool {
	return scanned == consumed && eventName == ""
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
	streamCtx, cancelStream := context.WithCancel(ctx)
	defer cancelStream()
	req, err := http.NewRequestWithContext(streamCtx, http.MethodGet, endpoint, nil)
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
	mediaType, _, err := mime.ParseMediaType(response.Header.Get("Content-Type"))
	if err != nil || mediaType != "text/event-stream" {
		return fmt.Errorf("source stream: expected text/event-stream, found %q", response.Header.Get("Content-Type"))
	}

	var streamBoundary sync.Mutex
	var streamEnded atomic.Bool
	var transportActivity atomic.Uint64
	var transportPending atomic.Bool
	scanner := bufio.NewScanner(readinessReader{
		reader: response.Body, boundary: &streamBoundary,
		activity: &transportActivity, ended: &streamEnded,
		afterRead: cfg.afterRead, pending: &transportPending,
	})
	// An event is only a trigger today, but allow enough room for a server
	// diagnostic without silently tokenizing it.
	scanner.Buffer(make([]byte, 1024), 1024*1024)
	type scanResult struct {
		line string
		err  error
		done bool
	}
	lines := make(chan scanResult)
	var streamActivity atomic.Uint64
	var streamFragment atomic.Bool
	scanner.Split(func(data []byte, atEOF bool) (advance int, token []byte, err error) {
		if cfg.beforeSplit != nil {
			cfg.beforeSplit()
		}
		advance, token, err = bufio.ScanLines(data, atEOF)
		streamBoundary.Lock()
		streamFragment.Store(len(data) > advance && !atEOF)
		transportPending.Store(false)
		if token != nil {
			if cfg.afterTokenPendingClear != nil {
				cfg.afterTokenPendingClear()
			}
			streamActivity.Add(1)
			if cfg.afterScan != nil {
				cfg.afterScan()
			}
		}
		if atEOF && len(data) == 0 {
			streamEnded.Store(true)
		}
		streamBoundary.Unlock()
		return advance, token, err
	})
	go func() {
		for scanner.Scan() {
			if cfg.beforeLineSend != nil {
				cfg.beforeLineSend()
			}
			select {
			case lines <- scanResult{line: scanner.Text()}:
			case <-streamCtx.Done():
				return
			}
			if cfg.beforeNextScan != nil {
				cfg.beforeNextScan()
			}
		}
		streamBoundary.Lock()
		streamEnded.Store(true)
		streamBoundary.Unlock()
		select {
		case lines <- scanResult{err: scanner.Err(), done: true}:
		case <-streamCtx.Done():
		}
	}()

	ready := false
	readyTimer := time.NewTimer(100 * time.Millisecond)
	defer readyTimer.Stop()
	readyTimerC := readyTimer.C
	type handshakeResult struct {
		err error
	}
	var handshake <-chan handshakeResult
	var catchup <-chan error
	var settleTimer *time.Timer
	var settleC <-chan time.Time
	var settleGeneration uint64
	var settleTransportGeneration uint64
	var consumedActivity uint64
	pendingDataChange := false
	startSettle := func() {
		if settleTimer != nil {
			settleTimer.Stop()
		}
		settleTimer = time.NewTimer(25 * time.Millisecond)
		settleC = settleTimer.C
		settleGeneration = streamActivity.Load()
		settleTransportGeneration = transportActivity.Load()
	}
	startCatchup := func() {
		result := make(chan error, 1)
		catchup = result
		go func() {
			result <- refreshSessions(streamCtx, client, cfg, stableTabID, stderr)
		}()
	}
	startHandshake := func() {
		result := make(chan handshakeResult, 1)
		handshake = result
		go func() {
			if err := waitForRecipient(streamCtx, cfg); err != nil {
				result <- handshakeResult{err: err}
				return
			}
			sessions, err := snapshotSessions(streamCtx, client, cfg, stableTabID)
			if err == nil {
				err = deliverSnapshot(streamCtx, cfg, stableTabID, sessions, stderr)
			}
			result <- handshakeResult{err: err}
		}()
	}
	defer func() {
		if settleTimer != nil {
			settleTimer.Stop()
		}
	}()

	eventName := ""
	for {
		select {
		case <-ctx.Done():
			return nil
		case <-readyTimerC:
			readyTimerC = nil
			startHandshake()
		case result := <-handshake:
			handshake = nil
			if result.err != nil {
				return result.err
			}
			if pendingDataChange {
				pendingDataChange = false
				startCatchup()
			} else {
				startSettle()
			}
		case err := <-catchup:
			catchup = nil
			if err != nil {
				return err
			}
			if pendingDataChange {
				pendingDataChange = false
				startCatchup()
			} else {
				startSettle()
			}
		case <-settleC:
			settleC = nil
			if cfg.beforeReadinessCheck != nil {
				cfg.beforeReadinessCheck()
			}
			streamBoundary.Lock()
			scanned := streamActivity.Load()
			if scanned != settleGeneration || transportActivity.Load() != settleTransportGeneration ||
				transportPending.Load() || streamFragment.Load() ||
				!startupStreamQuiet(scanned, consumedActivity, eventName) {
				streamBoundary.Unlock()
				startSettle()
				continue
			}
			if pendingDataChange {
				streamBoundary.Unlock()
				pendingDataChange = false
				startCatchup()
				continue
			}
			if streamEnded.Load() {
				streamBoundary.Unlock()
				return ErrSourceEOF
			}
			if err := startupSignal(cfg.StartupFD); err != nil {
				streamBoundary.Unlock()
				return fmt.Errorf("stream-ready signal: %w", err)
			}
			ready = true
			streamBoundary.Unlock()
		case result := <-lines:
			if !result.done {
				consumedActivity++
			}
			if !ready && settleC != nil {
				startSettle()
			}
			if result.done {
				if ctx.Err() != nil {
					return nil
				}
				if result.err == nil {
					return ErrSourceEOF
				}
				return fmt.Errorf("source stream: %w", result.err)
			}
			if result.line == "" {
				if eventName == "data_changed" {
					if ready {
						if err := refreshSessions(ctx, client, cfg, stableTabID, stderr); err != nil {
							return err
						}
					} else {
						pendingDataChange = true
					}
				} else if eventName == "heartbeat" && ready {
					if _, err := probeTarget(ctx, cfg, stableTabID); err != nil {
						return err
					}
				}
				eventName = ""
				continue
			}
			if strings.HasPrefix(result.line, "event:") {
				eventName = strings.TrimSpace(strings.TrimPrefix(result.line, "event:"))
			}
		}
	}
}

// runSubscribe has exactly one source connection. Initial state is listed
// after connecting, then data_changed causes a fresh list. EOF, source
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
	if cfg.SourceTimeout <= 0 {
		cfg.SourceTimeout = 5 * time.Second
	}
	cfg.nativeDiagnostics = stderr
	if _, err := probeTarget(ctx, cfg, stableTabID); err != nil {
		return err
	}
	client := &http.Client{}
	err = streamEvents(ctx, client, cfg, stableTabID, stderr)
	if ctx.Err() != nil {
		return nil
	}
	return err
}
