# Everything CI runs, locally. See CLAUDE.md for the workflow and README.md for the
# system packages Linux needs.

SHELL := /bin/bash
CARGO := cargo
MANIFEST := --manifest-path src-tauri/Cargo.toml
# A virtual display keeps the end-to-end window off the desktop when one is available.
XVFB := $(shell command -v xvfb-run >/dev/null 2>&1 && echo "xvfb-run -a")

.DEFAULT_GOAL := help
.PHONY: help deps run test test-bridge test-rust e2e fmt lint check bundle icons clean

help: ## list the targets
	@grep -hE '^[a-z-]+:.*?## ' $(MAKEFILE_LIST) | awk 'BEGIN{FS=":.*?## "}{printf "  \033[36m%-12s\033[0m %s\n", $$1, $$2}'

deps: ## install the Node tools (Tauri CLI, icon renderer)
	npm ci

run: ## the app in development
	$(CARGO) run $(MANIFEST)

test: test-bridge test-rust ## bridge, unit and IPC tests

test-bridge: ## bridge.js against a fake media session
	npm run test:bridge

test-rust: ## unit, IPC and capability tests
	$(CARGO) test $(MANIFEST)

e2e: ## the real binary in the real engine, against the fixture page
	$(XVFB) $(CARGO) test $(MANIFEST) --features e2e --test e2e

fmt: ## format the Rust code
	$(CARGO) fmt $(MANIFEST)

lint: ## rustfmt check and Clippy, with and without the e2e feature
	$(CARGO) fmt $(MANIFEST) --check
	$(CARGO) clippy $(MANIFEST) --all-targets -- -D warnings
	$(CARGO) clippy $(MANIFEST) --all-targets --features e2e -- -D warnings

check: lint test e2e ## everything CI runs, except the installers

bundle: ## the installers for this platform
	npm run tauri -- build

icons: ## regenerate the icons from assets/
	npm run icons

clean: ## remove build output
	$(CARGO) clean $(MANIFEST)
	rm -rf icons-build
