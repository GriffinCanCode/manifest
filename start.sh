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
echo -e "${GREEN}2)${NC} Backend/Tauri only (desktop app)"  
echo -e "${GREEN}3)${NC} Both (recommended - start frontend first, then Tauri)"
echo -e "${GREEN}4)${NC} Exit"
echo ""
read -p "Enter your choice (1-4): " -n 1 -r
echo ""

case $REPLY in
    1)
        echo -e "${YELLOW}🌐 Starting frontend only...${NC}"
        echo ""
        ./start-frontend.sh
        ;;
    2)
        echo -e "${YELLOW}🦀 Starting backend/Tauri only...${NC}"
        echo ""
        ./start-backend.sh
        ;;
    3)
        echo -e "${YELLOW}🚀 Starting both frontend and backend...${NC}"
        echo ""
        echo -e "${BLUE}Step 1: Starting frontend in new terminal...${NC}"
        # Open frontend in new terminal window
        osascript -e 'tell application "Terminal" to do script "cd \"'$(pwd)'\" && ./start-frontend.sh"'
        
        echo -e "${BLUE}Step 2: Waiting 3 seconds for frontend to start...${NC}"
        sleep 3
        
        echo -e "${BLUE}Step 3: Starting Tauri backend...${NC}"
        ./start-backend.sh
        ;;
    4)
        echo -e "${YELLOW}👋 Goodbye!${NC}"
        exit 0
        ;;
    *)
        echo -e "${YELLOW}❌ Invalid choice. Please run ./start.sh again.${NC}"
        exit 1
        ;;
esac
