#!/bin/bash
set -euo pipefail

# Script to generate the Langfuse client from OpenAPI specification

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
BASE_CLIENT_DIR="$PROJECT_ROOT/langfuse-client-base"
OPENAPI_URL="https://cloud.langfuse.com/generated/api/openapi.yml"
OPENAPI_FILE="$SCRIPT_DIR/openapi.yml"

echo "🔧 Generating Langfuse client from OpenAPI specification..."

# Check if openapi-generator-cli is installed
if ! command -v openapi-generator-cli &> /dev/null; then
    echo "❌ openapi-generator-cli is not installed."
    echo "Installing via npm..."
    npm install -g @openapitools/openapi-generator-cli
fi

# Download the latest OpenAPI specification
echo "📥 Downloading OpenAPI specification..."
curl -o "$OPENAPI_FILE" "$OPENAPI_URL"

# Generate the client
echo "🏗️ Generating Rust client..."
openapi-generator-cli generate \
    -i "$OPENAPI_FILE" \
    -g rust \
    -o "$BASE_CLIENT_DIR" \
    --additional-properties=packageName=langfuse-client-base \
    --additional-properties=packageVersion=0.1.0 \
    --additional-properties=library=reqwest \
    --additional-properties=supportAsync=true \
    --additional-properties=preferUnsignedInt=false \
    --additional-properties=useSingleRequestParameter=false

# Clean up generated files we don't need
echo "🧹 Cleaning up generated files..."
rm -f "$BASE_CLIENT_DIR/.travis.yml"
rm -f "$BASE_CLIENT_DIR/.gitignore"
rm -f "$BASE_CLIENT_DIR/git_push.sh"
rm -rf "$BASE_CLIENT_DIR/.openapi-generator"

# Preserve our custom Cargo.toml
echo "📝 Preserving custom Cargo.toml..."
if [ -f "$BASE_CLIENT_DIR/Cargo.toml.bak" ]; then
    mv "$BASE_CLIENT_DIR/Cargo.toml.bak" "$BASE_CLIENT_DIR/Cargo.toml"
fi

echo "✅ Client generation complete!"
echo ""
echo "Note: The generated client is in $BASE_CLIENT_DIR"
echo "      You may need to manually adjust some generated code for better ergonomics."