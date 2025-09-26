#!/bin/bash

# Clear the terminal
clear

# Colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
PURPLE='\033[0;35m'
NC='\033[0m' # No Color

echo -e "${PURPLE}🎮 Manifest Development Launcher${NC}"
echo -e "${BLUE}================================${NC}"
echo ""
echo "Choose what to start:"
echo ""
echo -e "${GREEN}1)${NC} Frontend only (Vite dev server on port 5173)"
echo -e "${GREEN}2)${NC} Backend/Tauri only (desktop app with file watching)"  
echo -e "${GREEN}3)${NC} Both (recommended - start frontend first, then Tauri with file watching)"
echo -e "${GREEN}4)${NC} Frontend live + Backend build-once (avoids rebuild loops)"
echo -e "${GREEN}5)${NC} Exit"
echo ""
read -p "Enter your choice (1-5): " -n 1 -r
echo ""

case $REPLY in
    1)
        echo -e "${YELLOW}🌐 Starting frontend only...${NC}"
        echo ""
        ./scripts/start-frontend.sh
        ;;
    2)
        echo -e "${YELLOW}🦀 Starting backend/Tauri only...${NC}"
        echo ""
        ./scripts/start-backend.sh
        ;;
    3)
        echo -e "${YELLOW}🚀 Starting both frontend and backend...${NC}"
        echo ""
        echo -e "${BLUE}Step 1: Starting frontend in new terminal...${NC}"
        # Open frontend in new terminal window
        osascript -e 'tell application "Terminal" to do script "cd \"'$(pwd)'\" && ./scripts/start-frontend.sh"'
        
        echo -e "${BLUE}Step 2: Waiting 3 seconds for frontend to start...${NC}"
        sleep 3
        
        echo -e "${BLUE}Step 3: Starting Tauri backend...${NC}"
        ./scripts/start-backend.sh
        ;;
    4)
        echo -e "${YELLOW}🏗️ Building and running (no file watching)...${NC}"
        echo ""
        echo -e "${BLUE}Step 1: Starting frontend dev server in new terminal...${NC}"
        # Open frontend in new terminal window for live reloading
        osascript -e 'tell application "Terminal" to do script "cd \"'$(pwd)'\" && ./scripts/start-frontend.sh"'
        
        echo -e "${BLUE}Step 2: Waiting 3 seconds for frontend to start...${NC}"
        sleep 3
        
        echo -e "${BLUE}Step 3: Building and running Tauri app (no file watching)...${NC}"
        echo -e "${YELLOW}Frontend will have live reloading, backend will not${NC}"
        echo -e "${YELLOW}Restart this script after making backend changes${NC}"
        echo ""
        
        cd backend
        # Build and run Tauri app once (connects to dev server)
        echo -e "${BLUE}Building and starting Tauri app...${NC}"
        cargo build && cargo run --bin manifest
        ;;
    5)
        echo -e "${YELLOW}👋 Goodbye!${NC}"
        exit 0
        ;;
    *)
        echo -e "${YELLOW}❌ Invalid choice. Please run ./start.sh again.${NC}"
        exit 1
        ;;
esac
