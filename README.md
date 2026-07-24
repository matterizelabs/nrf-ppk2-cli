# ppk2

CLI for Nordic Power Profiler Kit II.

## Install

```
curl -sSL https://raw.githubusercontent.com/matterizelabs/nrf-ppk2-cli/main/ppk2.sh | bash
```

## Remove

```
curl -sSL https://raw.githubusercontent.com/matterizelabs/nrf-ppk2-cli/main/ppk2.sh | bash -s -- remove
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
