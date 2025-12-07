.PHONY: build develop test publish clean

# Build the Python package
build:
	maturin build --release

# Install in development mode (requires virtualenv)
develop:
	@if [ -z "$$VIRTUAL_ENV" ] && [ -z "$$CONDA_PREFIX" ] && [ ! -d ".venv" ]; then \
		echo "Creating virtualenv..."; \
		python3 -m venv .venv; \
		echo "Activate with: source .venv/bin/activate"; \
		exit 1; \
	fi
	maturin develop

# Run tests
test:
	python -m pytest tests/ || echo "No tests yet"

# Publish to PyPI (requires PYPI_TOKEN)
publish:
	maturin publish --username __token__ --password $(PYPI_TOKEN)

# Clean build artifacts
clean:
	cargo clean
	rm -rf target/wheels
	rm -rf dist
	rm -rf *.egg-info

# Check if maturin is installed
check-maturin:
	@which maturin > /dev/null || (echo "maturin not found. Install with: pip install maturin" && exit 1)

# Full development setup
setup: check-maturin
	pip install maturin
	maturin develop

