.DEFAULT_GOAL := build

DOCKER_IMAGE := "ghcr.io/flared/lacuna"

#####################
## GENERAL TARGETS ##
#####################

.PHONY: ci
ci: \
	build \
	test \
	format-check \
	clippy \
	frontend-ci

.PHONY: .all
.all: build test

.PHONY: build
build:
	cargo build

.PHONY: run
run: frontend-build
	# NOTE(aviau): Claude Desktop's preview mode passes the PORT environment
	# variable when the default port is not available:
	# - https://code.claude.com/docs/en/desktop#port-conflicts
	ANTHROPIC_API_KEY=$${ANTHROPIC_API_KEY:-} \
	BEDROCK_API_KEY=$${BEDROCK_API_KEY:-} \
	    cargo run -- --config examples/lacuna/lacuna.config.json --port=$${PORT:-3000}

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
clean: frontend-clean
	rm -rf target

#################
## API TARGETS ##
#################

.PHONY: test
test: frontend-build
	cargo test

.PHONY: format
format:
	cargo fmt

######################
## FRONTEND TARGETS ##
######################

.PHONY: frontend-build
frontend-build:
	$(MAKE) -C frontend build

.PHONY: frontend-check
frontend-check:
	$(MAKE) -C frontend check

.PHONY: frontend-format
frontend-format:
	$(MAKE) -C frontend format

.PHONY: frontend-lint
frontend-lint:
	$(MAKE) -C frontend lint

.PHONY: frontend-run
frontend-run:
	$(MAKE) -C frontend run

.PHONY: frontend-clean
frontend-clean:
	$(MAKE) -C frontend clean

.PHONY: frontend-ci
frontend-ci:
		$(MAKE) -C frontend ci

###################
## DOCKER TARGETS #
###################

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
		-v ./examples/lacuna/lacuna.config.json:/opt/lacuna/config.json \
		--env=ANTHROPIC_API_KEY=$${ANTHROPIC_API_KEY:-} \
		--env=BEDROCK_API_KEY=$${BEDROCK_API_KEY:-} \
		${DOCKER_IMAGE} \
		--host=0.0.0.0 \
		--port=3000 \
		--config=/opt/lacuna/config.json
