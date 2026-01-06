# Contributing to tb-rs

Thank you for your interest in contributing to tb-rs!

## Before You Start

**Every pull request MUST have an associated issue first.**

This is a strict requirement. Before submitting any PR:

1. Check if an issue already exists for your change
2. If not, create a new issue describing what you want to do
3. Wait for feedback/approval on the issue before starting work
4. Reference the issue number in your PR

This helps us:
- Discuss the approach before code is written
- Avoid duplicate work
- Keep the project focused
- Track why changes were made

## Creating an Issue

When creating an issue, please include:

- **Bug reports**: Steps to reproduce, expected vs actual behavior, TigerBeetle version
- **Feature requests**: Use case, proposed API, why it's needed
- **Questions**: What you're trying to do, what you've tried

## Pull Request Process

1. Fork the repository
2. Create a branch from `main`
3. Make your changes
4. Run tests: `cargo test -p tb-rs`
5. Run clippy: `cargo clippy -p tb-rs`
6. Submit PR with reference to the issue (e.g., "Fixes #123" or "Closes #123")

## Code Style

- Follow the style guide in `CLAUDE.md`
- Use `rustfmt` for formatting
- Prefer explicit types over inference for public APIs
- Add tests for new functionality
- Keep commits focused and atomic

## Versioning

This crate uses a special versioning scheme: `TB_VERSION+CRATE_VERSION`

- The main version indicates TigerBeetle server compatibility
- The build metadata indicates library changes

When contributing, you generally don't need to bump versions - maintainers will handle this.

## License

By contributing, you agree that your contributions will be licensed under the Apache-2.0 license.
