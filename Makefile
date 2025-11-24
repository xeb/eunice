.PHONY: help build release install interact clean test list-models bump-version publish

# Colors
CYAN := \033[36m
GREEN := \033[32m
YELLOW := \033[33m
RESET := \033[0m

help: ## Show this help menu
	@echo ""
	@echo "$(CYAN)eunice$(RESET) - Agentic CLI runner in Rust"
	@echo ""
	@echo "$(YELLOW)Usage:$(RESET)"
	@echo "  make $(GREEN)<target>$(RESET)"
	@echo ""
	@echo "$(YELLOW)Targets:$(RESET)"
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "  $(GREEN)%-15s$(RESET) %s\n", $$1, $$2}'
	@echo ""

build: ## Build in debug mode
	cargo build

release: ## Build in release mode (optimized)
	cargo build --release

install: ## Install globally via cargo
	cargo install --path .

interact: release ## Run in interactive mode
	./target/release/eunice --interact

run: release ## Run with arguments (use ARGS="...")
	./target/release/eunice $(ARGS)

clean: ## Clean build artifacts
	cargo clean

test: ## Run tests
	cargo test

list-models: release ## List available models
	./target/release/eunice --list-models

bump-version: ## Bump patch version (0.1.0 -> 0.1.1)
	@current=$$(grep '^version' Cargo.toml | head -1 | sed 's/.*"\(.*\)"/\1/'); \
	major=$$(echo $$current | cut -d. -f1); \
	minor=$$(echo $$current | cut -d. -f2); \
	patch=$$(echo $$current | cut -d. -f3); \
	new_patch=$$((patch + 1)); \
	new_version="$$major.$$minor.$$new_patch"; \
	sed -i "s/^version = \"$$current\"/version = \"$$new_version\"/" Cargo.toml; \
	echo "$(GREEN)Version bumped: $$current -> $$new_version$(RESET)"

publish: bump-version ## Bump version and publish to crates.io
	cargo publish --allow-dirty
	@echo "$(GREEN)Published to crates.io!$(RESET)"
