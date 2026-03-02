---
name: Config Help / Question
about: Need help with your susshi config setup?
title: "[CONFIG] "
labels: question, discussion

---

**What are you trying to achieve?**
A clear description of your network topology or use case (e.g. "I want to access server C via jump host B which is inside environment A").

**Current Configuration**
Please share your minimal, sanitized `~/.susshi.yml` (remove any real IPs, users, SSH keys!):
```yaml
# susshi.yml
groups:
  - name: "MyGroup"
    ...
```

**What isn't working?**
- My config parses but the connection fails
- My config fails to parse with error: `Error: ...`
- The feature isn't behaving as documented

**Environment**
 - OS: [e.g. Linux, macOS]
 - susshi Version: [e.g. v1.0]

**Additional context**
Add any other context about the problem here.
