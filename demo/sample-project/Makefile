.PHONY: build test web lint

build:
	cargo build --workspace

test:
	cargo test --workspace
	go test ./services/...
	pytest ml

web:
	npm --prefix web run dev

lint:
	cargo clippy --workspace -- -D warnings
