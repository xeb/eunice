.PHONY: help docker docker-force test-docker test-host test-container test-fs test-tools test reinstall clean build-and-test

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
	docker run --rm --add-host=host.docker.internal:host-gateway xebxeb/eunice

test-host: ## Run host-based tests
	@echo "Running host tests..."
	./tests/host.sh

test-fs: ## Run filesystem tests
	@echo "Running filesystem tests..."
	./tests/test_filesystem.sh

test-tools: ## Run MCP tool routing tests
	@echo "Running MCP tool routing tests..."
	./tests/tools.sh

test: test-host test-fs ## Run all tests (host.sh already includes tools.sh)

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

clean: ## Clean up Docker images and containers
	@echo "Cleaning up Docker resources..."
	-docker rmi xebxeb/eunice
	docker system prune -f

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
