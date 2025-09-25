#!/bin/bash

# Clear the terminal  
clear

# Shared environment variables (same as frontend)
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
RED='\033[0;31m'
NC='\033[0m' # No Color

echo -e "${BLUE}🦀 Starting Manifest Tauri Development App...${NC}"
echo -e "${YELLOW}Connecting to frontend: http://localhost:${VITE_DEV_PORT}${NC}"
echo -e "${YELLOW}Environment: ${NODE_ENV}${NC}"
echo ""

# Navigate to backend directory
cd "$(dirname "$0")/backend"

# Check if frontend is running
echo -e "${YELLOW}🔍 Checking if frontend server is running on port ${VITE_DEV_PORT}...${NC}"
if ! curl -s "http://localhost:${VITE_DEV_PORT}" > /dev/null; then
    echo -e "${RED}❌ Frontend server is not running on port ${VITE_DEV_PORT}${NC}"
    echo -e "${YELLOW}💡 Please start the frontend first with: ./start-frontend.sh${NC}"
    echo ""
    read -p "Do you want to continue anyway? (y/N): " -n 1 -r
    echo ""
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        echo -e "${YELLOW}Exiting...${NC}"
        exit 1
    fi
else
    echo -e "${GREEN}✅ Frontend server detected${NC}"
fi

echo ""

# Start Tauri development app
echo -e "${GREEN}🚀 Starting Tauri development app...${NC}"
cargo tauri dev
