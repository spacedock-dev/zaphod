// ABOUTME: Serves two exact top-level sessions plus one unregistered child.
// ABOUTME: Logs every request so the native two-tab smoke can prove no list fallback.

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
	"sync"
	"syscall"
	"time"
)

const (
	firstID  = "codex:019f5f94-a596-7d92-9928-398653669161"
	secondID = "codex:019f5f95-bbfd-7993-8620-0d698008217f"
	childID  = "codex:019f5f95-cc22-77d2-9c3a-271b1edaabd8"
)

func main() {
	readyFile := flag.String("ready-file", "", "path that receives the fixture URL")
	requestLog := flag.String("request-log", "", "append-only request path log")
	flag.Parse()
	if *readyFile == "" || *requestLog == "" {
		fmt.Fprintln(os.Stderr, "--ready-file and --request-log are required")
		os.Exit(2)
	}
	listener, err := net.Listen("tcp", "127.0.0.1:0")
	if err != nil {
		panic(err)
	}
	var logLock sync.Mutex
	logRequest := func(path string) {
		logLock.Lock()
		defer logLock.Unlock()
		file, err := os.OpenFile(*requestLog, os.O_CREATE|os.O_APPEND|os.O_WRONLY, 0o600)
		if err != nil {
			panic(err)
		}
		defer file.Close()
		fmt.Fprintln(file, path)
	}
	record := func(id string) map[string]any {
		marker := map[string]string{firstID: "KJ_TAB_A_ROW", secondID: "KJ_TAB_B_ROW", childID: "KJ_CHILD_ROW"}[id]
		return map[string]any{
			"id": id, "cwd": "/same/cwd", "agent": "codex", "termination_status": "awaiting_user",
			"first_message": marker, "created_at": "2026-07-14T00:00:00Z",
		}
	}
	mux := http.NewServeMux()
	mux.HandleFunc("/api/v1/sessions/", func(w http.ResponseWriter, r *http.Request) {
		logRequest(r.URL.Path)
		id := strings.TrimPrefix(r.URL.Path, "/api/v1/sessions/")
		if id != firstID && id != secondID && id != childID {
			http.NotFound(w, r)
			return
		}
		w.Header().Set("Content-Type", "application/json")
		_ = json.NewEncoder(w).Encode(record(id))
	})
	mux.HandleFunc("/api/v1/sessions", func(w http.ResponseWriter, r *http.Request) {
		logRequest(r.URL.Path)
		_ = json.NewEncoder(w).Encode(map[string]any{"sessions": []map[string]any{record(firstID), record(secondID), record(childID)}})
	})
	mux.HandleFunc("/api/v1/events", func(w http.ResponseWriter, r *http.Request) {
		logRequest(r.URL.Path)
		w.Header().Set("Content-Type", "text/event-stream")
		fmt.Fprint(w, "event: heartbeat\ndata: {}\n\n")
		w.(http.Flusher).Flush()
		<-r.Context().Done()
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
