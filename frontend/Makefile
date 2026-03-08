node_modules: bun.lock
	rm -rf node_modules && bun install --frozen-lockfile

SRC_FILES := $(shell find src -type f) index.html pnpm-lock.yaml vite.config.ts tsconfig.json

.PHONY: clean
clean:
	rm -rf node_modules dist .vite

.PHONY: check
check: node_modules
	bun run check

dist/.build-stamp: node_modules $(SRC_FILES)
	bun run build || rm -rf dist
	@touch $@

.PHONY: build
build: dist/.build-stamp

.PHONY: format
format: node_modules
	bun run format

.PHONY: format-check
format-check: node_modules
	bun run format-check

.PHONY: run
run: node_modules
	bun run dev
