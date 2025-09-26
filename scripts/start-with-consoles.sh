#!/bin/bash

# Development script with separate console windows for frontend and backend
# This gives you the separate console experience you're used to

# Colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
PURPLE='\033[0;35m'
NC='\033[0m' # No Color

clear

echo -e "${PURPLE}🎮 Manifest Development - Separate Consoles${NC}"
echo -e "${BLUE}===========================================${NC}"
echo ""
echo -e "${GREEN}This will open separate Terminal windows for:${NC}"
echo -e "${BLUE}1. Frontend (Vite dev server) - Console output visible${NC}"
echo -e "${BLUE}2. Backend (Rust/Tauri) - Console output visible${NC}"
echo -e "${BLUE}3. Logs monitoring - Real-time log viewing${NC}"
echo ""
echo -e "${YELLOW}Each will have its own Terminal window with full console access${NC}"
echo ""
read -p "Press Enter to continue..."

echo -e "${BLUE}🚀 Starting development environment...${NC}"

# Create logs directory if it doesn't exist
mkdir -p "$(dirname "$0")/../backend/logs"
mkdir -p "$(dirname "$0")/../frontend/logs"

# Start log monitor in background
echo -e "${YELLOW}📊 Starting log monitor...${NC}"
osascript <<EOF
tell application "Terminal"
    set logWindow to do script "cd \"$(pwd)\" && ./scripts/monitor-logs.sh"
    try
        set the bounds of the front window to {100, 100, 800, 400}
    end try
end tell
EOF

sleep 1

# Start frontend in separate terminal
echo -e "${YELLOW}🌐 Starting frontend console...${NC}"
osascript <<EOF
tell application "Terminal"
    set frontendWindow to do script "cd \"$(pwd)\" && echo \"🌐 Manifest Frontend Development Console\" && echo \"=====================================\" && echo \"\" && export VITE_ENABLE_CONSOLE_LOGS=true && export VITE_LOG_TO_FILE=true && cd frontend && echo \"Starting Vite dev server with enhanced logging...\" && npm run dev"
    try
        set the bounds of the front window to {850, 100, 1550, 500}
    end try
end tell
EOF

# Wait for frontend to start
echo -e "${YELLOW}⏳ Waiting for frontend to start...${NC}"
sleep 5

# Check if frontend started
echo -e "${YELLOW}🔍 Checking frontend status...${NC}"
if curl -s "http://localhost:5173" > /dev/null; then
    echo -e "${GREEN}✅ Frontend is running${NC}"
else
    echo -e "${YELLOW}⏳ Frontend still starting...${NC}"
    sleep 3
fi

# Start backend in separate terminal
echo -e "${YELLOW}🦀 Starting backend console...${NC}"
osascript <<EOF
tell application "Terminal"
    set backendWindow to do script "cd \"$(pwd)\" && echo \"🦀 Manifest Backend Development Console\" && echo \"====================================\" && echo \"\" && export RUST_LOG=debug && export RUST_BACKTRACE=1 && export TAURI_ENV_DEBUG=true && cd backend && echo \"Starting Tauri app with enhanced logging...\" && echo \"All Rust logs will appear here\" && echo \"\" && cargo run --bin manifest 2>&1 | tee logs/backend-$(date +%Y%m%d-%H%M%S).log"
    try
        set the bounds of the front window to {850, 520, 1550, 920}
    end try
end tell
EOF

echo ""
echo -e "${GREEN}✅ Development environment started!${NC}"
echo ""
echo -e "${BLUE}You now have separate console windows:${NC}"
echo -e "${GREEN}• Frontend Console:${NC} Vite dev server output"
echo -e "${GREEN}• Backend Console:${NC} Rust/Tauri application logs"  
echo -e "${GREEN}• Log Monitor:${NC} Combined log file monitoring"
echo ""
echo -e "${YELLOW}💡 Tips:${NC}"
echo -e "  • Each console window shows live output"
echo -e "  • Backend logs are also saved to backend/logs/ directory"
echo -e "  • Close any window to stop that service"
echo -e "  • Use Cmd+T to open new tabs in any Terminal window"
echo ""
echo -e "${PURPLE}Happy debugging! 🐛${NC}"
