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

## Selection Strategy

When `mode: wallix` is active, susshi computes candidates before selecting an ID in the Wallix menu:

- Target candidates: FQDN, short host, and structure-derived aliases.
- Group candidates: configured `wallix.group` plus structure-derived variants.
- Prefixed authorizations are supported (for example, `ST-ANSIBLE_devops-admins` matches `devops-admins`).

## Pagination and Fallback

- Paginated menus are scanned automatically (`page X/Y`, then `n`).
- If Wallix asks for a secondary prompt like `Adresse cible`, susshi auto-fills the configured server host.
- If no reliable match is found, susshi falls back to manual in-session selection.

## Behavior Controls

- `auto_select`: enable automatic selection attempts.
- `fail_if_menu_match_error`: keep trying (including pagination) before falling back.
- `selection_timeout_secs`: menu parsing timeout budget.

## Notes

- Wallix mode can be inherited from defaults/group/environment.
- ControlMaster multiplexing is silently disabled in Wallix mode.
