.DEFAULT_GOAL := build

.PHONY: ci
ci: build test format-check clippy

.PHONY: .all
.all: build test

.PHONY: build
build:
	cargo build

.PHONY: docker-build
docker-build:
	docker build -t ghcr.io/flared/lacuna .

.PHONY: test
test:
	cargo test

.PHONY: format
format:
	cargo fmt

.PHONY: fix
fix:
	cargo fix --allow-dirty

.PHONY: format-check
format-check:
	cargo fmt --check

.PHONY: clippy
clippy:
	cargo clippy

.PHONY: clean
clean:
	rm -rf target
