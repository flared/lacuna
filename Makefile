.DEFAULT_GOAL := build

.PHONY: ci
ci: build test

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

.PHONY: clean
clean:
	rm -rf target
