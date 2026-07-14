// ABOUTME: Serves an isolated AgentsView session API plus a live SSE heartbeat.
// ABOUTME: The tmux/Zellij smoke owns this process and never uses ambient port 8080.

package main

import (
	"context"
	"encoding/json"
	"flag"
	"fmt"
	"net"
	"net/http"
	"os"
	"os/signal"
	"strings"
	"syscall"
	"time"
)

func main() {
	readyFile := flag.String("ready-file", "", "path that receives the fixture URL")
	cwd := flag.String("cwd", "", "session cwd returned by the fixture")
	triggerFile := flag.String("trigger-file", "", "file that adds a second session and emits data_changed")
	flag.Parse()
	if *readyFile == "" || *cwd == "" || *triggerFile == "" {
		fmt.Fprintln(os.Stderr, "--ready-file, --cwd, and --trigger-file are required")
		os.Exit(2)
	}

	listener, err := net.Listen("tcp", "127.0.0.1:0")
	if err != nil {
		panic(err)
	}
	mux := http.NewServeMux()
	const sessionID = "codex:019f5f94-a596-7d92-9928-398653669161"
	sessionRecord := func() map[string]any {
		firstMessage := "SMOKE_INITIAL_ROW"
		if _, err := os.Stat(*triggerFile); err == nil {
			firstMessage = "SMOKE_SECOND_ROW"
		}
		return map[string]any{
			"id": sessionID, "cwd": *cwd, "agent": "codex",
			"termination_status": "awaiting_user", "first_message": firstMessage,
			"created_at": "2026-07-13T00:00:00Z",
		}
	}
	mux.HandleFunc("/api/v1/sessions/", func(w http.ResponseWriter, r *http.Request) {
		if strings.TrimPrefix(r.URL.Path, "/api/v1/sessions/") != sessionID {
			http.NotFound(w, r)
			return
		}
		w.Header().Set("Content-Type", "application/json")
		_ = json.NewEncoder(w).Encode(sessionRecord())
	})
	mux.HandleFunc("/api/v1/sessions", func(w http.ResponseWriter, _ *http.Request) {
		w.Header().Set("Content-Type", "application/json")
		_ = json.NewEncoder(w).Encode(map[string]any{"sessions": []map[string]any{sessionRecord()}})
	})
	mux.HandleFunc("/api/v1/events", func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Type", "text/event-stream")
		w.Header().Set("Cache-Control", "no-cache")
		flusher, ok := w.(http.Flusher)
		if !ok {
			http.Error(w, "streaming unavailable", http.StatusInternalServerError)
			return
		}
		fmt.Fprint(w, "event: heartbeat\ndata: {}\n\n")
		flusher.Flush()
		ticker := time.NewTicker(10 * time.Millisecond)
		defer ticker.Stop()
		for {
			select {
			case <-r.Context().Done():
				return
			case <-ticker.C:
				if _, err := os.Stat(*triggerFile); err == nil {
					fmt.Fprint(w, "event: data_changed\ndata: {\"scope\":\"sessions\"}\n\n")
					flusher.Flush()
					<-r.Context().Done()
					return
				}
			}
		}
	})

	server := &http.Server{Handler: mux}
	go func() {
		if err := server.Serve(listener); err != nil && err != http.ErrServerClosed {
			panic(err)
		}
	}()
	if err := os.WriteFile(*readyFile, []byte("http://"+listener.Addr().String()+"\n"), 0o600); err != nil {
		panic(err)
	}

	stop := make(chan os.Signal, 1)
	signal.Notify(stop, os.Interrupt, syscall.SIGTERM)
	<-stop
	ctx, cancel := context.WithTimeout(context.Background(), time.Second)
	defer cancel()
	_ = server.Shutdown(ctx)
}
