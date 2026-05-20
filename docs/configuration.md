# Configuration Guide

susshi reads `~/.susshi.yml` by default. This page documents the full schema and inheritance model.

## Configuration Inheritance

Settings are resolved with this priority (highest last):

1. `defaults`
2. `group`
3. `environment`
4. `server`

Connection mode inheritance follows the same chain.

## `includes` (Multi-file Configuration)

Use `includes` to split configuration by team, perimeter, or environment:

```yaml
includes:
  - label: "DEV"
    path: "~/.susshi_dev.yml"
  - label: "QUALIF"
    path: "~/.susshi_qualif.yml"
    merge_defaults: true
```

Fields:

- `label`: displayed as a namespace header in the TUI.
- `path`: absolute path, `~`-expanded path, or a `https://` / `http://` URL.
- `merge_defaults` (optional, default: `false`): merge main-file defaults as base values for the included file.

### HTTPS includes

`path` can be a full HTTPS URL. susshi will fetch the file over TLS at startup and parse it like any local include:

```yaml
includes:
  - label: "SHARED"
    path: "https://inventory.example.com/team-servers.yml"
    merge_defaults: true
```

Behavior:

- The URL is fetched synchronously at startup (same point as file includes).
- HTTP errors and network failures are non-fatal — emitted as `LoadError` warnings.
- The downloaded content is validated with the same unknown-field checks as local files.
- Recursive includes inside a URL-fetched file use normal local-path resolution from the process working directory.

Behavior (all includes):

- Includes are resolved recursively.
- Circular includes are reported as non-blocking warnings.
- Missing/unreadable files are non-fatal warnings.
- Unknown YAML fields emit non-blocking `ValidationWarning` entries.

## `defaults`

Global values applied unless overridden:

- `user`, `ssh_key`, `ssh_cert`, `ssh_agent_sock`, `ssh_port`, `ssh_options`
- `mode`: `direct`, `jump`, or `wallix`
- `theme`: `latte`, `frappe`, `macchiato`, `mocha`
- `use_system_ssh_config`
- `keep_open`
- `default_filter`
- `agent_forwarding` — enable SSH agent forwarding (`-A` flag); default `false`
- `control_master`, `control_path`, `control_persist`
- `pre_connect_hook`, `post_disconnect_hook`, `hook_timeout_secs`
- `probe_filesystems`
- `jump` and `wallix` mode configuration blocks

### Jump block

`jump` is always a list (even for one host):

```yaml
jump:
  - host: "jump1.example.com"
    user: "jump-user"
  - host: "jump2.example.com"
    user: "jump-user"
```

### Wallix block

```yaml
wallix:
  host: "bastion.example.com"
  user: "bastion-user"
  group: "devops-admins"
  account: "default"
  protocol: "SSH"
  auto_select: true
  fail_if_menu_match_error: true
  selection_timeout_secs: 8
  # template: "{target_user}@%n:SSH:{bastion_user}"  # custom login-string template (default shown)
  # direct: false      # set true to skip menu probe for direct-checkout targets
  # authorization: ""  # force exact Wallix authorization name (skips menu matching)
  # header_columns: ["ID", "Cible", "Autorisation"]  # override if bastion uses different column labels
```

`bastion` is accepted as a backward-compatible alias key.

Wallix-specific fields:

- `template` — Go-style template for the Wallix SSH `User` identity string. Default: `{target_user}@%n:SSH:{bastion_user}`. Override when your bastion expects a different format.
- `direct` — skip the menu probe entirely and attempt a direct checkout connection. Use when the authorization is guaranteed and menu parsing is not needed.
- `authorization` — force the exact Wallix authorization name. When set, susshi skips the menu matching step and uses this value directly.
- `header_columns` — list of tokens used to detect the menu header row. Override if your bastion instance uses localized or custom column names.

## `_vars` and Interpolation

Define per-file variables:

```yaml
_vars:
  env: "prod"
```

Use placeholders in any string field:

```yaml
name: "api-{{ env }}"
```

Rules:

- Scope is file-local (does not leak across includes).
- Undefined variables emit a non-blocking warning.
- Built-in `{{ index }}` expands to the 1-based server position within each list.

## `groups`, `environments`, and `servers`

Top-level inventory is defined in `groups`.

A group can contain:

- `servers` directly, or
- nested `environments`, each with `servers`.

Any level may override defaults.

### Server-specific fields

- `name`, `host`
- `mode`
- `tags`
- `notes` — free-form description shown in the detail panel
- `ssh_key` — path to the private key (tilde expanded)
- `ssh_cert` — path to a signed SSH certificate to pass as a second `-i` alongside `ssh_key`
- `ssh_agent_sock` — path to a Unix socket for a dedicated SSH agent (sets `SSH_AUTH_SOCK` and `-o IdentityAgent=`); useful for GPG-based agents or per-server agent isolation (Unix only)
- `tunnels` (optional predefined local forwards)
- hook overrides (`pre_connect_hook`, `post_disconnect_hook`)

Tunnel entry schema:

```yaml
tunnels:
  - label: "PostgreSQL"
    local_port: 15432
    remote_host: "127.0.0.1"
    remote_port: 5432
```

## Complete Example

See [../examples/full_config.yaml](../examples/full_config.yaml) for a complete annotated configuration.
