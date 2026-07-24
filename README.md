# ppk2

CLI for Nordic Power Profiler Kit II — measures sub-uA to 600mA at 100ksps.

## Install

```
curl -sSL https://raw.githubusercontent.com/matterizelabs/nrf-ppk2-cli/main/install.sh | bash
```

Install a specific version:

```
curl -sSL https://raw.githubusercontent.com/matterizelabs/nrf-ppk2-cli/main/install.sh | bash -s -- v0.1.2
```

Default install path is `$HOME/.local/bin`. Override with `INSTALL_DIR`:

```
curl -sSL https://raw.githubusercontent.com/matterizelabs/nrf-ppk2-cli/main/install.sh | INSTALL_DIR=/usr/local/bin bash
```

## Remove

```
curl -sSL https://raw.githubusercontent.com/matterizelabs/nrf-ppk2-cli/main/install.sh | bash -s -- remove
```

## Global flags

| Flag | Description |
|------|-------------|
| `-p, --port` | Serial port path (e.g. `/dev/ttyACM0`) |
| `-s, --serial` | Device serial number (auto-discovers port) |
| `--json` | Output machine-readable JSON |

If neither `--port` nor `--serial` given, uses first PPK2 found.

## Usage

### List devices

```
ppk2 list
ppk2 list --json
```

Output:

```
F057566F0FD6  control=/dev/ttyACM0  data=/dev/ttyACM1
```

### Device setup

```
ppk2 mode source       # DUT powered by PPK2
ppk2 mode ampere       # DUT powered externally
ppk2 voltage 3300      # Set VDD to 3.3V (800–5000mV, source mode only)
ppk2 power on          # Enable DUT power
ppk2 power off         # Disable DUT power
```

### Measure

```
ppk2 measure                      # Run until Ctrl+C (live stats on stderr)
ppk2 measure --duration 5         # Measure for 5 seconds
ppk2 measure --duration 10 --save out.ppk2
ppk2 measure --duration 5 --json  # JSON summary only (no live output)
```

Live status line updates in-place every 500ms:

```
2.5s  avg 41.9mA  #250880  138.4mW
```

Text summary:

```
duration 5.0s  samples 500000  avg 42.3uA  charge 0.059uAh  power 140uW
```

JSON summary:

```json
{"duration_s":5.0,"samples":500000,"avg_ua":42.300,"charge_uah":0.058750,"power_uw":139.6,"min_ua":1.200,"max_ua":15000.000}
```

Current limit warnings: >400mA warns to connect both USB ports; >580mA warns to switch to ampere mode.

### Trigger (analog threshold)

Capture spikes with pre/post-trigger buffering:

```
ppk2 trigger --threshold 5000 --edge rising
ppk2 trigger --threshold 1000 --edge falling --pre-trigger 200 --post-trigger 500
ppk2 trigger --threshold 100 --edge both --save spike.ppk2
```

| Flag | Description |
|------|-------------|
| `-t, --threshold` | Current threshold in uA |
| `-e, --edge` | `rising`, `falling`, `both` (default: `rising`) |
| `--pre-trigger` | Samples (ms) before trigger (default: 100) |
| `--post-trigger` | Samples (ms) after trigger (default: 1000) |
| `--save` | Save to .ppk2 file |

### File operations

```
ppk2 info capture.ppk2
ppk2 info capture.ppk2 --json
ppk2 report *.ppk2
ppk2 report *.ppk2 --json
ppk2 convert capture.ppk2                    # → capture.csv
ppk2 convert capture.ppk2 --output out.csv
```

### Daemon

Long-running background measurement with realtime status:

```
ppk2 daemon start                   # background, prints socket path + PID
ppk2 daemon status                  # realtime stats (JSON)
ppk2 daemon stop                    # stop and finalize autosave
ppk2 daemon stop --save out.ppk2    # stop with named .ppk2 file
```

Communicates via Unix socket at `~/.local/state/ppk2/<serial>/daemon.sock`.

### Firmware

```
ppk2 firmware info
```

Reads firmware version from device metadata.

### Autosave & recovery

Config (`~/.config/ppk2/config.toml`):

```
[autosave]
enabled = true
interval_s = 30
dir = "/path/to/autosaves"
```

Recover orphaned autosaves after crash or disconnect:

```
ppk2 recover
ppk2 recover --serial F057566F0FD6
ppk2 recover --json
```

## Config

```
[defaults]
mode = "source"
voltage_mv = 3300

[behavior]
auto_power = "session"   # on during measure, off after (default)
# auto_power = "never"   # manual power control
# auto_power = "always"  # keep power on

[autosave]
enabled = true
interval_s = 30
```

Env var overrides: `PPK2_VOLTAGE`, `PPK2_MODE`, `PPK2_AUTOSAVE_DIR`, `PPK2_PORT`.

## Example workflows

Measure BLE sleep current:

```
ppk2 mode source
ppk2 voltage 3000
ppk2 power on
ppk2 measure --duration 30 --save sleep.ppk2
ppk2 info sleep.ppk2
```

Capture TX current spike:

```
ppk2 mode source
ppk2 voltage 3300
ppk2 power on
ppk2 trigger --threshold 15000 --pre-trigger 50 --post-trigger 200 --save tx.ppk2
ppk2 convert tx.ppk2 --output tx.csv
```

Long-term logging:

```
ppk2 daemon start
# ... hours/days pass ...
ppk2 daemon stop --save long_run.ppk2
```

## Development

```
nix develop
cargo build --release
```
