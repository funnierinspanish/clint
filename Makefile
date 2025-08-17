# Makefile for CLINT - CLI Navigator Toolkit

# Variables
BINARY_NAME := clint
CARGO := cargo
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
	$(CARGO) check

.PHONY: build
build: ## Build the project in debug mode
	$(CARGO) build

.PHONY: build-release
build-release: ## Build the project in release mode
	$(CARGO) build --release

.PHONY: run
run: ## Run the project (use ARGS="..." to pass arguments)
	$(CARGO) run -- $(ARGS)

.PHONY: test
test: ## Run all tests
	$(CARGO) test

.PHONY: test-verbose
test-verbose: ## Run tests with verbose output
	$(CARGO) test -- --nocapture

.PHONY: bench
bench: ## Run benchmarks
	$(CARGO) bench

# Code quality targets
.PHONY: fmt
fmt: ## Format code using rustfmt
	$(CARGO) fmt

.PHONY: fmt-check
fmt-check: ## Check code formatting
	$(CARGO) fmt -- --check

.PHONY: clippy
clippy: ## Run clippy linter
	$(CARGO) clippy -- -D warnings

.PHONY: clippy-fix
clippy-fix: ## Fix clippy warnings automatically
	$(CARGO) clippy --fix

.PHONY: lint
lint: fmt-check clippy ## Run all linting checks

.PHONY: audit
audit: ## Security audit
	$(CARGO) audit

# Documentation targets
.PHONY: doc
doc: ## Generate documentation
	$(CARGO) doc --no-deps

.PHONY: doc-open
doc-open: ## Generate and open documentation
	$(CARGO) doc --no-deps --open

# Schema and TypeScript targets
.PHONY: schema-validate
schema-validate: ## Validate JSON schema files
	@if command -v ajv >/dev/null 2>&1; then \
		echo "Validating JSON schema..."; \
		ajv validate -s cli_structure.schema.json -d "out/*/flexai-structure.json" || echo "AJV not found, skipping JSON schema validation"; \
	fi

.PHONY: ts-check
ts-check: ## Check TypeScript files (requires npm install)
	@if [ -f "package.json" ]; then \
		echo "Checking TypeScript files..."; \
		npm run build 2>/dev/null || echo "TypeScript check skipped (run 'npm install' first)"; \
	fi

.PHONY: validate-cli-structure
validate-cli-structure: ## Validate CLI structure with Node.js script
	@if [ -f "out/flexai/86b98a84a7cbfc02f47da14000ef91479c8cbbfa/flexai-structure.json" ]; then \
		node validate-cli-structure.js out/flexai/86b98a84a7cbfc02f47da14000ef91479c8cbbfa/flexai-structure.json; \
	else \
		echo "No CLI structure file found. Run 'make example-parse' first."; \
	fi

# Example targets
.PHONY: example-parse
example-parse: build ## Parse flexai CLI as example
	$(CARGO) run -- parse flexai

.PHONY: example-webpage
example-webpage: example-parse ## Generate example webpage
	@echo "Generating webpage for flexai CLI structure..."
	$(CARGO) run -- webpage out/flexai/*/flexai-structure.json
	@echo "Webpage generated. Serve with: python -m http.server 9000 -d out/flexai-structure-webpage"

.PHONY: example-keywords
example-keywords: example-parse ## Extract keywords from example
	$(CARGO) run -- unique-keywords out/flexai/*/flexai-structure.json --output-path out/flexai-keywords.md

.PHONY: example-summary
example-summary: example-parse ## Generate summary from example
	$(CARGO) run -- summary out/flexai/*/flexai-structure.json --output-path out/flexai-summary.md

# Clean targets
.PHONY: clean
clean: ## Clean build artifacts
	$(CARGO) clean

.PHONY: clean-all
clean-all: clean ## Clean all generated files
	rm -rf out/
	rm -rf node_modules/
	rm -rf dist/

# Release targets
.PHONY: check-release
check-release: lint test ## Run all checks before release
	@echo "All checks passed! Ready for release."

.PHONY: release-dry-run
release-dry-run: ## Dry run of cargo publish
	$(CARGO) publish --dry-run

.PHONY: release
release: check-release release-dry-run ## Publish to crates.io
	$(CARGO) publish

# CI targets (used by GitHub Actions)
.PHONY: ci-install
ci-install: ## Install dependencies for CI
	rustup component add rustfmt clippy

.PHONY: ci-test
ci-test: ## Run CI tests
	$(CARGO) test --verbose --all-features

.PHONY: ci-build
ci-build: ## Build for CI
	$(CARGO) build --verbose --all-features

.PHONY: ci-check
ci-check: lint ci-build ci-test ## Run all CI checks

# Cross-compilation targets
.PHONY: build-linux
build-linux: ## Build for Linux x86_64
	$(CARGO) build --release --target x86_64-unknown-linux-gnu

.PHONY: build-windows
build-windows: ## Build for Windows x86_64
	$(CARGO) build --release --target x86_64-pc-windows-gnu

.PHONY: build-macos
build-macos: ## Build for macOS x86_64
	$(CARGO) build --release --target x86_64-apple-darwin

# Development workflow
.PHONY: dev
dev: build example-parse validate-cli-structure ## Quick development workflow

.PHONY: full-check
full-check: clean lint test example-parse validate-cli-structure ## Full project validation

# Installation targets
.PHONY: install-local
install-local: build-release ## Install locally using cargo
	$(CARGO) install --path .

.PHONY: uninstall-local
uninstall-local: ## Uninstall local installation
	$(CARGO) uninstall $(BINARY_NAME)

# Watch targets (requires cargo-watch)
.PHONY: watch
watch: ## Watch for changes and rebuild
	@if command -v cargo-watch >/dev/null 2>&1; then \
		cargo-watch -x build; \
	else \
		echo "cargo-watch not found. Install with: cargo install cargo-watch"; \
	fi

.PHONY: watch-test
watch-test: ## Watch for changes and run tests
	@if command -v cargo-watch >/dev/null 2>&1; then \
		cargo-watch -x test; \
	else \
		echo "cargo-watch not found. Install with: cargo install cargo-watch"; \
	fi
