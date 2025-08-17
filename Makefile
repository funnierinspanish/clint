# Makefile for CLINT - CLI Navigator Toolkit

# Variables
BINARY_NAME := clint
BIN_DIR_PATH ?= $(HOME)/.local/bin
BIN_PATH     := ./target/$(PROFILE)/$(PROGRAM_NAME)

RUST_VERSION := stable
TARGET_DIR := target
RELEASE_DIR := $(TARGET_DIR)/release
DEBUG_DIR := $(TARGET_DIR)/debug

# Default target
.PHONY: help
help: ## Show this help message
	@echo "CLINT - CLI Navigator Toolkit"
	@echo ""
	@echo "Available targets:"
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-20s\033[0m %s\n", $$1, $$2}'

# Development targets
.PHONY: install
install: ## Install Rust toolchain and dependencies
	@echo "Installing Rust toolchain..."
	rustup install $(RUST_VERSION)
	rustup default $(RUST_VERSION)
	rustup component add rustfmt clippy

.PHONY: deps
deps: ## Check and install dependencies
	cargo check

.PHONY: build
build: ## Build the project in debug mode
	cargo build

.PHONY: build-release
build-release: ## Build the project in release mode
	cargo build --release

.PHONY: run
run: ## Run the project (use ARGS="..." to pass arguments)
	cargo run -- $(ARGS)

.PHONY: test
test: ## Run all tests
	cargo test

.PHONY: test-verbose
test-verbose: ## Run tests with verbose output
	cargo test -- --nocapture

.PHONY: bench
bench: ## Run benchmarks
	cargo bench

# Code quality targets
.PHONY: fmt
fmt: ## Format code using rustfmt
	cargo fmt

.PHONY: fmt-check
fmt-check: ## Check code formatting
	cargo fmt -- --check

.PHONY: clippy
clippy: ## Run clippy linter
	cargo clippy -- -D warnings

.PHONY: clippy-fix
clippy-fix: ## Fix clippy warnings automatically
	cargo clippy --fix

.PHONY: lint
lint: fmt-check clippy ## Run all linting checks

.PHONY: audit
audit: ## Security audit
	cargo audit

# Documentation targets
.PHONY: doc
doc: ## Generate documentation
	cargo doc --no-deps

.PHONY: doc-open
doc-open: ## Generate and open documentation
	cargo doc --no-deps --open

# Clean targets
.PHONY: clean
clean: ## Clean build artifacts
	cargo clean

.PHONY: clean-all
clean-all: clean ## Clean all generated files
	rm -rf out/
	rm -rf node_modules/
	rm -rf dist/

# Installation targets
.PHONY: install-local
install-local: build-release ## Install locally using cargo
	cargo install --path .

.PHONY: uninstall-local
uninstall-local: ## Uninstall local installation
	cargo uninstall $(BINARY_NAME)
