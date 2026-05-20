# Hooks

susshi can execute shell scripts before and after each SSH connection.

## Configuration

Set hooks at any config level (defaults, group, environment, server). Lower levels override higher ones.

```yaml
defaults:
  pre_connect_hook: "~/.config/susshi/hooks/pre_connect.sh"
  post_disconnect_hook: "~/.config/susshi/hooks/post_disconnect.sh"
  hook_timeout_secs: 5
```

| Field | Description |
| --- | --- |
| `pre_connect_hook` | Path to script executed before the SSH connection opens |
| `post_disconnect_hook` | Path to script executed after the SSH connection closes |
| `hook_timeout_secs` | Max seconds to wait for a hook to complete (default: 5) |

A non-zero exit code from `pre_connect_hook` **cancels the connection**.

## Environment Variables

The following variables are set when the hook runs:

| Variable | Value |
| --- | --- |
| `SUSSHI_SERVER` | Server name as declared in the config |
| `SUSSHI_HOST` | Resolved hostname or IP |
| `SUSSHI_USER` | SSH user |
| `SUSSHI_PORT` | SSH port |
| `SUSSHI_MODE` | Connection mode: `direct`, `jump`, or `wallix` |

## Examples

**Bash — log connections:**

```bash
#!/usr/bin/env bash
echo "$(date -Iseconds) connect $SUSSHI_USER@$SUSSHI_HOST ($SUSSHI_SERVER)" \
  >> ~/.local/share/susshi/connections.log
```

**Fish — notify on connect:**

```fish
#!/usr/bin/env fish
notify-send "susshi" "Connecting to $SUSSHI_SERVER ($SUSSHI_HOST)"
```

**Pre-connect guard — block production servers outside business hours:**

```bash
#!/usr/bin/env bash
if [[ "$SUSSHI_MODE" == "wallix" ]]; then
  hour=$(date +%H)
  if (( hour < 8 || hour > 18 )); then
    echo "Production connections blocked outside 08:00–18:00" >&2
    exit 1
  fi
fi
exit 0
```

Make hook scripts executable: `chmod +x ~/.config/susshi/hooks/*.sh`
