// ABOUTME: Private native sidecar entry plus internal row-building seams.
// ABOUTME: Only zaphod subscribe is executable; its target tuple is explicit.

package main

import (
	"bufio"
	"context"
	"errors"
	"flag"
	"fmt"
	"io"
	"os"
	"os/exec"
	"os/signal"
	"path/filepath"
	"strconv"
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
	fmt.Fprintln(stderr, "usage: zaphod watch-tab --server URL [--foreground]")
	fmt.Fprintln(stderr, "usage: zaphod subscribe --server URL --zellij-bin PATH --zellij-config-dir DIR --zellij-config FILE --zellij-data-dir DIR --zellij-session NAME --tab-id ID --rail-url URL --checkout-cwd PATH --recipient-token TOKEN [--registry-dir DIR]")
	fmt.Fprintln(stderr, "       zaphod register-agent-session [--watch-dir DIR]")
}

func runRegisterAgentSession(args []string, stdin io.Reader, stderr io.Writer) error {
	flags := flag.NewFlagSet("zaphod register-agent-session", flag.ContinueOnError)
	flags.SetOutput(stderr)
	watchDir := flags.String("watch-dir", defaultWatchRoot(), "private tab-watcher socket root")
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
	if !filepath.IsAbs(*watchDir) {
		return fmt.Errorf("register-agent-session watch directory must be absolute")
	}
	const maxHookBytes = 1 << 20
	payload, err := io.ReadAll(io.LimitReader(stdin, maxHookBytes+1))
	if err != nil {
		return fmt.Errorf("read Codex hook: %w", err)
	}
	if len(payload) > maxHookBytes {
		return fmt.Errorf("Codex hook exceeds %d bytes", maxHookBytes)
	}
	ctx, cancel := context.WithTimeout(context.Background(), 2*time.Second)
	defer cancel()
	return sendWatchHook(ctx, *watchDir, zellijSession, paneID, payload)
}

type WatchCommandConfig struct {
	WatchConfig
	Foreground bool
	StartupFD  int
}

func watchDaemonChildArgs(args []string, startupFD int) ([]string, error) {
	for _, arg := range args {
		if arg == "--foreground" || arg == "--startup-fd" || strings.HasPrefix(arg, "--startup-fd=") {
			return nil, fmt.Errorf("watch-tab daemon caller cannot set internal flag %s", arg)
		}
	}
	child := make([]string, 0, len(args)+4)
	child = append(child, "watch-tab")
	child = append(child, args...)
	child = append(child, "--foreground", "--startup-fd", strconv.Itoa(startupFD))
	return child, nil
}

func launchWatchDaemon(executable string, args []string, stderr io.Writer) error {
	childArgs, err := watchDaemonChildArgs(args, 3)
	if err != nil {
		return err
	}
	readiness, signalWriter, err := os.Pipe()
	if err != nil {
		return fmt.Errorf("create watcher readiness pipe: %w", err)
	}
	defer readiness.Close()

	logRoot := os.Getenv("ZAPHOD_ZELLIJ_DATA_DIR")
	if logRoot == "" {
		logRoot = os.TempDir()
	}
	if err := os.MkdirAll(logRoot, 0o700); err != nil {
		signalWriter.Close()
		return fmt.Errorf("create watcher log directory: %w", err)
	}
	logFile, err := os.CreateTemp(logRoot, "zaphod-watch-tab.*.log")
	if err != nil {
		signalWriter.Close()
		return fmt.Errorf("create watcher log: %w", err)
	}
	defer logFile.Close()
	_ = logFile.Chmod(0o600)

	command := exec.Command(executable, childArgs...)
	command.Stdin = nil
	command.Stdout = logFile
	command.Stderr = logFile
	command.ExtraFiles = []*os.File{signalWriter}
	command.SysProcAttr = &syscall.SysProcAttr{Setsid: true}
	if err := command.Start(); err != nil {
		signalWriter.Close()
		return fmt.Errorf("start watch-tab daemon: %w", err)
	}
	signalWriter.Close()
	failed := func(cause error) error {
		_ = command.Process.Kill()
		_ = command.Wait()
		return cause
	}
	if err := readiness.SetReadDeadline(time.Now().Add(10 * time.Second)); err != nil {
		return failed(fmt.Errorf("bound watcher readiness: %w", err))
	}
	line, err := bufio.NewReader(io.LimitReader(readiness, 64)).ReadString('\n')
	if err != nil {
		return failed(fmt.Errorf("watch-tab daemon exited before readiness: %w", err))
	}
	if line != "ready\n" {
		return failed(fmt.Errorf("watch-tab daemon returned invalid readiness %q", strings.TrimSpace(line)))
	}
	pid := command.Process.Pid
	if err := command.Process.Release(); err != nil {
		return fmt.Errorf("release watch-tab daemon: %w", err)
	}
	fmt.Fprintf(stderr, "watch-tab ready pid=%d log=%s\n", pid, logFile.Name())
	return nil
}

func defaultWatchRoot() string {
	if value := os.Getenv("ZAPHOD_WATCH_DIR"); value != "" {
		return value
	}
	if value := os.Getenv("XDG_RUNTIME_DIR"); value != "" {
		return filepath.Join(value, "zaphod", "watch-tab-v1")
	}
	return filepath.Join("/tmp", fmt.Sprintf("zaphod-watch-tab-v1-%d", os.Getuid()))
}

func parseWatchTabArgs(args []string, stderr io.Writer) (WatchCommandConfig, error) {
	flags := flag.NewFlagSet("zaphod watch-tab", flag.ContinueOnError)
	flags.SetOutput(stderr)
	server := flags.String("server", os.Getenv("ZAPHOD_AGENTSVIEW_URL"), "AgentsView server URL")
	watchDir := flags.String("watch-dir", defaultWatchRoot(), "private tab-watcher socket root")
	foreground := flags.Bool("foreground", false, "run the watcher in the foreground")
	startupFD := flags.Int("startup-fd", -1, "private daemon readiness descriptor")
	if err := flags.Parse(args); err != nil {
		return WatchCommandConfig{}, err
	}
	if flags.NArg() != 0 {
		return WatchCommandConfig{}, fmt.Errorf("watch-tab accepts flags only")
	}
	paneID, err := canonicalPaneID(os.Getenv("ZELLIJ_PANE_ID"))
	if err != nil {
		return WatchCommandConfig{}, err
	}
	values := map[string]string{
		"server": *server, "ZELLIJ_SESSION_NAME": os.Getenv("ZELLIJ_SESSION_NAME"),
		"ZAPHOD_RAIL_URL":           os.Getenv("ZAPHOD_RAIL_URL"),
		"ZAPHOD_RECIPIENT_TOKEN":    os.Getenv("ZAPHOD_RECIPIENT_TOKEN"),
		"ZAPHOD_ZELLIJ_CONFIG_DIR":  os.Getenv("ZAPHOD_ZELLIJ_CONFIG_DIR"),
		"ZAPHOD_ZELLIJ_CONFIG_FILE": os.Getenv("ZAPHOD_ZELLIJ_CONFIG_FILE"),
		"ZAPHOD_ZELLIJ_DATA_DIR":    os.Getenv("ZAPHOD_ZELLIJ_DATA_DIR"),
	}
	missing := make([]string, 0)
	for name, value := range values {
		if value == "" {
			missing = append(missing, name)
		}
	}
	if len(missing) > 0 {
		return WatchCommandConfig{}, fmt.Errorf("watch-tab missing explicit route context: %s", strings.Join(missing, ", "))
	}
	if !filepath.IsAbs(*watchDir) {
		return WatchCommandConfig{}, fmt.Errorf("watch-tab watch directory must be absolute")
	}
	zellijBin := os.Getenv("ZELLIJ_BIN")
	if zellijBin == "" {
		zellijBin = "zellij"
	}
	return WatchCommandConfig{
		WatchConfig: WatchConfig{
			SubscribeConfig: SubscribeConfig{
				ServerURL: *server, ZellijBin: zellijBin,
				ZellijConfigDir:  values["ZAPHOD_ZELLIJ_CONFIG_DIR"],
				ZellijConfigFile: values["ZAPHOD_ZELLIJ_CONFIG_FILE"],
				ZellijDataDir:    values["ZAPHOD_ZELLIJ_DATA_DIR"],
				ZellijSession:    values["ZELLIJ_SESSION_NAME"], RailURL: values["ZAPHOD_RAIL_URL"],
				RecipientToken: values["ZAPHOD_RECIPIENT_TOKEN"], PipeTimeout: 5 * time.Second,
				SourceTimeout: 5 * time.Second, SummaryClampBytes: 512,
			},
			PaneID: paneID, SocketRoot: *watchDir, Lease: 500 * time.Millisecond, Heartbeat: 200 * time.Millisecond,
		},
		Foreground: *foreground, StartupFD: *startupFD,
	}, nil
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
	registryDir := flags.String("registry-dir", defaultAgentRegistryDir(), "private agent-session registry root")
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
		RegistryDir:       *registryDir,
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
	if args[0] == "watch-tab" {
		cfg, err := parseWatchTabArgs(args[1:], stderr)
		if err != nil {
			if !errors.Is(err, flag.ErrHelp) {
				fmt.Fprintln(stderr, err)
			}
			return 2
		}
		if !cfg.Foreground {
			executable, err := os.Executable()
			if err != nil {
				fmt.Fprintln(stderr, err)
				return 1
			}
			if err := launchWatchDaemon(executable, args[1:], stderr); err != nil {
				fmt.Fprintln(stderr, err)
				return 1
			}
			return 0
		}
		ctx, stop := signal.NotifyContext(context.Background(), os.Interrupt, syscall.SIGTERM)
		defer stop()
		ready := make(chan WatchReady, 1)
		cfg.Ready = ready
		watchResult := make(chan error, 1)
		go func() { watchResult <- runWatchTab(ctx, cfg.WatchConfig, stderr) }()
		select {
		case started := <-ready:
			if err := startupSignal(cfg.StartupFD); err != nil {
				stop()
				<-watchResult
				fmt.Fprintln(stderr, err)
				return 1
			}
			fmt.Fprintf(stderr, "watch-tab foreground ready session=%s tab=%d pane=%d rail=%d socket=%s generation=%s\n",
				cfg.ZellijSession, started.Target.TabID, started.Target.TerminalPaneID, started.Target.RailPaneID,
				started.SocketPath, started.Generation)
		case err := <-watchResult:
			if err != nil {
				fmt.Fprintln(stderr, err)
				return 1
			}
			return 0
		}
		if err := <-watchResult; err != nil {
			fmt.Fprintln(stderr, err)
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
