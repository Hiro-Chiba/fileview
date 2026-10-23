package main

import (
	"log"
	"net/http"
	"os"
)

func main() {
	addr := os.Getenv("GATEWAY_ADDR")
	if addr == "" {
		addr = ":9090"
	}

	r := NewRouter()
	log.Printf("gateway listening on %s", addr)
	log.Fatal(http.ListenAndServe(addr, r))
}
