#!/bin/bash

# Log viewer script for Manifest development
# Provides easy access to recent logs for debugging

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
PURPLE='\033[0;35m'
NC='\033[0m'

clear

echo -e "${PURPLE}📋 Manifest Log Viewer${NC}"
echo -e "${BLUE}=====================${NC}"
echo ""

# Get current date
TODAY=$(date +%Y-%m-%d)
YESTERDAY=$(date -d "yesterday" +%Y-%m-%d 2>/dev/null || date -v-1d +%Y-%m-%d 2>/dev/null)

# Check for logs in various locations
BACKEND_LOGS_DIR="$(dirname "$0")/../backend/logs"
TAURI_LOGS_DIR="$HOME/Library/Application Support/com.griffincancode.manifest/logs"
FRONTEND_LOGS_DIR="$(dirname "$0")/../frontend/logs"

echo -e "${YELLOW}🔍 Searching for logs...${NC}"
echo ""

# Function to display logs if they exist
show_logs() {
    local log_path="$1"
    local log_name="$2"
    
    if [ -f "$log_path" ]; then
        local size=$(du -h "$log_path" | cut -f1)
        local lines=$(wc -l < "$log_path")
        echo -e "${GREEN}✅ Found: $log_name${NC} (${size}, ${lines} lines)"
        return 0
    else
        return 1
    fi
}

# Check all possible log locations
found_any=false

echo -e "${BLUE}Backend Logs:${NC}"
if ls "$BACKEND_LOGS_DIR"/*.log >/dev/null 2>&1; then
    for log in "$BACKEND_LOGS_DIR"/*.log; do
        if show_logs "$log" "$(basename "$log")"; then
            found_any=true
        fi
    done
else
    echo -e "${YELLOW}  No backend logs found in $BACKEND_LOGS_DIR${NC}"
fi

echo ""
echo -e "${BLUE}Frontend Logs:${NC}"
if ls "$FRONTEND_LOGS_DIR"/*.log >/dev/null 2>&1 || ls "$TAURI_LOGS_DIR"/*.log >/dev/null 2>&1; then
    for log in "$FRONTEND_LOGS_DIR"/*.log "$TAURI_LOGS_DIR"/*.log; do
        if [ -f "$log" ]; then
            if show_logs "$log" "$(basename "$log")"; then
                found_any=true
            fi
        fi
    done
else
    echo -e "${YELLOW}  No frontend logs found${NC}"
fi

echo ""

if [ "$found_any" = false ]; then
    echo -e "${RED}❌ No log files found!${NC}"
    echo ""
    echo -e "${YELLOW}💡 Possible reasons:${NC}"
    echo -e "  • Application hasn't been started yet"
    echo -e "  • Logging system isn't working properly"
    echo -e "  • Logs are being written elsewhere"
    echo ""
    echo -e "${BLUE}🔧 To fix this:${NC}"
    echo -e "  1. Start the app with: ${GREEN}./scripts/start-with-consoles.sh${NC}"
    echo -e "  2. Use the app for a few seconds to generate logs"
    echo -e "  3. Run this script again"
    echo ""
    echo -e "${YELLOW}📊 You can also check browser console logs:${NC}"
    echo -e "  • Right-click in the app → Inspect Element → Console"
    echo -e "  • Or use Cmd+Option+I (Chrome/Tauri dev tools)"
    exit 1
fi

echo -e "${GREEN}📖 What would you like to do?${NC}"
echo ""
echo "1) View most recent backend logs (last 50 lines)"
echo "2) View most recent frontend logs (last 50 lines)" 
echo "3) View all logs from today"
echo "4) Search logs for specific text"
echo "5) Follow logs in real-time (tail -f)"
echo "6) View logs from around 7:35 PM (when freeze occurred)"
echo "7) Export logs to desktop for sharing"
echo "8) Clear old logs"
echo "q) Quit"
echo ""

read -p "Choose an option (1-8, q): " choice

case $choice in
    1)
        echo -e "${BLUE}📄 Most recent backend logs:${NC}"
        echo ""
        find "$BACKEND_LOGS_DIR" -name "*.log" -exec tail -50 {} +
        ;;
    2)
        echo -e "${BLUE}📄 Most recent frontend logs:${NC}"
        echo ""
        find "$FRONTEND_LOGS_DIR" "$TAURI_LOGS_DIR" -name "*.log" 2>/dev/null -exec tail -50 {} +
        ;;
    3)
        echo -e "${BLUE}📄 All logs from today:${NC}"
        echo ""
        find "$BACKEND_LOGS_DIR" "$FRONTEND_LOGS_DIR" "$TAURI_LOGS_DIR" -name "*$TODAY*.log" 2>/dev/null -exec cat {} +
        ;;
    4)
        read -p "Enter search text: " search_text
        echo -e "${BLUE}🔍 Searching for '$search_text':${NC}"
        echo ""
        find "$BACKEND_LOGS_DIR" "$FRONTEND_LOGS_DIR" "$TAURI_LOGS_DIR" -name "*.log" 2>/dev/null -exec grep -l "$search_text" {} + | while read -r file; do
            echo -e "${GREEN}Found in: $(basename "$file")${NC}"
            grep --color=always -n "$search_text" "$file"
            echo ""
        done
        ;;
    5)
        echo -e "${BLUE}📊 Following logs in real-time (Ctrl+C to stop):${NC}"
        echo ""
        find "$BACKEND_LOGS_DIR" "$FRONTEND_LOGS_DIR" "$TAURI_LOGS_DIR" -name "*.log" 2>/dev/null -exec tail -f {} +
        ;;
    6)
        echo -e "${BLUE}🕐 Logs from around 7:35 PM:${NC}"
        echo ""
        # Search for logs around 7:35 PM (19:35)
        find "$BACKEND_LOGS_DIR" "$FRONTEND_LOGS_DIR" "$TAURI_LOGS_DIR" -name "*.log" 2>/dev/null -exec grep -A 10 -B 10 "19:3[0-9]:\|7:3[0-9] PM" {} + || echo "No logs found around 7:35 PM"
        ;;
    7)
        export_dir="$HOME/Desktop/manifest-logs-$(date +%Y%m%d-%H%M%S)"
        mkdir -p "$export_dir"
        echo -e "${BLUE}📦 Exporting logs to: $export_dir${NC}"
        find "$BACKEND_LOGS_DIR" "$FRONTEND_LOGS_DIR" "$TAURI_LOGS_DIR" -name "*.log" 2>/dev/null -exec cp {} "$export_dir/" \;
        echo -e "${GREEN}✅ Logs exported to Desktop${NC}"
        ;;
    8)
        echo -e "${YELLOW}⚠️  This will delete logs older than 7 days. Continue? (y/N):${NC}"
        read -n 1 confirm
        echo ""
        if [[ $confirm =~ ^[Yy]$ ]]; then
            find "$BACKEND_LOGS_DIR" "$FRONTEND_LOGS_DIR" "$TAURI_LOGS_DIR" -name "*.log" -mtime +7 -delete 2>/dev/null
            echo -e "${GREEN}✅ Old logs cleaned up${NC}"
        else
            echo -e "${YELLOW}Cancelled${NC}"
        fi
        ;;
    q)
        echo -e "${YELLOW}👋 Goodbye!${NC}"
        exit 0
        ;;
    *)
        echo -e "${RED}❌ Invalid choice${NC}"
        exit 1
        ;;
esac
