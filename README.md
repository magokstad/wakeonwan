
# Wake‑On‑WAN

**wakeonwan** – a lightweight, cross‑platform command‑line tool for sending Wake‑On‑LAN magic packets over a network.  
Written in Rust, it supports both IPv4 and IPv6 destinations and allows you to wake one or more devices by MAC address.

## Installation From Source

```bash
# Clone the repository
git clone https://github.com/magokstad/wakeonwan.git
cd wakeonwan

# Build (release mode)
cargo build --release

# The binary will be in ./target/release/wakeonwan
# Optionally, move it to a location in your PATH
sudo cp target/release/wakeonwan /usr/local/bin/
```

## Usage

To see basic usage of the command run `wakeonwan -h`

### Basic Command

Send a Wake‑On‑LAN packet to a single device:

```bash
wakeonwan -i 192.168.1.255 -p 9 00:11:22:33:44:55
```

The default destination is the broadcast address `255.255.255.255` on port `9`, so the most minimal command is:

```bash
wakeonwan 00:11:22:33:44:55
```

### Options

| Flag | Argument | Default | Description |
|------|----------|---------|-------------|
| `-i, --uri` | `URI` | `255.255.255.255` | Destination IP/IPv6 address or hostname. |
| `-p, --port` | `PORT` | `9` | Destination UDP port. |
| `-v, --verbose` | — | — | Enable debug logging (`env_logger`). |
| `-D, --dry-run` | — | — | Do not actually send the packet; just print what would be sent. |
| `-h, --help` | — | — | Print help information. |

> The `uri` argument is parsed as an `http::Uri`. Only the host part is used, so you can pass URLs such as `http://my-router.local/` or IPv6 addresses wrapped in `[]` (e.g., `[fe80::1]`).

### Examples

#### Wake multiple devices

```bash
wakeonwan 00:11:22:33:44:55 66:77:88:99:AA:BB
```

#### Use a hostname (resolved by DNS)

```bash
wakeonwan -i my-router.local 00:11:22:33:44:55
```

#### Send to a specific port

```bash
wakeonwan -p 23 00:11:22:33:44:55
```

#### Dry‑run (no packet sent)

```bash
wakeonwan -D 00:11:22:33:44:55
```

#### Verbose mode

```bash
wakeonwan -v 00:11:22:33:44:55
```
