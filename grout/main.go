// ABOUTME: Private native sidecar entry plus internal row-building seams.
// ABOUTME: Only zaphod subscribe is executable; its target tuple is explicit.

package main

import (
	"context"
	"errors"
	"flag"
	"fmt"
	"io"
	"os"
	"os/signal"
	"path/filepath"
	"strings"
	"syscall"
	"time"
)

type Config struct {
	AgentsviewBin     string        // "agentsview"
	SessionID         string        // required argv[1]
	GateLog           string        // required argv[2]
	ZellijBin         string        // "zellij"
	ZellijConfigDir   string        // explicit Zellij profile root, when known
	ZellijConfigFile  string        // explicit Zellij config file, when known
	ZellijDataDir     string        // explicit Zellij data root, when known
	ZellijSession     string        // explicit target session, when known
	PipeName          string        // "agent-event" — protocol constant
	PipeTimeout       time.Duration // 5 * time.Second — kill timer
	SummaryClampBytes int           // 512
}

// startupSignal writes one short confirmation to the direct script's private
// FIFO after the SSE response, exact recipient, and one bounded initial
// snapshot delivery are all acknowledged.
func startupSignal(fd int) error {
	if fd == -1 {
		return nil
	}
	if fd < 3 {
		return fmt.Errorf("startup fd must be 3 or greater")
	}
	file := os.NewFile(uintptr(fd), "zaphod-startup-signal")
	if file == nil {
		return fmt.Errorf("startup fd %d is unavailable", fd)
	}
	defer file.Close()
	if _, err := io.WriteString(file, "ready\n"); err != nil {
		return fmt.Errorf("startup signal: %w", err)
	}
	return nil
}

// defaultConfig carries only cwd-independent knobs. SessionID and GateLog have
// no default: a demo session id exists only in one machine's DB and a relative
// gate-log path resolves against the cwd, so both must come from argv.
func defaultConfig() Config {
	return Config{
		AgentsviewBin:     "agentsview",
		ZellijBin:         "zellij",
		ZellijSession:     "",
		PipeName:          "agent-event",
		PipeTimeout:       5 * time.Second,
		SummaryClampBytes: 512,
	}
}

// run emits the session row then the gate row. Source failures are fatal
// before anything is piped; pipe failures are per-row independent — a
// timed-out session pipe must not stop the gate row.
func run(cfg Config, stderr io.Writer) error {
	si, err := FetchSession(cfg.AgentsviewBin, cfg.SessionID)
	if err != nil {
		return err
	}
	gi, err := GateFromLog(cfg.GateLog)
	if err != nil {
		return err
	}

	sRow := BuildSessionRow(si, time.Now(), cfg.SummaryClampBytes)
	gRow := BuildGateRow(gi, time.Now())
	failed := false
	for _, it := range []struct {
		kind string
		row  any
	}{{sRow.Kind, sRow}, {gRow.Kind, gRow}} {
		if err := EmitRow(cfg, it.kind, it.row, stderr); err != nil {
			fmt.Fprintln(stderr, err)
			failed = true
		}
	}
	if failed {
		return fmt.Errorf("not every row was piped cleanly")
	}
	return nil
}

func subscribeUsage(stderr io.Writer) {
	fmt.Fprintln(stderr, "usage: zaphod subscribe --server URL --zellij-bin PATH --zellij-config-dir DIR --zellij-config FILE --zellij-data-dir DIR --zellij-session NAME --tab-id ID --rail-url URL --checkout-cwd PATH --recipient-token TOKEN")
	fmt.Fprintln(stderr, "       zaphod register-agent-session [--registry-dir DIR]")
}

func runRegisterAgentSession(args []string, stdin io.Reader, stderr io.Writer) error {
	flags := flag.NewFlagSet("zaphod register-agent-session", flag.ContinueOnError)
	flags.SetOutput(stderr)
	registryDir := flags.String("registry-dir", defaultAgentRegistryDir(), "private agent-session registry root")
	if err := flags.Parse(args); err != nil {
		return err
	}
	if flags.NArg() != 0 {
		return fmt.Errorf("register-agent-session accepts flags only")
	}
	zellijSession := os.Getenv("ZELLIJ_SESSION_NAME")
	paneID := os.Getenv("ZELLIJ_PANE_ID")
	if zellijSession == "" && paneID == "" {
		return nil
	}
	if zellijSession == "" || paneID == "" {
		return fmt.Errorf("register-agent-session requires both ZELLIJ_SESSION_NAME and ZELLIJ_PANE_ID")
	}
	if !filepath.IsAbs(*registryDir) {
		return fmt.Errorf("register-agent-session registry directory must be absolute")
	}
	const maxHookBytes = 1 << 20
	payload, err := io.ReadAll(io.LimitReader(stdin, maxHookBytes+1))
	if err != nil {
		return fmt.Errorf("read Codex hook: %w", err)
	}
	if len(payload) > maxHookBytes {
		return fmt.Errorf("Codex hook exceeds %d bytes", maxHookBytes)
	}
	registration, err := decodeCodexRegistration(payload, zellijSession, paneID, os.Getpid(), time.Now())
	if err != nil {
		return err
	}
	return (agentRegistryStore{root: *registryDir}).upsert(registration)
}

func parseSubscribeArgs(args []string, stderr io.Writer) (SubscribeConfig, error) {
	flags := flag.NewFlagSet("zaphod subscribe", flag.ContinueOnError)
	flags.SetOutput(stderr)
	server := flags.String("server", "", "AgentsView server URL")
	zellijBin := flags.String("zellij-bin", "", "Zellij binary")
	configDir := flags.String("zellij-config-dir", "", "Zellij config directory")
	configFile := flags.String("zellij-config", "", "Zellij config file")
	dataDir := flags.String("zellij-data-dir", "", "Zellij data directory")
	session := flags.String("zellij-session", "", "Zellij session")
	tabID := flags.String("tab-id", "", "stable Zellij tab ID")
	railURL := flags.String("rail-url", "", "canonical sidebar WASM URL")
	checkoutCWD := flags.String("checkout-cwd", "", "selected checkout root")
	recipientToken := flags.String("recipient-token", "", "private direct-entry recipient token")
	startupFD := flags.Int("startup-fd", -1, "private direct-script stream-ready confirmation fd")
	if err := flags.Parse(args); err != nil {
		return SubscribeConfig{}, err
	}
	if flags.NArg() != 0 {
		return SubscribeConfig{}, fmt.Errorf("subscribe accepts flags only")
	}
	missing := make([]string, 0, 8)
	for _, flag := range []struct {
		name  string
		value string
	}{
		{"--server", *server},
		{"--zellij-bin", *zellijBin},
		{"--zellij-config-dir", *configDir},
		{"--zellij-config", *configFile},
		{"--zellij-data-dir", *dataDir},
		{"--zellij-session", *session},
		{"--tab-id", *tabID},
		{"--rail-url", *railURL},
		{"--checkout-cwd", *checkoutCWD},
		{"--recipient-token", *recipientToken},
	} {
		if flag.value == "" {
			missing = append(missing, flag.name)
		}
	}
	if len(missing) > 0 {
		return SubscribeConfig{}, fmt.Errorf("subscribe requires %s", strings.Join(missing, ", "))
	}
	if *startupFD < -1 || (*startupFD >= 0 && *startupFD < 3) {
		return SubscribeConfig{}, fmt.Errorf("subscribe startup fd must be 3 or greater")
	}
	return SubscribeConfig{
		ServerURL:         *server,
		ZellijBin:         *zellijBin,
		ZellijConfigDir:   *configDir,
		ZellijConfigFile:  *configFile,
		ZellijDataDir:     *dataDir,
		ZellijSession:     *session,
		TabID:             *tabID,
		RailURL:           *railURL,
		CheckoutCWD:       *checkoutCWD,
		RecipientToken:    *recipientToken,
		StartupFD:         *startupFD,
		PipeTimeout:       5 * time.Second,
		SummaryClampBytes: 512,
	}, nil
}

func runMain(args []string, stderr io.Writer) int {
	if len(args) == 0 {
		subscribeUsage(stderr)
		return 2
	}
	if args[0] == "register-agent-session" {
		if err := runRegisterAgentSession(args[1:], os.Stdin, stderr); err != nil {
			if !errors.Is(err, flag.ErrHelp) {
				fmt.Fprintln(stderr, err)
			}
			return 1
		}
		return 0
	}
	if args[0] != "subscribe" {
		subscribeUsage(stderr)
		return 2
	}
	cfg, err := parseSubscribeArgs(args[1:], stderr)
	if err != nil {
		if !errors.Is(err, flag.ErrHelp) {
			fmt.Fprintln(stderr, err)
		}
		subscribeUsage(stderr)
		return 2
	}
	ctx, stop := signal.NotifyContext(context.Background(), os.Interrupt, syscall.SIGTERM)
	defer stop()
	if err := runSubscribe(ctx, cfg, stderr); err != nil {
		switch {
		case errors.Is(err, ErrTargetLost):
			fmt.Fprintf(stderr, "target-lost: %v\n", err)
		case errors.Is(err, ErrSourceEOF):
			fmt.Fprintln(stderr, "source-eof")
		default:
			fmt.Fprintln(stderr, err)
		}
		return 1
	}
	return 0
}

func main() {
	os.Exit(runMain(os.Args[1:], os.Stderr))
}
