# 寿司 sushi : ssh user settings hosts import

![sushi](assets/sushi.png)<br>

A ssh connection manager with ProxyJump. This is a free to use application and does not comes with any warranty. 

![GitHub](https://img.shields.io/github/license/yatoub/sushi)
![GitHub go.mod Go version](https://img.shields.io/github/go-mod/go-version/yatoub/sushi)
![GitHub code size in bytes](https://img.shields.io/github/languages/code-size/yatoub/sushi)
![GitHub Workflow Status](https://img.shields.io/github/workflow/status/yatoub/sushi/goreleaser)
![GitHub release (latest by date)](https://img.shields.io/github/v/release/yatoub/sushi)



## Install

Download binary from [releases](//github.com/yatoub/sushi/releases) and extract it where you want.

## Configuration

config file load in following order:

- `~/.sushi`
- `~/.sushi.yml`
- `~/.sushi.yaml`
- `./.sushi`
- `./.sushi.yml`
- `./.sushi.yaml`

config example:

<!-- prettier-ignore -->
```yaml
- { name: dev server with proxy and keypath as anchor, user: &user appuser, host: &proxy 192.168.1.2, port: 22, keypath: &key /path/to/id_rsa }
- { name: dev server with passphrase key, user: appuser, host: 192.168.8.35, port: 22, keypath: /root/.ssh/id_rsa, passphrase: abcdefghijklmn}
- { name: dev server without port, user: appuser, host: 192.168.8.35 }
- { name: dev server without user, host: 192.168.8.35 }
- { name: dev server without password, host: 192.168.8.35 }
- { name: ⚡️ server with emoji name, host: 192.168.8.35 }
- { name: server with alias, alias: dev, host: 192.168.8.35 }


# server group 1
- name: server group 1
  user: *user
  keypath: *key
  proxyhost: *proxy
  children:
  - { name: server 1, host: 192.168.1.3 }
  - { name: server 2, host: 192.168.1.4 }
  - { name: server 3, host: 192.168.1.5 }

# server group 2
- name: server group 2
  keypath: *key
  proxyhost: *proxy
  children:
  - { name: server 1, user: root, host: 192.168.2.3 }
  - { name: server 2, user: root, host: 192.168.3.4 }
  - { name: server 3, user: root, host: 192.168.4.5 }
```
