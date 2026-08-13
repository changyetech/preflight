# ==============================================================================
# ipcheck - Makefile
# ==============================================================================

# --- Variables ----------------------------------------------------------------

APP_NAME        := ipcheck

# Colors
CYAN  := \033[36m
RESET := \033[0m
BOLD  := \033[1m

.DEFAULT_GOAL := help

# ==============================================================================
# HELP
# ==============================================================================

.PHONY: help
help: ## Show this help
	@echo ""
	@echo "$(BOLD)ipcheck$(RESET)"
	@echo ""
	@awk 'BEGIN {FS = ":.*##"} /^[a-zA-Z_-]+:.*?##/ { printf "  $(CYAN)%-20s$(RESET) %s\n", $$1, $$2 }' $(MAKEFILE_LIST)
	@echo ""

# ==============================================================================
# BUILD
# ==============================================================================

.PHONY: build
build: ## Build for production
	pnpm build

# ==============================================================================
# DEV
# ==============================================================================

.PHONY: dev
dev: ## Start dev server (Vite + @cloudflare/vite-plugin：前端 HMR 与 /api/* 同一进程)
	pnpm dev

.PHONY: preview
preview: ## Build then preview the production bundle in workerd
	pnpm preview

# ==============================================================================
# DEPENDENCY MANAGEMENT
# ==============================================================================

.PHONY: install
install: ## Install dependencies
	pnpm install

# ==============================================================================
# TESTING
# ==============================================================================

.PHONY: test
test: ## Run tests
	pnpm vitest run

# ==============================================================================
# CODE QUALITY
# ==============================================================================

.PHONY: lint
lint: ## Run linter
	pnpm lint

.PHONY: fmt
fmt: ## Format code
	pnpm exec prettier --write "src/**/*.{ts,tsx,css}" "worker/**/*.ts" "tests/**/*.ts" "*.{ts,json}"

.PHONY: check
check: fmt lint build test ## Run all quality checks, web only (fmt + lint + build + test)

# ==============================================================================
# CLI (Rust)
# ==============================================================================
# 刻意与 Web 的 target 分开：改一行 CSS 不该等 Rust 编译，
# 反过来改 CLI 也不该等 vite build。两侧各自独立，`check-all` 才跑全部。

.PHONY: cli-build
cli-build: ## Build the Rust CLI
	cargo build

.PHONY: cli-release
cli-release: ## Build the Rust CLI (release, target/release/)
	cargo build --release

.PHONY: cli-test
cli-test: ## Run Rust tests
	cargo test

.PHONY: cli-lint
cli-lint: ## Run clippy
	cargo clippy --all-targets -- -D warnings

.PHONY: cli-fmt
cli-fmt: ## Format Rust code
	cargo fmt --all

.PHONY: check-cli
check-cli: cli-fmt cli-lint cli-build cli-test ## Run all quality checks, CLI only

.PHONY: check-all
check-all: check check-cli ## Run all quality checks for both web and CLI

# ==============================================================================
# HOUSEKEEPING
# ==============================================================================

.PHONY: clean
clean: ## Remove build artifacts and generated files
	rm -rf dist node_modules/.tmp .wrangler target
