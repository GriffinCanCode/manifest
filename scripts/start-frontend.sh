#!/bin/bash

# Clear the terminal
clear

# Shared environment variables
export VITE_APP_NAME="Manifest"
export VITE_APP_VERSION="0.1.0"
export VITE_DEV_PORT="5173"
export TAURI_ENV_DEBUG="true"
export RUST_LOG="debug"
export NODE_ENV="development"

# Colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${BLUE}🚀 Starting Manifest Frontend Development Server...${NC}"
echo -e "${YELLOW}Port: ${VITE_DEV_PORT}${NC}"
echo -e "${YELLOW}Environment: ${NODE_ENV}${NC}"
echo ""

# Navigate to frontend directory
cd "$(dirname "$0")/../frontend"

# Install dependencies if node_modules doesn't exist
if [ ! -d "node_modules" ]; then
    echo -e "${YELLOW}📦 Installing frontend dependencies...${NC}"
    npm install
    echo ""
fi

# Start the development server
echo -e "${GREEN}🌐 Frontend server starting on http://localhost:${VITE_DEV_PORT}${NC}"
npm run dev
