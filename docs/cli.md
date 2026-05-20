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

## List Servers (JSON)

Print all resolved servers as JSON — one object per server, ready for `jq` or `fzf`:

```bash
susshi --list
susshi --list --list-filter "web"
susshi --list --list-filter "#prod"
```

Each entry contains: `name`, `host`, `user`, `port`, `group`, `env`, `namespace`, `tags`, `mode`.

Example pipelines:

```bash
# Pick a server with fzf and connect
susshi --list | jq -r '.[].name' | fzf | xargs -I{} susshi --direct {}

# Show all hostnames in the prod group
susshi --list --list-filter "#prod" | jq -r '.[].host'
```

## Execute a Command on a Group

Run a non-interactive SSH command on every server in a group, **in parallel**:

```bash
susshi --exec-group prod --exec-cmd "uptime"
susshi --exec-group prod --exec-cmd "df -h /data" --exec-timeout 15
```

- Each server runs in its own thread; results are printed as they arrive.
- Output is prefixed with `=== <server-name> (exit <code>) ===`.
- `--exec-timeout` (default: 30 s) sets `ConnectTimeout` per host.
- The command runs in `BatchMode=yes` — password prompts are never shown.
- Exit code is 0 only if **all** hosts succeed.

## Export Inventory

### Ansible

```bash
susshi --export ansible
susshi --export ansible --export-output ~/inventory.yml
susshi --export ansible --export-filter "#prod"
susshi --export ansible --export-filter "web"
```

### Terraform

Generates a JSON file loadable with `jsondecode(file(...))` in a Terraform `locals` block:

```bash
susshi --export terraform
susshi --export terraform --export-output inventory.json
susshi --export terraform --export-filter "#prod"
```

```hcl
locals {
  susshi = jsondecode(file("${path.module}/inventory.json"))
}

resource "null_resource" "ping" {
  for_each = { for s in local.susshi.servers : s.name => s }
  # each.value.host, each.value.user, etc.
}
```

### Nmap / masscan

Generates a target-list file usable with `-iL`:

```bash
susshi --export nmap
susshi --export nmap --export-output targets.txt
susshi --export nmap --export-filter "#prod"

nmap -iL targets.txt -p 22 -sV
masscan -iL targets.txt -p 22
```

---

The same text + tag filter syntax applies to all export and list commands.

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
