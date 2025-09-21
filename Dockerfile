# Use Alpine Linux as the small base image
FROM alpine:latest

# Install git, curl, bash, python3, sqlite3, and Node.js
RUN apk add --no-cache git curl bash python3 sqlite nodejs npm

# Install uv and add to PATH
RUN curl -LsSf https://astral.sh/uv/install.sh | sh
ENV PATH="/root/.local/bin:$PATH"

# Set working directory
WORKDIR /root

# Copy only the essential files
COPY eunice.py pyproject.toml test-host.sh README.md config.example.json ./

# Copy config.example.json as eunice.json for MCP server configuration
COPY config.example.json /root/eunice.json

# Copy test scripts
COPY test-container-eunice.sh test-container.sh ./

# Make scripts executable
RUN chmod +x test-host.sh test-container-eunice.sh test-container.sh

# Install eunice using uv with explicit path
RUN /root/.local/bin/uv tool install .

# Add uv tools to PATH
ENV PATH="/root/.local/bin:$PATH"

# Run the container test suite
CMD ["./test-container.sh"]