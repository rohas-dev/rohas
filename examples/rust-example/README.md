# rust-example

Rohas project initialized with rust handlers.

## Getting Started

### For End Users

If you have the `rohas` CLI installed:

1. Generate code:
   ```bash
   rohas codegen
   ```

2. Start development server:
   ```bash
   rohas dev --workbench
   ```

3. Validate schema:
   ```bash
   rohas validate
   ```

### For Rohas Developers

If you're developing Rohas itself and working in the examples directory:

1. Use the development helper script:
   ```bash
   ./dev.sh --workbench
   ```
   
   This script automatically finds the workspace root and compiles/runs the CLI.

2. Or use Make shortcuts:
   ```bash
   make dev ARGS="--workbench"  # Start dev server
   make codegen                 # Generate code
   make check                   # Check Rust code
   make validate                # Validate schema
   ```

3. Or from workspace root:
   ```bash
   cd ../..
   cargo run -p rohas-cli -- dev --workbench --schema examples/rust-example/schema
   ```

## Project Structure

- `schema/` - Schema definitions (.ro files)
- `src/handlers/` - Your handler implementations
- `src/generated/` - Auto-generated Rust types (DO NOT EDIT)
- `config/` - Configuration files
- `dev.sh` - Development helper script
- `Makefile` - Development shortcuts

## Development Workflow

1. Edit your schema files in `schema/`
2. Run `make codegen` to regenerate types
3. Implement handlers in `src/handlers/`
4. Run `make dev` to start the dev server
5. The server will automatically recompile Rust handlers on file changes
