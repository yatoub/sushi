# CLI Guide

susshi can run as a command-line connector without opening the TUI.

## Connect Modes

```bash
# Direct
susshi --direct root@myserver
susshi --direct admin@10.0.1.5:2222

# Jump
susshi --jump root@192.168.1.50

# Wallix
susshi --wallix web-01.prod.example.com
```

## SSH Overrides

```bash
susshi --direct myserver.com --user deploy --port 2222 --key ~/.ssh/deploy_rsa
susshi --direct myserver.com --verbose
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

## Help

```bash
susshi --help
```
