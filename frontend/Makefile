node_modules: pnpm-lock.yaml
	rm -rf node_modules && pnpm install --frozen-lockfile

SRC_FILES := $(shell find src -type f) index.html pnpm-lock.yaml vite.config.ts tsconfig.json

.PHONY: ci
ci: build check lint

.PHONY: clean
clean:
	rm -rf node_modules dist .vite

.PHONY: check
check: node_modules
	pnpm run check

dist/.build-stamp: node_modules $(SRC_FILES)
	pnpm run build || rm -rf dist
	@touch $@

.PHONY: build
build: dist/.build-stamp

.PHONY: format
format: node_modules
	pnpm run format

.PHONY: lint
lint: node_modules
	pnpm run lint

.PHONY: run
run: node_modules
	pnpm run dev
