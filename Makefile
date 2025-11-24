.PHONY: help build release install interact clean test

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
