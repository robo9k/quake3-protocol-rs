# quake3-protocol-rs

Inofficial Rust implementation of the Quake 3 network protocol.

> [!NOTE]
> This implementation is incomplete and experimental at best.

You might be interested in [the `dpmaster-rs` project](https://github.com/robo9k/dpmaster-rs) aswell.

## Implementation status

- ❌ id Quake 3 v1.32c - protocol 68
- ❌ ioQuake3 v1.36 - protocol 71

### Packets

#### Connectionless

##### Client

| command                 | serialize | deserialize |
| ----------------------- | :-------: | :---------: |
| `challengeResponse`     | ❌        | ❌         |
| `connectResponse`       | ❌        | ❌         |
| `infoResponse`          | ❌        | ❌         |
| `statusResponse`        | ❌        | ❌         |
| `echo`                  | ❌        | ❌         |
| `print`                 | ❌        | ❌         |
| `keyAuthorize`          | ❌        | ❌         |
| `motd`                  | ❌        | ❌         |
| `getserversResponse`    | ❌        | ❌         |
| `getserversExtResponse` | ❌        | ❌         |

##### Game server

| command                 | serialize | deserialize |
| ----------------------- | :-------: | :---------: |
| `getchallenge`          | ❌        | ❌         |
| `connect`               | ❌        | ❌         |
| `disconnect`            | ❌        | ❌         |
| `getinfo`               | ❌        | ❌         |
| `getstatus`             | ❌        | ❌         |
| `ipAuthorize`           | ❌        | ❌         |
| `rcon`                  | ❌        | ❌         |

##### Update server

| command                 | serialize | deserialize |
| ----------------------- | :-------: | :---------: |
| `getmotd`               | ❌        | ❌         |

##### Master server

| command                 | serialize | deserialize |
| ----------------------- | :-------: | :---------: |
| `heartbeat`             | ❌        | ❌         |
| `getservers`            | ❌        | ❌         |
| `getserversExt`         | ❌        | ❌         |

##### Auth server

| command                 | serialize | deserialize |
| ----------------------- | :-------: | :---------: |
| `getKeyAuthorize`       | ❌        | ❌         |
| `getIpAuthorize`        | ❌        | ❌         |
| `banUser`               | ❌        | ❌         |

#### Netchan

#### Client

| command                 | serialize | deserialize |
| ----------------------- | :-------: | :---------: |
| `clc_bad`               | ❌        | ❌         |
| `clc_nop`               | ❌        | ❌         |
| `clc_move`              | ❌        | ❌         |
| `clc_moveNoDelta`       | ❌        | ❌         |
| `clc_clientCommand`     | ❌        | ❌         |
| `clc_EOF`               | ❌        | ❌         |
| `clc_voipSpeex`         | ❌        | ❌         |
| `clc_voipOpus`          | ❌        | ❌         |

#### Server

| command                 | serialize | deserialize |
| ----------------------- | :-------: | :---------: |
| `svc_bad`               | ❌        | ❌         |
| `svc_nop`               | ❌        | ❌         |
| `svc_gamestate`         | ❌        | ❌         |
| `svc_configstring`      | ❌        | ❌         |
| `svc_baseline`          | ❌        | ❌         |
| `svc_serverCommand`     | ❌        | ❌         |
| `svc_download`          | ❌        | ❌         |
| `svc_snapshot`          | ❌        | ❌         |
| `svc_EOF`               | ❌        | ❌         |
| `svc_voipSpeex`         | ❌        | ❌         |
| `svc_voipOpus`          | ❌        | ❌         |
