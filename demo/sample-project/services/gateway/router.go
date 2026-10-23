package main

import (
	"encoding/json"
	"net/http"
	"sync/atomic"
)

type Router struct {
	mux      *http.ServeMux
	requests atomic.Int64
}

func NewRouter() *Router {
	r := &Router{mux: http.NewServeMux()}
	r.mux.HandleFunc("GET /healthz", r.health)
	r.mux.HandleFunc("POST /jobs", r.submit)
	return r
}

func (r *Router) ServeHTTP(w http.ResponseWriter, req *http.Request) {
	r.requests.Add(1)
	r.mux.ServeHTTP(w, req)
}

func (r *Router) health(w http.ResponseWriter, _ *http.Request) {
	json.NewEncoder(w).Encode(map[string]any{"ok": true, "requests": r.requests.Load()})
}

// TODO: forward to the scheduler over gRPC instead of returning 202 directly
func (r *Router) submit(w http.ResponseWriter, _ *http.Request) {
	w.WriteHeader(http.StatusAccepted)
}
