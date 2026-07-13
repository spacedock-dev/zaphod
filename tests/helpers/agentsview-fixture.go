// ABOUTME: Serves an isolated empty AgentsView API plus a live SSE heartbeat.
// ABOUTME: The tmux/Zellij smoke owns this process and never uses ambient port 8080.

package main

import (
	"context"
	"flag"
	"fmt"
	"net"
	"net/http"
	"os"
	"os/signal"
	"syscall"
	"time"
)

func main() {
	readyFile := flag.String("ready-file", "", "path that receives the fixture URL")
	flag.Parse()
	if *readyFile == "" {
		fmt.Fprintln(os.Stderr, "--ready-file is required")
		os.Exit(2)
	}

	listener, err := net.Listen("tcp", "127.0.0.1:0")
	if err != nil {
		panic(err)
	}
	mux := http.NewServeMux()
	mux.HandleFunc("/api/v1/sessions", func(w http.ResponseWriter, _ *http.Request) {
		w.Header().Set("Content-Type", "application/json")
		fmt.Fprint(w, `{"sessions":[]}`)
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
