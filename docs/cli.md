# CLI Guide

susshi can run as a command-line connector without opening the TUI.

## Connect Modes

```bash
# Direct
susshi --direct ops-user@app-01.internal.example
susshi --direct ops-user@198.51.100.5:2222

# Jump
susshi --jump ops-user@198.51.100.50

# Wallix
susshi --wallix web-01.internal.example
```

## SSH Overrides

```bash
susshi --direct app-01.internal.example --user deploy --port 2222 --key ~/.ssh/deploy_rsa
susshi --direct app-01.internal.example --verbose
```

## Alternate Config

```bash
susshi --config ~/work/.susshi.yml
```

## Import OpenSSH Config

Generate susshi YAML from an OpenSSH config file:

```bash
susshi --import-ssh-config
susshi --import-ssh-config --dry-run
susshi --import-ssh-config --output ~/.susshi.yml
susshi --import-ssh-config --ssh-config-path ~/work/.ssh/config
```

Behavior:

- Recursive `Include` directives are supported.
- `ProxyJump` is converted to jump-mode configuration.

## Export Ansible Inventory

```bash
susshi --export ansible
susshi --export ansible --export-output ~/inventory.yml
susshi --export ansible --export-filter "#prod"
susshi --export ansible --export-filter "web"
```

The same text + tag filter model as TUI search is applied.

## Export CSV

Exports all servers as a CSV file with columns: `name`, `host`, `user`, `port`, `ssh_key`, `group`, `env`, `namespace`, `tags`, `notes`.

```bash
susshi --export csv
susshi --export csv --export-output ~/servers.csv
susshi --export csv --export-filter "#prod"
```

Fields containing commas, double-quotes, or newlines are quoted per RFC 4180. Multiple tags are joined with `;`.

## Export OpenSSH Config

Exports all servers as `~/.ssh/config`-compatible `Host` blocks.

```bash
susshi --export openssh
susshi --export openssh --export-output ~/.ssh/config.d/susshi
susshi --export openssh --export-filter "#prod"
```

Each server produces a `Host` block with `HostName`, `User`, `Port` (if not 22), `IdentityFile`, `ProxyJump` (jump mode), and `IdentityAgent` (if `ssh_agent_sock` is set). Wallix servers are exported as direct blocks — their bastion routing is not representable in standard SSH config.

## Help

```bash
susshi --help
```
