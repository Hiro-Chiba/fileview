package main

import (
	"net/http/httptest"
	"testing"
)

func TestHealthReturnsOK(t *testing.T) {
	r := NewRouter()
	rec := httptest.NewRecorder()
	r.ServeHTTP(rec, httptest.NewRequest("GET", "/healthz", nil))
	if rec.Code != 200 {
		t.Fatalf("got %d", rec.Code)
	}
}
