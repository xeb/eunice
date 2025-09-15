# Use Alpine Linux as the small base image
FROM alpine:latest

# Install git, curl, bash, and python3
RUN apk add --no-cache git curl bash python3

# Install uv and add to PATH
RUN curl -LsSf https://astral.sh/uv/install.sh | sh
ENV PATH="/root/.local/bin:$PATH"

# Set working directory
WORKDIR /root

# Copy only the essential files
COPY eunice.py pyproject.toml test.sh README.md ./

# Make test.sh executable
RUN chmod +x test.sh

# Install eunice using uv with explicit path
RUN /root/.local/bin/uv tool install .

# Add uv tools to PATH
ENV PATH="/root/.local/bin:$PATH"

# Run the test suite
CMD ["./test.sh"]