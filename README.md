# ppk2

CLI for Nordic Power Profiler Kit II.

## Install

```
curl -sSL https://raw.githubusercontent.com/matterizelabs/nrf-ppk2-cli/main/install.sh | bash
```

Install a specific version:

```
curl -sSL https://raw.githubusercontent.com/matterizelabs/nrf-ppk2-cli/main/install.sh | bash -s -- v0.1.0
```

Default install path is `$HOME/.local/bin`. Override with `INSTALL_DIR`:

```
curl -sSL https://raw.githubusercontent.com/matterizelabs/nrf-ppk2-cli/main/install.sh | INSTALL_DIR=/usr/local/bin bash
```

## Remove

```
curl -sSL https://raw.githubusercontent.com/matterizelabs/nrf-ppk2-cli/main/install.sh | bash -s -- remove
```

## Usage

```
ppk2 list
ppk2 mode source
ppk2 voltage 3300
ppk2 power on
ppk2 measure --duration 5
ppk2 measure --duration 5 --json
```

## Development

```
nix develop
cargo build --release
```

Binary at `target/release/ppk2`.
