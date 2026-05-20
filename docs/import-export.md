# Import and Export

## Import from OpenSSH Config

Generate susshi YAML from an existing `~/.ssh/config`:

```bash
susshi --import-ssh-config
susshi --import-ssh-config --dry-run                        # preview without writing
susshi --import-ssh-config --output ~/.susshi.yml           # write to specific file
susshi --import-ssh-config --ssh-config-path ~/work/.ssh/config
```

Behavior:

- Recursive `Include` directives are followed.
- `ProxyJump` entries are converted to jump-mode configuration.
- The output is printed to stdout unless `--output` is specified.

## List Servers as JSON

Print all resolved servers as a JSON array — ready for `jq` or `fzf` pipelines:

```bash
susshi --list
susshi --list --list-filter "web"
susshi --list --list-filter "#prod"
```

Each object contains: `name`, `host`, `user`, `port`, `group`, `env`, `namespace`, `tags`, `mode`.

Example pipelines:

```bash
# Pick a server with fzf and connect
susshi --list | jq -r '.[].name' | fzf | xargs -I{} susshi --direct {}

# Show all hostnames tagged prod
susshi --list --list-filter "#prod" | jq -r '.[].host'
```

## Export Inventory

All export commands support `--export-filter` with the same text and `#tag` syntax as the TUI search bar. Output goes to stdout unless `--export-output <file>` is specified.

### Ansible

```bash
susshi --export ansible
susshi --export ansible --export-output ~/inventory.yml
susshi --export ansible --export-filter "#prod"
```

Produces a standard Ansible YAML inventory with groups mapped from susshi groups and environments.

### Terraform

```bash
susshi --export terraform
susshi --export terraform --export-output inventory.json
susshi --export terraform --export-filter "#prod"
```

Produces a JSON file loadable with `jsondecode(file(...))`:

```hcl
locals {
  susshi = jsondecode(file("${path.module}/inventory.json"))
}

resource "null_resource" "ping" {
  for_each = { for s in local.susshi.servers : s.name => s }
  # each.value.host, each.value.user, each.value.port, …
}
```

### Nmap / masscan

```bash
susshi --export nmap
susshi --export nmap --export-output targets.txt
susshi --export nmap --export-filter "#prod"

nmap -iL targets.txt -p 22 -sV
masscan -iL targets.txt -p 22
```

### CSV

Columns: `name`, `host`, `user`, `port`, `ssh_key`, `group`, `env`, `namespace`, `tags`, `notes`.

```bash
susshi --export csv
susshi --export csv --export-output ~/servers.csv
susshi --export csv --export-filter "#prod"
```

Fields with commas, double-quotes, or newlines are quoted per RFC 4180. Multiple tags are joined with `;`.

### OpenSSH Config

Exports servers as `Host` blocks compatible with `~/.ssh/config`:

```bash
susshi --export openssh
susshi --export openssh --export-output ~/.ssh/config.d/susshi
susshi --export openssh --export-filter "#prod"
```

Each server produces a `Host` block with `HostName`, `User`, `Port` (if not 22), `IdentityFile`, `ProxyJump` (jump mode), and `IdentityAgent` (if `ssh_agent_sock` is set). Wallix servers are exported as direct blocks — their bastion routing is not representable in standard SSH config.
