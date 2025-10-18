.PHONY: help docker docker-force test-docker test-host test-container test-fs test-tools test-models test reinstall clean build-and-test

.DEFAULT_GOAL := help

help: ## Show this help menu
	@echo "Eunice Development Commands"
	@echo "==========================="
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-20s\033[0m %s\n", $$1, $$2}'

docker: ## Build Docker image with cache
	@echo "Building xebxeb/eunice image..."
	docker build -t xebxeb/eunice .

docker-force: ## Build Docker image without cache (force rebuild)
	@echo "Building xebxeb/eunice image (no cache)..."
	docker build --no-cache -t xebxeb/eunice .

test-docker: ## Run Docker container tests
	@echo "Running container tests..."
	docker run --rm --add-host=host.docker.internal:host-gateway \
		-e OPENAI_API_KEY="$(OPENAI_API_KEY)" \
		-e GEMINI_API_KEY="$(GEMINI_API_KEY)" \
		-e ANTHROPIC_API_KEY="$(ANTHROPIC_API_KEY)" \
		xebxeb/eunice

test-host: ## Run host-based tests
	@echo "Running host tests..."
	./tests/host.sh

test-events: ## Run event streaming tests
	@echo "Running filesystem tests..."
	./tests/events.sh

test-fs: ## Run filesystem tests
	@echo "Running filesystem tests..."
	./tests/test_filesystem.sh

test-tools: ## Run MCP tool routing tests
	@echo "Running MCP tool routing tests..."
	./tests/tools.sh

test-models: ## Run comprehensive model tests for all providers
	@echo "Running comprehensive model tests..."
	./tests/all_models.sh

test: test-host test-fs test-models test-events ## Run all tests (host.sh already includes tools.sh)

reinstall: ## Reinstall eunice using the reinstall script
	@echo "Reinstalling eunice..."
	./scripts/reinstall.sh

build-and-test: docker-force test-docker ## Build Docker image (force) and run tests
	@echo ""
	@echo "=== Build and Test Results ==="
	@echo "🎉 Docker image built and tested successfully!"
	@echo "✅ xebxeb/eunice image is ready for use"

publish: ## Push Docker image to registry
	@echo "Publishing xebxeb/eunice to Docker registry..."
	docker push xebxeb/eunice

clean: ## Clean up Docker images, containers, and test files
	@echo "Cleaning up Docker resources..."
	-docker rmi xebxeb/eunice
	#docker system prune -f
	@echo "Cleaning up test artifacts..."
	-rm -rf test_data
	-rm -f test_prompt.txt
	-rm -f test_colored_output.py
	-rm -f test_mcp_config.json
	-rm -f test_underscore_server.json

install: ## Install eunice globally using uv
	@echo "Installing eunice globally..."
	uv tool install .

uninstall: ## Uninstall eunice globally
	@echo "Uninstalling eunice..."
	uv tool uninstall eunice

dev: ## Run eunice in development mode (usage: make dev PROMPT="your prompt here")
	@if [ -z "$(PROMPT)" ]; then \
		echo "Usage: make dev PROMPT=\"your prompt here\""; \
		echo "Example: make dev PROMPT=\"What files are in this directory?\""; \
		exit 1; \
	fi
	@echo "Running eunice in development mode..."
	uv run eunice.py "$(PROMPT)"

line-numbers: ## Update line count in README.md
	@LINES=$$(wc -l < eunice.py); \
	PERCENT=$$(echo "scale=1; $$LINES * 100 / 2000" | bc); \
	REMAINING=$$(echo "scale=1; 100 - $$PERCENT" | bc); \
	sed -i "s/\*\*Current Status\*\*: \`eunice.py\` is \*\*[0-9]*\/2,000 lines\*\* ([0-9.]*% used, \*\*[0-9.]*% remaining\*\*)/\*\*Current Status\*\*: \`eunice.py\` is \*\*$$LINES\/2,000 lines\*\* ($$PERCENT% used, \*\*$$REMAINING% remaining\*\*)/" README.md; \
	echo "Updated README.md: eunice.py is $$LINES/2,000 lines ($$PERCENT% used, $$REMAINING% remaining)"
