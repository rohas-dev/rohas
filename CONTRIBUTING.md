# Contributing to Rohas

Thank you for your interest in contributing to Rohas! This document provides guidelines and instructions for contributing.

## Development Setup

### Prerequisites

- Rust 1.70 or higher
- Cargo
- Node.js 18+ (for TypeScript runtime testing)
- Python 3.9+ (for Python runtime testing)

### Building the Project

```bash
# Clone the repository
git clone https://github.com/rohas-dev/rohas.git
cd rohas

# Build all crates
cargo build

# Run tests
cargo test

# Build CLI
cargo build --release -p rohas-cli
```

## Project Structure

The project is organized as a Cargo workspace with the following crates:

- `rohas-parser`: Schema parsing and AST
- `rohas-engine`: Core engine and event system
- `rohas-runtime`: Runtime executors (Python, Node.js)
- `rohas-cli`: Command-line interface
- `rohas-codegen`: Code generation
- `rohas-cron`: Cron job scheduling
- `rohas-dev-server`: Development server
- `rohas-adapters/`: Event adapters (Memory, NATS, Kafka, RabbitMQ, SQS)

## Making Changes

### Branching Strategy

We use the following branch naming conventions:

- `feat/` - New features
- `fix/` - Bug fixes
- `docs/` - Documentation updates
- `refactor/` - Code refactoring
- `test/` - Test additions or improvements
- `chore/` - Maintenance tasks

Example: `feat/add-postgres-adapter`, `fix/parser-validation-bug`

### Commit Messages

We follow conventional commit format:

```
<type>(<scope>): <description>

[optional body]

[optional footer]
```

Types:
- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation changes
- `test`: Test additions/changes
- `refactor`: Code refactoring
- `chore`: Maintenance tasks
- `perf`: Performance improvements

Example:
```
feat(parser): add support for nested models

- Implement nested model parsing
- Add validation for circular references
- Update tests

Closes #123
```

### Testing

Before submitting a PR:

```bash
# Run all tests
cargo test

# Run specific crate tests
cargo test -p rohas-parser

# Run with logging
RUST_LOG=debug cargo test

# Format code
cargo fmt --all

# Run clippy
cargo clippy --all-targets --all-features
```

## Pull Request Process

1. **Create a feature branch** from `main`
2. **Make your changes** following the coding standards
3. **Add tests** for new functionality
4. **Update documentation** if needed
5. **Run tests and linting** to ensure quality
6. **Submit a PR** with a clear description
7. **Address review feedback** promptly

### PR Template

```markdown
## Description
Brief description of changes

## Type of Change
- [ ] Bug fix
- [ ] New feature
- [ ] Breaking change
- [ ] Documentation update

## Testing
Describe testing performed

## Checklist
- [ ] Tests pass locally
- [ ] Code formatted (cargo fmt)
- [ ] Lints pass (cargo clippy)
- [ ] Documentation updated
```

## Code Style

- Follow Rust standard formatting (use `cargo fmt`)
- Use meaningful variable and function names
- Add comments for complex logic
- Keep functions focused and small
- Write unit tests for new functionality

## Questions?

If you have questions or need help:
- Open a GitHub issue
- Join our Discord community
- Check existing documentation

## License

By contributing, you agree that your contributions will be licensed under the project's license.

