---
name: Bug Report
about: Something isn't working as expected in susshi
title: "fix: "
labels: bug
assignees: ''

---

## Describe the bug

A clear and concise description of what the bug is.

## Steps to reproduce

1. Launch susshi with `susshi` (or `susshi --config ...`)
2. Navigate to '...'
3. Press key '...'
4. Observe: ...

## Expected behavior

What you expected to happen instead.

## Error output

If susshi printed an error or exited unexpectedly, paste it here:

```

```

## Minimal config (`~/.susshi.yml`)

Provide the smallest config that reproduces the issue. **Remove all sensitive data** (real hostnames, IPs, usernames, SSH keys).

```yaml
groups:
  - name: "Example"
    servers:
      - name: "test-server"
        host: "192.0.2.1"
        mode: direct
```

## Environment

| Field | Value |
|---|---|
| susshi version | `susshi --version` |
| OS | e.g. Ubuntu 24.04 / macOS 15 / Windows 11 |
| Terminal emulator | e.g. Alacritty, Kitty, WezTerm, iTerm2 |
| SSH version | `ssh -V` |
| Installation method | binary / AUR / built from source |

## Screenshots / recordings

If the bug is visual, add a screenshot or an [asciinema](https://asciinema.org) recording.

## Additional context

Anything else that might help.
