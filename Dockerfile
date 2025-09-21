# Use Alpine Linux as the small base image
FROM alpine:latest

# Install dependencies, uv, and setup environment in one layer
RUN apk add --no-cache git curl bash python3 sqlite nodejs npm && \
    curl -LsSf https://astral.sh/uv/install.sh | sh

# Set working directory and PATH
WORKDIR /root
ENV PATH="/root/.local/bin:$PATH"

# Copy all files and setup permissions in one layer
COPY eunice.py pyproject.toml README.md config.example.json ./
COPY tests/ tests/
COPY config.example.json /root/eunice.json
RUN chmod +x tests/host.sh tests/container-eunice.sh tests/container.sh

# Install eunice using uv
RUN /root/.local/bin/uv tool install .

# Run the container test suite
CMD ["./tests/container.sh"]
