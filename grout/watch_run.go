// ABOUTME: Foreground core of the manually launched tab-local watcher.
// ABOUTME: It owns one live registration, exact projection, and leased delivery.

package main

import (
	"bufio"
	"context"
	"crypto/rand"
	"encoding/hex"
	"errors"
	"fmt"
	"io"
	"mime"
	"net"
	"net/http"
	"os"
	"strconv"
	"strings"
	"time"
)

type WatchConfig struct {
	WatchRoute
	PaneID           uint32
	SocketRoot       string
	Lease            time.Duration
	Heartbeat        time.Duration
	AuthorityTimeout time.Duration
	Ready            chan<- WatchReady
}

type WatchReady struct {
	Target     WatchTarget
	SocketPath string
	Generation string
}

type watchSourceEvent uint8

const (
	watchSourceHeartbeat watchSourceEvent = iota + 1
	watchSourceDataChanged
)

func runWatchTab(ctx context.Context, cfg WatchConfig, stderr io.Writer) (result error) {
	if cfg.Lease <= 0 {
		cfg.Lease = 2500 * time.Millisecond
	}
	if cfg.Heartbeat <= 0 {
		cfg.Heartbeat = 1400 * time.Millisecond
	}
	if cfg.PipeTimeout <= 0 {
		cfg.PipeTimeout = 5 * time.Second
	}
	if cfg.SourceTimeout <= 0 {
		cfg.SourceTimeout = 5 * time.Second
	}
	if cfg.SummaryClampBytes <= 0 {
		cfg.SummaryClampBytes = 512
	}
	if cfg.AuthorityTimeout <= 0 {
		cfg.AuthorityTimeout = 5 * time.Second
	}
	if cfg.SocketRoot == "" || cfg.ServerURL == "" || cfg.ZellijBin == "" || cfg.ZellijSession == "" ||
		cfg.RailURL == "" || cfg.RecipientToken == "" {
		return fmt.Errorf("watch-tab requires source, Zellij target, rail, recipient, and socket root")
	}
	resolveCtx, cancelResolve := context.WithTimeout(ctx, cfg.AuthorityTimeout)
	target, err := resolveWatchTarget(resolveCtx, cfg.WatchRoute, cfg.PaneID)
	cancelResolve()
	if err != nil {
		return err
	}
	cfg.TabID = strconv.FormatUint(target.TabID, 10)
	cfg.RailID = strconv.FormatUint(target.RailPaneID, 10)
	listener, socketPath, err := listenWatchSocket(cfg.SocketRoot, cfg.ZellijSession, strconv.FormatUint(uint64(cfg.PaneID), 10))
	if err != nil {
		return err
	}
	defer func() {
		_ = listener.Close()
		_ = os.Remove(socketPath)
	}()

	client := &http.Client{}
	streamEvents, streamErrors, closeStream, err := openWatchEventStream(ctx, client, cfg.WatchRoute)
	if err != nil {
		return err
	}
	defer closeStream()
	registrations := make(chan WatchRegistration, 1)
	acceptErrors := make(chan error, 1)
	go acceptWatchHooks(ctx, listener, cfg.ZellijSession, strconv.FormatUint(uint64(cfg.PaneID), 10), registrations, acceptErrors)

	generation, err := newWatchGeneration()
	if err != nil {
		return err
	}
	var registration *WatchRegistration
	rows := make([]SessionRow, 0, 1)
	emit := func() error {
		return EmitLeasedSnapshotForTab(ctx, cfg.emitConfig(), rows, cfg.TabID, cfg.RailID, cfg.RecipientToken, generation, cfg.Lease, stderr)
	}
	heartbeat := func() error {
		return EmitLeaseHeartbeatForTab(ctx, cfg.emitConfig(), cfg.TabID, cfg.RailID, cfg.RecipientToken, generation, cfg.Lease, stderr)
	}
	refreshSource := func() error {
		nextRows := make([]SessionRow, 0, 1)
		if registration != nil {
			session, err := fetchExactSession(ctx, client, cfg.ServerURL, registration.AgentsViewSessionID, cfg.SourceTimeout)
			if err != nil && !errors.Is(err, ErrExactSessionNotFound) {
				return err
			}
			if err == nil {
				nextRows = append(nextRows, BuildRegisteredSessionRow(session, registration.PaneID, time.Now(), cfg.SummaryClampBytes))
			}
		}
		rows = nextRows
		return emit()
	}
	refreshLeasedSource := func() error {
		if err := heartbeat(); err != nil {
			return err
		}
		return refreshSource()
	}
	if err := waitForRecipient(ctx, cfg.WatchRoute); err != nil {
		return err
	}
	select {
	case next := <-registrations:
		copy := next
		registration = &copy
	default:
	}
	if registration != nil {
		err = refreshSource()
	} else {
		err = emit()
	}
	if err != nil {
		return err
	}
	if cfg.Ready != nil {
		select {
		case cfg.Ready <- WatchReady{Target: target, SocketPath: socketPath, Generation: generation}:
		case <-ctx.Done():
			return nil
		}
	}

	ticker := time.NewTicker(cfg.Heartbeat)
	defer ticker.Stop()
	for {
		select {
		case <-ctx.Done():
			return nil
		case err := <-acceptErrors:
			if ctx.Err() != nil || errors.Is(err, os.ErrClosed) {
				return nil
			}
			return err
		case next := <-registrations:
			copy := next
			registration = &copy
			if err := refreshLeasedSource(); err != nil {
				if ctx.Err() != nil {
					return nil
				}
				return err
			}
		case event := <-streamEvents:
			if event == watchSourceDataChanged {
				if err := refreshLeasedSource(); err != nil {
					if ctx.Err() != nil {
						return nil
					}
					return err
				}
			} else if err := heartbeat(); err != nil {
				if ctx.Err() != nil {
					return nil
				}
				return err
			}
		case err := <-streamErrors:
			if ctx.Err() != nil {
				return nil
			}
			return err
		case <-ticker.C:
			info, err := os.Lstat(socketPath)
			if err != nil || info.Mode()&os.ModeSocket == 0 || info.Mode().Perm() != 0o600 {
				return fmt.Errorf("watch socket lost")
			}
			if err := heartbeat(); err != nil {
				if ctx.Err() != nil {
					return nil
				}
				return err
			}
		}
	}
}

func acceptWatchHooks(
	ctx context.Context,
	listener *net.UnixListener,
	zellijSession, paneValue string,
	registrations chan<- WatchRegistration,
	errs chan<- error,
) {
	for {
		_, err := acceptWatchRegistrationWithAdmission(ctx, listener, zellijSession, paneValue, func(registration WatchRegistration) error {
			select {
			case registrations <- registration:
				return nil
			case <-ctx.Done():
				return ctx.Err()
			}
		})
		if err != nil {
			if errors.Is(err, errWatchSocketProbe) {
				continue
			}
			select {
			case errs <- err:
			case <-ctx.Done():
			}
			return
		}
	}
}

func newWatchGeneration() (string, error) {
	var value [16]byte
	if _, err := rand.Read(value[:]); err != nil {
		return "", fmt.Errorf("create watch generation: %w", err)
	}
	return hex.EncodeToString(value[:]), nil
}

func openWatchEventStream(
	ctx context.Context,
	client *http.Client,
	cfg WatchRoute,
) (<-chan watchSourceEvent, <-chan error, func(), error) {
	endpoint, err := serverEndpoint(cfg.ServerURL, "/api/v1/events")
	if err != nil {
		return nil, nil, nil, err
	}
	streamCtx, cancel := context.WithCancel(ctx)
	req, err := http.NewRequestWithContext(streamCtx, http.MethodGet, endpoint, nil)
	if err != nil {
		cancel()
		return nil, nil, nil, err
	}
	req.Header.Set("Accept", "text/event-stream")
	response, err := client.Do(req)
	if err != nil {
		cancel()
		return nil, nil, nil, fmt.Errorf("source stream: %w", err)
	}
	if response.StatusCode != http.StatusOK {
		response.Body.Close()
		cancel()
		return nil, nil, nil, fmt.Errorf("source stream: unexpected HTTP status %s", response.Status)
	}
	mediaType, _, err := mime.ParseMediaType(response.Header.Get("Content-Type"))
	if err != nil || mediaType != "text/event-stream" {
		response.Body.Close()
		cancel()
		return nil, nil, nil, fmt.Errorf("source stream: expected text/event-stream, found %q", response.Header.Get("Content-Type"))
	}
	events := make(chan watchSourceEvent, 1)
	errs := make(chan error, 1)
	go func() {
		defer response.Body.Close()
		scanner := bufio.NewScanner(response.Body)
		scanner.Buffer(make([]byte, 1024), 1<<20)
		eventName := ""
		for scanner.Scan() {
			line := scanner.Text()
			if strings.HasPrefix(line, "event:") {
				eventName = strings.TrimSpace(strings.TrimPrefix(line, "event:"))
				continue
			}
			if line == "" {
				if eventName == "data_changed" {
					select {
					case events <- watchSourceDataChanged:
					case <-streamCtx.Done():
						return
					}
				} else if eventName == "heartbeat" {
					select {
					case events <- watchSourceHeartbeat:
					default:
					}
				}
				eventName = ""
			}
		}
		if streamCtx.Err() == nil {
			if err := scanner.Err(); err != nil {
				errs <- fmt.Errorf("source stream: %w", err)
			} else {
				errs <- ErrSourceEOF
			}
		}
	}()
	closeFn := func() {
		cancel()
		response.Body.Close()
	}
	return events, errs, closeFn, nil
}
