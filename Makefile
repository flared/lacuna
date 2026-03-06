DEFAULT_GOAL := .all

.PHONY: .all
.all: build

.PHONY: build
build:
	cargo build

.PHONY: docker-build
docker-build:
	docker build -t ghcr.io/flared/lacuna .

.PHONY: clean
clean:
	rm -rf target
