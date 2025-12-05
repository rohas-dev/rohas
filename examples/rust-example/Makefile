# Makefile for Rohas developers working in examples
# End users: Install rohas CLI and use "rohas dev --workbench" directly

.PHONY: dev dev-watch codegen check build validate

# Run development server (for Rohas developers - finds workspace automatically)
# Usage: make dev ARGS="--workbench"
dev:
	@./dev.sh $(ARGS)

# Run development server with workbench
dev-watch:
	@./dev.sh --workbench

# Generate code from schema (for Rohas developers)
codegen:
	@SCRIPT_DIR=$$(pwd); \
	WORKSPACE_ROOT=$$SCRIPT_DIR; \
	for i in {1..10}; do \
		if [ -f "$$WORKSPACE_ROOT/Cargo.toml" ] && grep -q "^\[workspace\]" "$$WORKSPACE_ROOT/Cargo.toml" 2>/dev/null && [ -d "$$WORKSPACE_ROOT/crates/rohas-cli" ]; then \
			break; \
		fi; \
		WORKSPACE_ROOT=$$(dirname "$$WORKSPACE_ROOT"); \
	done; \
	cd "$$WORKSPACE_ROOT" && cargo run -p rohas-cli -- codegen --schema "$$SCRIPT_DIR/schema" --output "$$SCRIPT_DIR/src" --lang rust

# Check Rust code
check:
	@CARGO_TARGET_DIR=../../target cargo check

# Build Rust project
build:
	@CARGO_TARGET_DIR=../../target cargo build --release

# Validate schema (for Rohas developers)
validate:
	@SCRIPT_DIR=$$(pwd); \
	WORKSPACE_ROOT=$$SCRIPT_DIR; \
	for i in {1..10}; do \
		if [ -f "$$WORKSPACE_ROOT/Cargo.toml" ] && grep -q "^\[workspace\]" "$$WORKSPACE_ROOT/Cargo.toml" 2>/dev/null && [ -d "$$WORKSPACE_ROOT/crates/rohas-cli" ]; then \
			break; \
		fi; \
		WORKSPACE_ROOT=$$(dirname "$$WORKSPACE_ROOT"); \
	done; \
	cd "$$WORKSPACE_ROOT" && cargo run -p rohas-cli -- validate --schema "$$SCRIPT_DIR/schema"
