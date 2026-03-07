.DEFAULT_GOAL := build

DOCKER_IMAGE := "ghcr.io/flared/lacuna"

.PHONY: ci
ci: build test format-check clippy

.PHONY: .all
.all: build test

.PHONY: build
build:
	cargo build

.PHONY: run
run:
	# NOTE(aviau): Claude Desktop's preview mode passes the PORT environment
	# variable when the default port is not available:
	# - https://code.claude.com/docs/en/desktop#port-conflicts
	ANTHROPIC_API_KEY=$${ANTHROPIC_API_KEY:-} \
	BEDROCK_API_KEY=$${BEDROCK_API_KEY:-} \
	    cargo run -- --config example/config.json --port=$${PORT:-3000}

.PHONY: docker-build
docker-build:
	docker build -t ${DOCKER_IMAGE} .

.PHONY: docker-run
docker-run:
	docker run \
		--init \
		-it \
		--rm \
		-p 3000:3000 \
		-v ./example/config.json:/opt/lacuna/config.json \
		--env=ANTHROPIC_API_KEY=$${ANTHROPIC_API_KEY:-} \
		--env=BEDROCK_API_KEY=$${BEDROCK_API_KEY:-} \
		${DOCKER_IMAGE} \
		--host=0.0.0.0 \
		--port=3000 \
		--config=/opt/lacuna/config.json

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
