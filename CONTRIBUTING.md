# Contributing to susshi 🍣

Thank you for taking the time to contribute! This guide covers everything you need to get started.

---

## Table of contents

1. [Development setup](#development-setup)
2. [Workflow](#workflow)
3. [Commit conventions](#commit-conventions)
4. [Coding standards](#coding-standards)
5. [Testing](#testing)
6. [Submitting a pull request](#submitting-a-pull-request)
7. [Reporting bugs](#reporting-bugs)
8. [Requesting features](#requesting-features)

---

## Development setup

**Prerequisites**: Rust stable, a terminal with truecolor support.

```bash
rustup update stable
git clone https://github.com/yatoub/susshi.git
cd susshi
cargo build
cargo run
```

A minimal `~/.susshi.yml` is enough to test the TUI locally. See [`examples/full_config.yaml`](examples/full_config.yaml) for a fully annotated reference.

---

## Workflow

> **`master` is a protected branch.** Direct pushes are not allowed. All changes go through a pull request.

1. **Fork** the repository (external contributors) or create a branch (maintainers).
2. Branch off `master`: `git checkout -b fix/my-bug` or `feat/my-feature`.
3. Write your code — **tests first** (TDD).
4. Ensure the full quality gate passes locally (see [Coding standards](#coding-standards)).
5. Open a Pull Request against `master` using the PR template.
6. The CI (`ci.yml`) must pass: fmt + clippy + tests.

> Do **not** bump the version in `Cargo.toml` or create a git tag manually.  
> Releases are handled automatically by [release-plz](https://release-plz.gg/) once your PR is merged:  
> it opens a release PR, bumps the version, generates the CHANGELOG, creates the tag and the GitHub Release.  
> The `release.yml` workflow then builds the binaries for all platforms.

---

## Commit conventions

All commits **must** follow [Conventional Commits](https://www.conventionalcommits.org/en/v1.0.0/).  
This is not just style — release-plz uses commit types to determine the next version and to generate the CHANGELOG automatically.

| Type | When to use | Version bump |
|---|---|---|
| `feat:` | New user-facing feature | minor |
| `fix:` | Bug fix | patch |
| `feat!:` / `fix!:` | Breaking change | major |
| `perf:` | Performance improvement | patch |
| `refactor:` | Internal restructure, no behaviour change | — |
| `docs:` | Documentation only | — |
| `test:` | Tests only | — |
| `chore:` | Build, CI, dependencies, tooling | — |

**Examples:**
```
feat: add SCP file transfer with real-time progress
fix: cast TIOCSCTTY to c_ulong for macOS ioctl compatibility
docs: add tunnel configuration example to README
test: add unit tests for build_tunnel_args
```

**Multi-line commit bodies**: fish shell does not support heredoc-style inline multiline strings. Write the message to a file and use `git commit -F`:

```bash
# Write message to a temp file, then:
git commit -F /tmp/my_commit_msg.txt
```

---

## Coding standards

All three checks must pass before opening a PR — the CI will enforce them.

```bash
# 1. Format
cargo fmt --all

# 2. Lint (zero warnings)
cargo clippy --all-targets --all-features -- -D warnings

# 3. Tests
cargo test
```

**Additional rules:**

- No `unwrap()` in production code without an explicit comment justifying why it cannot panic.
- Unix-only APIs (`libc`, `nix`, `std::os::unix::process::CommandExt`) must be gated behind `#[cfg(unix)]`.  
  `nix` is declared under `[target.'cfg(unix)'.dependencies]` in `Cargo.toml` — keep it there.
- Public items (functions, structs, enums) must have English doc comments (`///`).
- Internal/private comments may be in French.

---

## Testing

We follow TDD: write tests before (or alongside) implementation.

```bash
# Run all tests including integration tests in tests/
cargo test

# Run a specific test
cargo test tunnel_args_includes_ssh_key
```

- Unit tests live in `#[cfg(test)] mod tests` at the bottom of each module.
- Integration tests live in `tests/` and load fixtures from `tests/fixtures/`.
- If you add a new SSH argument or config option, add a corresponding unit test in `src/ssh/client.rs`, `src/ssh/tunnel.rs`, or `src/ssh/scp.rs`.

---

## Submitting a pull request

1. Fill in the [PR template](.github/PULL_REQUEST_TEMPLATE.md).
2. The PR **title** must follow Conventional Commits — it becomes the squash-merge commit message and feeds the CHANGELOG.
3. If the PR changes the TUI, attach a screenshot or an [asciinema](https://asciinema.org) recording.
4. A maintainer will review and merge. release-plz takes care of the rest.

---

## Reporting bugs

Use the [Bug Report template](.github/ISSUE_TEMPLATE/bug_report.md). Include:

- Steps to reproduce.
- Expected vs. actual behaviour.
- A **sanitized** minimal `~/.susshi.yml` (no real hosts, IPs, usernames or keys).
- Environment: OS, terminal emulator, `ssh -V` output, susshi version.

---

## Requesting features

Use the [Feature Request template](.github/ISSUE_TEMPLATE/feature_request.md). Describe the problem you're solving, your proposed UX (keybinding, config key, TUI panel), and any config schema changes.

---

## License

By contributing, you agree that your contributions will be licensed under the project's [MIT License](LICENCE).

