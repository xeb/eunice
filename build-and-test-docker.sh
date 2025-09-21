#!/bin/bash

echo "=== Building and Testing xebxeb/eunice Docker Image ==="

# Build the Docker image
echo "Building xebxeb/eunice image..."
docker build -t xebxeb/eunice .
BUILD_EXIT_CODE=$?

if [ $BUILD_EXIT_CODE -eq 0 ]; then
    echo "✅ Docker image built successfully"
else
    echo "❌ Docker image build failed with exit code $BUILD_EXIT_CODE"
    exit $BUILD_EXIT_CODE
fi

# Run the container tests
echo ""
echo "Running container tests..."
docker run --rm --network host xebxeb/eunice
TEST_EXIT_CODE=$?

echo ""
echo "=== Test Results ==="
if [ $TEST_EXIT_CODE -eq 0 ]; then
    echo "🎉 All Docker container tests passed!"
    echo "✅ xebxeb/eunice image is ready for use"
else
    echo "💥 Docker container tests failed with exit code $TEST_EXIT_CODE"
    exit $TEST_EXIT_CODE
fi