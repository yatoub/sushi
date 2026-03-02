---
name: Config Help / Question
about: Need help with your susshi configuration?
title: "[config] "
labels: question
assignees: ''

---

## What are you trying to achieve?

Describe your setup in plain terms:
> e.g. "I have a jump host at `jump.example.com` and want to reach servers inside a private network. I also use a Wallix bastion for a second environment."

## Your config (`~/.susshi.yml`)

**Remove all sensitive data** (real hostnames, IPs, usernames, SSH keys) before pasting.

```yaml
defaults:
  user: admin
  mode: direct

groups:
  - name: "MyGroup"
    servers:
      - name: "my-server"
        host: "192.0.2.1"
```

## What's happening?

- [ ] The config fails to parse — error: `...`
- [ ] The config parses but the connection fails
- [ ] The TUI doesn't display what I expect
- [ ] I'm not sure how to write the config for my use case

## Environment

| Field | Value |
|---|---|
| susshi version | `susshi --version` |
| OS | e.g. Ubuntu 24.04 / macOS 15 |
| Connection mode used | direct / jump / wallix |

## Additional context

Anything else that might help (SSH config, network topology diagram, etc.).
