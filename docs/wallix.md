# Wallix Guide

This page describes how susshi behaves in `wallix` mode.

## Basic Configuration

```yaml
defaults:
  wallix:
    host: "bastion.example.com"
    user: "bastion"
    group: "devops-admins"
    account: "default"
    protocol: "SSH"
    auto_select: true
    fail_if_menu_match_error: true
    selection_timeout_secs: 8
```

Enable Wallix on a server:

```yaml
- name: "internal-nas"
  host: "192.168.1.200"
  mode: "wallix"
```

## Group Inheritance and Override

`wallix.group` follows the same inheritance model as other susshi settings.

- If `server.wallix.group` is set, it has priority.
- Otherwise susshi uses the inherited group from environment/group/defaults.
- If no group can be resolved, susshi switches to targeted fallback selection.

Example:

```yaml
defaults:
  wallix:
    host: "ssh.in.phm.education.gouv.fr"
    user: "pcollin"
    group: "ces3s-admins"

groups:
  - name: "Databases"
    servers:
      - name: "pr-ond-bdd07"
        host: "pr-ond-bdd07.onde.in.phm.education.gouv.fr"
        mode: "wallix"
        wallix:
          group: "crtech-admins"
```

## Automatic SSH User Identity

When `mode: wallix` is active, susshi builds the SSH `User` field as:

`<wallix_user>@<target_fqdn>:SSH[:<ENV_GROUP>]:<wallix_user>`

Where:

- `<target_fqdn>` comes from the selected server host.
- `<ENV_GROUP>` is included only when a deterministic authorization group is resolved.
- If resolution is ambiguous, susshi avoids generating a potentially wrong authorization silently.

## Selection Strategy

When `mode: wallix` is active, susshi computes candidates before selecting an ID in the Wallix menu:

- Target candidates: FQDN, short host, and structure-derived aliases.
- Group candidates: configured `wallix.group` plus structure-derived variants.
- Prefixed authorizations are supported (for example, `ST-ANSIBLE_devops-admins` matches `devops-admins`).

## Targeted Fallback Popup

When automatic resolution fails (no match or ambiguity), susshi opens a focused popup instead of exposing the full global menu flow.

- Candidate entries are pre-filtered against the current target server.
- Search starts from the target hostname to reduce noise.
- The selected Wallix entry ID is cached for the current session.
- During nominal flow, the global pseudo-TTY menu output stays hidden.

## Pagination and Fallback

- Paginated menus are scanned automatically (`page X/Y`, then `n`).
- If Wallix asks for a secondary prompt like `Adresse cible`, susshi auto-fills the configured server host.
- If no reliable match is found, susshi falls back to manual in-session selection.

## Behavior Controls

- `auto_select`: enable automatic selection attempts.
- `fail_if_menu_match_error`: keep trying (including pagination) before falling back.
- `selection_timeout_secs`: menu parsing timeout budget.

## Troubleshooting

### No matching authorization

- Verify `wallix.group` at defaults/group/environment/server level.
- Ensure the target host in config matches the Wallix entry target naming.
- If still unresolved, use the popup and keep the selected entry for the current session.

### Ambiguous authorization

- Multiple authorizations can match the same server target.
- susshi will not guess silently and opens the targeted popup.
- Set a more specific `wallix.group` on the server to make resolution deterministic.

## Notes

- Wallix mode can be inherited from defaults/group/environment.
- ControlMaster multiplexing is silently disabled in Wallix mode.
