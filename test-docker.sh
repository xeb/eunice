#!/bin/bash

# Build the Docker image
echo "Building eunice Docker image..."
docker build -t eunice-test .

# Check if build was successful
if [ $? -eq 0 ]; then
    echo "Running eunice tests in Docker container..."
    docker run --rm \
        -e OPENAI_API_KEY="${OPENAI_API_KEY}" \
        -e GEMINI_API_KEY="${GEMINI_API_KEY}" \
        -e OLLAMA_HOST="http://host.docker.internal:11434" \
        --add-host=host.docker.internal:host-gateway \
        eunice-test
else
    echo "Docker build failed!"
    exit 1
fi

echo "Docker test completed."