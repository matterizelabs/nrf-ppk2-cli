# ppk2

CLI for Nordic Power Profiler Kit II.

## Install

```
nix develop -c cargo build --release
```

Binary at `target/release/ppk2`.

## Quick Start

```
ppk2 list
ppk2 mode source
ppk2 voltage 3300
ppk2 power on
ppk2 measure --duration 5
ppk2 measure --duration 5 --json
```
