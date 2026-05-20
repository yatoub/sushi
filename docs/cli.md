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

## Import and Export

See [import-export.md](import-export.md) for the full reference: OpenSSH config import, `--list --json`, `--exec-group`, and all export formats (Ansible, Terraform, Nmap, CSV, OpenSSH).

## Help

```bash
susshi --help
```
