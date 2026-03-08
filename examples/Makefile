.DEFAULT_GOAL := ci

RUFF_VERSION := 0.15.5
TY_VERSION := 0.0.21

RUFF_RUN := uvx ruff@${RUFF_VERSION}
TY_RUN := uvx ty@${TY_VERSION}

.PHONY: ci
ci: check format-check

PYTHON_FILES := $(shell find . -name '*.py')

.PHONY: check
check:
	for python_file in $(PYTHON_FILES); do \
		uvx --with-requirements $${python_file} ty@${TY_VERSION} check $${python_file}; \
	done

.PHONY: format-check
format-check:
	${RUFF_RUN} check
	${RUFF_RUN} format --check

.PHONY: format
format:
	${RUFF_RUN} check --fix --unsafe-fixes
	${RUFF_RUN} format

.PHONY: clean
clean:
	rm -rf .ruff_cache
