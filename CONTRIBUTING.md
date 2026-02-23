# Contributing to Sushi 🍣

First off, thank you for considering contributing to Sushi! It's people like you that make tools better for everyone.

Following these guidelines helps to communicate that you respect the time of the developers managing and developing this open source project. In return, they should reciprocate that respect in addressing your issue, assessing changes, and helping you finalize your pull requests.

## 🛠️ Development Setup

### Prerequisites

- **Rust**: Ensure you have the latest stable version of Rust installed.
  ```bash
  rustup update stable
  ```

### Building the Project

1. Clone the repository:
   ```bash
   git clone https://github.com/yourusername/sushi.git
   cd sushi
   ```

2. Build the project:
   ```bash
   cargo build
   ```

3. Run the project:
   ```bash
   cargo run
   ```

## 🧪 Testing

We value high code quality and reliability. Before submitting a PR, please ensure all tests pass.

### Running Unit Tests

```bash
cargo test
```

### Manual Testing

It is recommended to create a local `sushi.yml` config file for testing UI interactions and parsing logic manually.

## 🎨 Coding Style

We follow standard Rust coding conventions.

### Formatting

Please ensure your code is formatted with `rustfmt` before committing.

```bash
cargo fmt --all
```

### Linting

We use `clippy` to catch common mistakes and improve code quality.

```bash
cargo clippy --all-targets --all-features -- -D warnings
```

*Note: The CI pipeline will fail if there are any warnings or formatting issues.*

## 📬 Submitting a Pull Request

1. **Fork the Repository**: Create your own fork of the code.
2. **Create a Branch**: Create a branch for your feature or fix (`git checkout -b feature/amazing-feature`).
3. **Commit Changes**: Make sure your commit messages are clear and descriptive.
4. **Push to Branch**: Push your changes to your fork (`git push origin feature/amazing-feature`).
5. **Open a Pull Request**: Go to the original repository and click "New Pull Request".

### PR Guidelines

- **Title**: Use a clear title describing the change.
- **Description**: Follow the [PULL_REQUEST_TEMPLATE](.github/PULL_REQUEST_TEMPLATE.md). Explain *what* you changed and *why*.
- **Tests**: Include tests for any new logic.
- **Screenshots**: If you changed the TUI, please include a screenshot or GIF.

## 🐛 Reporting Bugs

Bugs are tracked as GitHub issues. When filing an issue, please use the [Bug Report Template](.github/ISSUE_TEMPLATE/bug_report.md) and include:

- A clear title and description.
- Steps to reproduce.
- Expected vs. actual behavior.
- Your `sushi.yml` configuration (sanitized).
- Environment details (OS, Terminal, Sushi version).

## 💡 Feature Requests

Have an idea? We'd love to hear it! Please use the [Feature Request Template](.github/ISSUE_TEMPLATE/feature_request.md).

## 📜 License

By contributing, you agree that your contributions will be licensed under the project's [MIT License](LICENSE).
