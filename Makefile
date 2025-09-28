# Manifest Game Engine - Comprehensive Build System
# =============================================================================
# 
# A modern Makefile for the Manifest grand strategy game engine.
# Orchestrates builds across Rust backend (with Zig FFI), TypeScript frontend,
# and provides comprehensive development, testing, and deployment workflows.
#
# Architecture:
# - Rust Backend (Tauri) with Zig SIMD optimizations  
# - TypeScript/React Frontend (Vite)
# - Lua scripting (runtime loaded)
# - Cross-platform desktop application
#
# Usage:
#   make help           - Show all available targets
#   make dev            - Start development environment  
#   make build          - Build for production
#   make clean          - Clean all build artifacts
#   make test           - Run all tests
#
# =============================================================================

# -----------------------------------------------------------------------------
# Configuration & Environment
# -----------------------------------------------------------------------------

# Project metadata
PROJECT_NAME := manifest
VERSION := 0.1.0
RUST_VERSION := 1.77.2

# Directories
ROOT_DIR := $(shell pwd)
BACKEND_DIR := $(ROOT_DIR)/backend
FRONTEND_DIR := $(ROOT_DIR)/frontend  
ZIG_DIR := $(BACKEND_DIR)/zig-modules
SCRIPTS_DIR := $(ROOT_DIR)/scripts
DOCS_DIR := $(ROOT_DIR)/docs

# Build directories
BUILD_DIR := $(ROOT_DIR)/build
DIST_DIR := $(ROOT_DIR)/dist
CACHE_DIR := $(ROOT_DIR)/.cache

# Environment detection
OS := $(shell uname -s)
ARCH := $(shell uname -m)
ifeq ($(OS),Darwin)
    PLATFORM := macos
    OPEN_CMD := open
else ifeq ($(OS),Linux)
    PLATFORM := linux  
    OPEN_CMD := xdg-open
else
    PLATFORM := windows
    OPEN_CMD := start
endif

# Development configuration
DEV_PORT := 5173
RUST_LOG := debug
NODE_ENV := development

# Build flags
CARGO_FLAGS := --release
CARGO_DEV_FLAGS := 
VITE_FLAGS := --mode production
VITE_DEV_FLAGS := --mode development

# Colors for output
GREEN := \033[0;32m
BLUE := \033[0;34m
YELLOW := \033[1;33m
PURPLE := \033[0;35m
RED := \033[0;31m
CYAN := \033[0;36m
NC := \033[0m

# -----------------------------------------------------------------------------
# Help System
# -----------------------------------------------------------------------------

.PHONY: help
help: ## Show this help message
	@echo "$(PURPLE)🎮 Manifest Game Engine Build System$(NC)"
	@echo "$(BLUE)======================================$(NC)"
	@echo ""
	@echo "$(YELLOW)Development:$(NC)"
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | grep -E "(dev|start|watch|serve)" | awk 'BEGIN {FS = ":.*?## "}; {printf "  $(GREEN)%-15s$(NC) %s\n", $$1, $$2}'
	@echo ""
	@echo "$(YELLOW)Building:$(NC)"
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | grep -E "(build|compile|install)" | awk 'BEGIN {FS = ":.*?## "}; {printf "  $(GREEN)%-15s$(NC) %s\n", $$1, $$2}'
	@echo ""
	@echo "$(YELLOW)Testing:$(NC)"
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | grep -E "(test|check|lint|bench)" | awk 'BEGIN {FS = ":.*?## "}; {printf "  $(GREEN)%-15s$(NC) %s\n", $$1, $$2}'
	@echo ""
	@echo "$(YELLOW)Maintenance:$(NC)"
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | grep -E "(clean|setup|update|format)" | awk 'BEGIN {FS = ":.*?## "}; {printf "  $(GREEN)%-15s$(NC) %s\n", $$1, $$2}'
	@echo ""
	@echo "$(YELLOW)Deployment:$(NC)"
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | grep -E "(release|package|deploy|dist)" | awk 'BEGIN {FS = ":.*?## "}; {printf "  $(GREEN)%-15s$(NC) %s\n", $$1, $$2}'

# -----------------------------------------------------------------------------
# Environment Setup & Dependencies
# -----------------------------------------------------------------------------

.PHONY: setup
setup: setup-rust setup-node setup-zig ## Setup complete development environment
	@echo "$(GREEN)✅ Development environment setup complete!$(NC)"

.PHONY: setup-rust
setup-rust: ## Install/verify Rust toolchain
	@echo "$(BLUE)🦀 Setting up Rust environment...$(NC)"
	@if ! command -v rustc >/dev/null 2>&1; then \
		echo "$(YELLOW)Installing Rust...$(NC)"; \
		curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y; \
		source ~/.cargo/env; \
	fi
	@rustc --version | head -1
	@echo "$(GREEN)✅ Rust environment ready$(NC)"

.PHONY: setup-node
setup-node: ## Install frontend dependencies
	@echo "$(BLUE)📦 Setting up Node.js dependencies...$(NC)"
	@cd $(FRONTEND_DIR) && \
		if [ ! -d "node_modules" ]; then \
			echo "$(YELLOW)Installing frontend dependencies...$(NC)"; \
			npm install; \
		else \
			echo "$(GREEN)Frontend dependencies already installed$(NC)"; \
		fi

.PHONY: setup-zig
setup-zig: ## Verify Zig toolchain (optional but recommended)
	@echo "$(BLUE)⚡ Checking Zig toolchain...$(NC)"
	@if command -v zig >/dev/null 2>&1; then \
		echo "$(GREEN)✅ Zig $(shell zig version) detected - SIMD optimizations enabled$(NC)"; \
	else \
		echo "$(YELLOW)⚠️  Zig not found - using fallback implementations$(NC)"; \
		echo "$(CYAN)💡 Install Zig for better performance: https://ziglang.org/$(NC)"; \
	fi

# -----------------------------------------------------------------------------
# Development Workflow
# -----------------------------------------------------------------------------

.PHONY: dev
dev: setup-node ## Start complete development environment
	@echo "$(PURPLE)🚀 Starting Manifest development environment...$(NC)"
	@echo "$(BLUE)This will open two terminals:$(NC)"
	@echo "  $(GREEN)1.$(NC) Frontend dev server (hot reload)"
	@echo "  $(GREEN)2.$(NC) Backend Tauri app (connects to frontend)"
	@echo ""
	@if [ "$(PLATFORM)" = "macos" ]; then \
		echo "$(YELLOW)Starting frontend in new terminal...$(NC)"; \
		osascript -e 'tell application "Terminal" to do script "cd \"$(ROOT_DIR)\" && make dev-frontend"'; \
		sleep 3; \
		echo "$(YELLOW)Starting backend...$(NC)"; \
		$(MAKE) dev-backend; \
	else \
		echo "$(RED)Multi-terminal startup only supported on macOS currently.$(NC)"; \
		echo "$(CYAN)Please run these commands in separate terminals:$(NC)"; \
		echo "  $(GREEN)Terminal 1:$(NC) make dev-frontend"; \
		echo "  $(GREEN)Terminal 2:$(NC) make dev-backend"; \
	fi

.PHONY: dev-frontend
dev-frontend: setup-node ## Start frontend development server only
	@echo "$(BLUE)🌐 Starting frontend development server...$(NC)"
	@cd $(FRONTEND_DIR) && \
		export VITE_APP_NAME="$(PROJECT_NAME)" && \
		export VITE_APP_VERSION="$(VERSION)" && \
		export VITE_DEV_PORT="$(DEV_PORT)" && \
		export NODE_ENV="$(NODE_ENV)" && \
		echo "$(GREEN)Frontend server starting on http://localhost:$(DEV_PORT)$(NC)" && \
		npm run dev

.PHONY: dev-backend
dev-backend: check-frontend-server build-frontend-if-needed ## Start backend development (Tauri app)
	@echo "$(BLUE)🦀 Starting backend development server...$(NC)"
	@cd $(BACKEND_DIR) && \
		export RUST_LOG="$(RUST_LOG)" && \
		export TAURI_ENV_DEBUG="true" && \
		echo "$(GREEN)Building and starting Tauri application...$(NC)" && \
		cargo build $(CARGO_DEV_FLAGS) && \
		cargo run --bin manifest

.PHONY: dev-frontend-only
dev-frontend-only: dev-frontend ## Alias for dev-frontend

.PHONY: dev-backend-only  
dev-backend-only: dev-backend ## Alias for dev-backend

# -----------------------------------------------------------------------------
# Build System
# -----------------------------------------------------------------------------

.PHONY: build
build: build-frontend build-backend ## Build complete application for production
	@echo "$(GREEN)✅ Complete build finished!$(NC)"
	@echo "$(CYAN)📦 Built artifacts:$(NC)"
	@echo "  Frontend: $(FRONTEND_DIR)/dist/"
	@echo "  Backend:  $(BACKEND_DIR)/target/release/"

.PHONY: build-frontend
build-frontend: setup-node ## Build frontend for production
	@echo "$(BLUE)🌐 Building frontend...$(NC)"
	@cd $(FRONTEND_DIR) && \
		export NODE_ENV="production" && \
		npm run build
	@echo "$(GREEN)✅ Frontend build complete$(NC)"

.PHONY: build-backend  
build-backend: build-zig ## Build backend for production
	@echo "$(BLUE)🦀 Building backend...$(NC)"
	@cd $(BACKEND_DIR) && \
		cargo build $(CARGO_FLAGS)
	@echo "$(GREEN)✅ Backend build complete$(NC)"

.PHONY: build-zig
build-zig: ## Build Zig SIMD library (automatically called by Rust build)
	@echo "$(BLUE)⚡ Building Zig SIMD library...$(NC)"
	@if command -v zig >/dev/null 2>&1; then \
		cd $(ZIG_DIR) && \
		zig build -Doptimize=ReleaseFast && \
		echo "$(GREEN)✅ Zig library built with SIMD optimizations$(NC)"; \
	else \
		echo "$(YELLOW)⚠️  Zig not available, using fallback implementations$(NC)"; \
	fi

.PHONY: build-debug
build-debug: setup-node ## Build in debug mode (faster compilation)
	@echo "$(BLUE)🔧 Building in debug mode...$(NC)"
	@$(MAKE) build-frontend
	@cd $(BACKEND_DIR) && cargo build $(CARGO_DEV_FLAGS)
	@echo "$(GREEN)✅ Debug build complete$(NC)"

# -----------------------------------------------------------------------------
# Testing
# -----------------------------------------------------------------------------

.PHONY: test
test: test-backend test-frontend ## Run all tests
	@echo "$(GREEN)✅ All tests completed$(NC)"

.PHONY: test-backend
test-backend: ## Run Rust backend tests
	@echo "$(BLUE)🧪 Running backend tests...$(NC)"
	@cd $(BACKEND_DIR) && cargo test
	@echo "$(GREEN)✅ Backend tests passed$(NC)"

.PHONY: test-frontend
test-frontend: setup-node ## Run frontend tests
	@echo "$(BLUE)🧪 Running frontend tests...$(NC)"
	@cd $(FRONTEND_DIR) && npm test
	@echo "$(GREEN)✅ Frontend tests passed$(NC)"

.PHONY: test-zig
test-zig: ## Run Zig library tests
	@echo "$(BLUE)🧪 Testing Zig SIMD library...$(NC)"
	@if command -v zig >/dev/null 2>&1; then \
		cd $(ZIG_DIR) && zig build test && \
		echo "$(GREEN)✅ Zig tests passed$(NC)"; \
	else \
		echo "$(YELLOW)⚠️  Zig not available, skipping tests$(NC)"; \
	fi

.PHONY: bench
bench: ## Run performance benchmarks
	@echo "$(BLUE)⚡ Running performance benchmarks...$(NC)"
	@cd $(BACKEND_DIR) && cargo bench --features=bench
	@echo "$(GREEN)✅ Benchmarks complete$(NC)"

# -----------------------------------------------------------------------------
# Code Quality & Linting
# -----------------------------------------------------------------------------

.PHONY: check
check: check-backend check-frontend ## Run all code quality checks
	@echo "$(GREEN)✅ All checks passed$(NC)"

.PHONY: check-backend
check-backend: ## Check Rust code quality
	@echo "$(BLUE)🔍 Checking backend code quality...$(NC)"
	@cd $(BACKEND_DIR) && \
		cargo fmt --check && \
		cargo clippy -- -D warnings && \
		cargo check
	@echo "$(GREEN)✅ Backend checks passed$(NC)"

.PHONY: check-frontend  
check-frontend: setup-node ## Check frontend code quality
	@echo "$(BLUE)🔍 Checking frontend code quality...$(NC)"
	@cd $(FRONTEND_DIR) && \
		npm run type-check && \
		npm run lint && \
		npm run format:check
	@echo "$(GREEN)✅ Frontend checks passed$(NC)"

.PHONY: lint
lint: lint-backend lint-frontend ## Run linters for all code

.PHONY: lint-backend
lint-backend: ## Lint Rust code
	@cd $(BACKEND_DIR) && cargo clippy -- -D warnings

.PHONY: lint-frontend
lint-frontend: setup-node ## Lint frontend code
	@cd $(FRONTEND_DIR) && npm run lint

.PHONY: format
format: format-backend format-frontend ## Format all code

.PHONY: format-backend
format-backend: ## Format Rust code
	@echo "$(BLUE)🎨 Formatting backend code...$(NC)"
	@cd $(BACKEND_DIR) && cargo fmt
	@echo "$(GREEN)✅ Backend formatted$(NC)"

.PHONY: format-frontend
format-frontend: setup-node ## Format frontend code
	@echo "$(BLUE)🎨 Formatting frontend code...$(NC)"
	@cd $(FRONTEND_DIR) && npm run format
	@echo "$(GREEN)✅ Frontend formatted$(NC)"

# -----------------------------------------------------------------------------
# Deployment & Packaging
# -----------------------------------------------------------------------------

.PHONY: package
package: build package-$(PLATFORM) ## Build platform-specific packages

.PHONY: package-macos
package-macos: build ## Build macOS app bundle and DMG
	@echo "$(BLUE)📦 Packaging for macOS...$(NC)"
	@cd $(BACKEND_DIR)/scripts && npm run build:macos
	@echo "$(GREEN)✅ macOS package complete$(NC)"

.PHONY: package-linux
package-linux: build ## Build Linux AppImage and DEB
	@echo "$(BLUE)📦 Packaging for Linux...$(NC)"
	@cd $(BACKEND_DIR)/scripts && npm run build:linux
	@echo "$(GREEN)✅ Linux package complete$(NC)"

.PHONY: package-windows
package-windows: build ## Build Windows MSI installer
	@echo "$(BLUE)📦 Packaging for Windows...$(NC)"
	@cd $(BACKEND_DIR)/scripts && npm run build:windows
	@echo "$(GREEN)✅ Windows package complete$(NC)"

.PHONY: package-all
package-all: build ## Build packages for all platforms
	@echo "$(BLUE)📦 Packaging for all platforms...$(NC)"
	@cd $(BACKEND_DIR)/scripts && npm run build
	@echo "$(GREEN)✅ All packages complete$(NC)"

# -----------------------------------------------------------------------------
# Maintenance & Cleanup
# -----------------------------------------------------------------------------

.PHONY: clear-cache
clear-cache: ## Clear Vite dependency caches (fixes 504 errors)
	@echo "$(BLUE)🧹 Clearing development caches...$(NC)"
	@cd $(FRONTEND_DIR) && npm run clear-cache
	@echo "$(GREEN)✅ Development caches cleared$(NC)"

.PHONY: clean
clean: clean-backend clean-frontend clean-zig ## Clean all build artifacts
	@echo "$(GREEN)✅ All build artifacts cleaned$(NC)"

.PHONY: clean-backend
clean-backend: ## Clean Rust build artifacts
	@echo "$(BLUE)🧹 Cleaning backend build artifacts...$(NC)"
	@cd $(BACKEND_DIR) && \
		cargo clean && \
		rm -rf target/
	@echo "$(GREEN)✅ Backend cleaned$(NC)"

.PHONY: clean-frontend
clean-frontend: ## Clean frontend build artifacts  
	@echo "$(BLUE)🧹 Cleaning frontend build artifacts...$(NC)"
	@cd $(FRONTEND_DIR) && \
		rm -rf dist/ && \
		rm -rf .vite/ && \
		rm -rf node_modules/.cache/
	@echo "$(GREEN)✅ Frontend cleaned$(NC)"

.PHONY: clean-zig
clean-zig: ## Clean Zig build artifacts
	@echo "$(BLUE)🧹 Cleaning Zig build artifacts...$(NC)"
	@cd $(ZIG_DIR) && \
		rm -rf zig-out/ && \
		rm -rf .zig-cache/
	@echo "$(GREEN)✅ Zig artifacts cleaned$(NC)"

.PHONY: clean-all
clean-all: clean ## Alias for clean (clean all artifacts)

.PHONY: reset
reset: clean ## Reset to clean development state
	@echo "$(BLUE)🔄 Resetting development environment...$(NC)"
	@cd $(FRONTEND_DIR) && rm -rf node_modules/
	@cd $(BACKEND_DIR)/scripts && rm -rf node_modules/
	@echo "$(GREEN)✅ Development environment reset$(NC)"
	@echo "$(CYAN)Run 'make setup' to reinitialize$(NC)"

# -----------------------------------------------------------------------------
# Utility Targets
# -----------------------------------------------------------------------------

.PHONY: update
update: update-frontend update-backend ## Update all dependencies

.PHONY: update-frontend
update-frontend: ## Update frontend dependencies
	@echo "$(BLUE)📦 Updating frontend dependencies...$(NC)"
	@cd $(FRONTEND_DIR) && npm update
	@echo "$(GREEN)✅ Frontend dependencies updated$(NC)"

.PHONY: update-backend
update-backend: ## Update Rust dependencies
	@echo "$(BLUE)📦 Updating backend dependencies...$(NC)"
	@cd $(BACKEND_DIR) && cargo update
	@echo "$(GREEN)✅ Backend dependencies updated$(NC)"

.PHONY: logs
logs: ## View application logs
	@echo "$(BLUE)📋 Recent application logs:$(NC)"
	@find $(BACKEND_DIR)/logs -name "*.log" -type f -exec echo "$(YELLOW)=== {} ===$(NC)" \; -exec tail -20 {} \; 2>/dev/null || echo "No logs found"

.PHONY: info
info: ## Show project information
	@echo "$(PURPLE)📊 Manifest Project Information$(NC)"
	@echo "$(BLUE)================================$(NC)"
	@echo "Project Name: $(PROJECT_NAME)"
	@echo "Version:      $(VERSION)"
	@echo "Platform:     $(PLATFORM) ($(ARCH))"
	@echo "Rust Version: $(RUST_VERSION)"
	@echo ""
	@echo "$(YELLOW)Directories:$(NC)"
	@echo "Root:      $(ROOT_DIR)"
	@echo "Backend:   $(BACKEND_DIR)"
	@echo "Frontend:  $(FRONTEND_DIR)" 
	@echo "Zig:       $(ZIG_DIR)"
	@echo ""
	@echo "$(YELLOW)Toolchain Status:$(NC)"
	@printf "Rust:     "; rustc --version 2>/dev/null || echo "Not installed"
	@printf "Node:     "; node --version 2>/dev/null || echo "Not installed"
	@printf "Zig:      "; zig version 2>/dev/null || echo "Not installed (optional)"

# -----------------------------------------------------------------------------
# Internal Helper Targets
# -----------------------------------------------------------------------------

.PHONY: check-frontend-server
check-frontend-server:
	@if ! curl -s "http://localhost:$(DEV_PORT)" > /dev/null 2>&1; then \
		echo "$(YELLOW)ℹ️  Frontend server not detected on port $(DEV_PORT)$(NC)"; \
	else \
		echo "$(GREEN)✅ Frontend server detected$(NC)"; \
	fi

.PHONY: build-frontend-if-needed
build-frontend-if-needed:
	@if [ ! -d "$(FRONTEND_DIR)/dist" ] || [ -z "$$(find $(FRONTEND_DIR)/dist -name '*.js' -o -name '*.html' 2>/dev/null)" ]; then \
		echo "$(YELLOW)Frontend not built, building now...$(NC)"; \
		$(MAKE) build-frontend; \
	fi

# -----------------------------------------------------------------------------
# Special Targets
# -----------------------------------------------------------------------------

# Prevent deletion of intermediate files
.PRECIOUS: $(FRONTEND_DIR)/dist $(BACKEND_DIR)/target

# Default target
.DEFAULT_GOAL := help

# Declare phony targets (targets that don't create files)
.PHONY: all help setup dev build test clean package info logs update

# -----------------------------------------------------------------------------
# Platform-Specific Extensions (Optional)
# -----------------------------------------------------------------------------

ifeq ($(PLATFORM),macos)
.PHONY: dev-xcode
dev-xcode: ## Open project in Xcode (macOS only)
	@if [ -d "$(BACKEND_DIR)/target/debug/bundle/osx/manifest.app" ]; then \
		open -a Xcode $(BACKEND_DIR)/target/debug/bundle/osx/manifest.app; \
	else \
		echo "$(RED)App bundle not found. Run 'make build-debug' first.$(NC)"; \
	fi
endif

# vim: set noexpandtab tabstop=4 shiftwidth=4:
