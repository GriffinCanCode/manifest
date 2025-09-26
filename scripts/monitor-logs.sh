#!/bin/bash

# Dedicated log monitor script
# This avoids complex shell escaping in AppleScript

# Colors
YELLOW='\033[1;33m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m'

echo -e "${BLUE}📊 Log Monitor - Press Ctrl+C to stop${NC}"
echo -e "${YELLOW}Watching for logs...${NC}"
echo ""

cd "$(dirname "$0")/.."

# Create log directories if they don't exist
mkdir -p backend/logs frontend/logs

echo "Waiting for log files to be created..."
echo ""

# Function to check if any log files exist
check_logs() {
    find backend/logs frontend/logs -name "*.log" -type f 2>/dev/null | head -1
}

# Wait for logs to appear
while true; do
    log_file=$(check_logs)
    if [ -n "$log_file" ]; then
        echo -e "${GREEN}✅ Found log files! Starting real-time monitoring...${NC}"
        echo ""
        break
    else
        echo "No logs yet - checking again in 3 seconds..."
        sleep 3
    fi
done

# Start tailing all log files
if command -v multitail >/dev/null 2>&1; then
    # Use multitail if available (better for multiple files)
    echo "Using multitail for enhanced log monitoring..."
    find backend/logs frontend/logs -name "*.log" -type f 2>/dev/null | xargs multitail
else
    # Fallback to regular tail with find
    echo "Monitoring all log files with tail -f..."
    while true; do
        # Get current log files
        log_files=$(find backend/logs frontend/logs -name "*.log" -type f 2>/dev/null)
        
        if [ -n "$log_files" ]; then
            # Use tail with explicit file list to avoid glob issues
            echo "$log_files" | xargs tail -f
            break
        else
            echo "Log files disappeared, waiting..."
            sleep 2
        fi
    done
fi
