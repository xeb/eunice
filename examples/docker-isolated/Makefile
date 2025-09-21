.PHONY: help build build-force run run-restricted test

.DEFAULT_GOAL := help

help:
	@echo "Eunice Isolated Make Commands"
	@echo "----------------------"
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-30s\033[0m %s\n", $$1, $$2}'

run-full: ## Run the Daydream container with access to host networking
	docker run --rm -it \
		--network host \
		-v $(PWD)/workspace:/root/workspace \
		xebxeb/eunice /bin/bash

run-restricted: ## Run container with restricted network (only host localhost:11434)
	docker run --rm -it \
		--network none \
		--add-host host.docker.internal:host-gateway \
		-v $(PWD)/workspace:/root/workspace \
		xebxeb/eunice sh -c 'echo "Access host service via host.docker.internal:11434" && /bin/bash'

