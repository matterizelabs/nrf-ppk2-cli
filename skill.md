---
name: ppk2-cli
description: Use for controlling Nordic Power Profiler Kit II via the ppk2 CLI. Covers device setup, measurement (live status, downsampling, JSON output), analog trigger, daemon mode with background spawn, firmware info, file operations (info/report/convert), autosave recovery, config management, and common workflows.
---

# ppk2 CLI

CLI for Nordic Power Profiler Kit II — measures sub-uA to 600mA current with 100ksps sampling.

Real PPK2 device at `/dev/ttyACM0` (serial `F057566F0FD6`).

## Global flags

| Flag | Description |
|------|-------------|
| `-p, --port` | Serial port path (e.g. `/dev/ttyACM0`) |
| `-s, --serial` | Device serial number (e.g. `F057566F0FD6`) |
| `--json` | Output machine-readable JSON |

If `--serial` is given, ppk2 auto-discovers the port. If neither flag given, uses first PPK2 found.

## Device setup

### List connected devices
```
ppk2 list
ppk2 list --json
```
Output:
```
F057566F0FD6  /dev/ttyACM0
```
JSON: `[{"serial":"F057566F0FD6","port":"/dev/ttyACM0"}]`

### Set measurement mode
```
ppk2 mode source    # DUT powered by PPK2 (up to ~600mA)
ppk2 mode ampere    # DUT powered externally (measures current only)
```

### Set output voltage
```
ppk2 voltage 3300   # Set source mode VDD to 3.3V
ppk2 voltage 1800   # Set to 1.8V
```
Range: 800mV–5000mV. Only applies in source mode.

### Power control
```
ppk2 power on       # Enable DUT power output
ppk2 power off      # Disable DUT power output
```

## Measurement

```
ppk2 measure                      # Run until Ctrl+C (live stats on stderr)
ppk2 measure --duration 5         # Measure for 5 seconds at 100ksps
ppk2 measure --duration 10 --save out.ppk2
ppk2 measure --duration 5 --json  # JSON summary only
ppk2 measure --rate 1000          # Downsample to 1ksps
ppk2 measure --rate 100 --duration 300 --save slow.ppk2
```

| Flag | Description |
|------|-------------|
| `-d, --duration` | Seconds to measure (omitted = run until Ctrl+C) |
| `--save` | Save to .ppk2 file |
| `-r, --rate` | Downsample to N samples/sec (default: 100000). Rates not dividing 100000 evenly are rounded to nearest divisor |

Live status line updates in-place every 500ms on stderr:
```
2.5s  avg 41.9uA  138.4uW
```

Text summary (stdout):
```
duration 5.0s  samples 500000  avg 42.3uA  charge 0.059uAh  power 140uW
saved /home/amac/.local/share/ppk2/autosave/F057566F0FD6/ppk2-1784911444.ppk2
```

JSON summary:
```json
{"duration_s":5.0,"samples":500000,"avg_ua":42.300,"charge_uah":0.058750,"power_uw":139.6,"min_ua":1.200,"max_ua":15000.000}
```

### Interpreting output
- `avg_ua`: Mean current in microamps over measurement window
- `charge_uah`: Total charge consumed in microamp-hours (avg × duration / 3600)
- `power_uw`: Power in microwatts (voltage × avg current / 1000), only shown in source mode
- `samples`: Total sample count (at chosen rate, not raw 100ksps)
- `min_ua` / `max_ua`: Min/max current observed

### Auto-power behavior
Controlled by config (`~/.config/ppk2/config.toml`):
```
[behavior]
auto_power = "session"  # on during measure, off after (default)
# auto_power = "never"  # manual power control
# auto_power = "always" # keep power on
```

Current limit warnings: >400mA in source mode warns to connect both USB ports; >580mA warns to switch to ampere mode.

## Trigger (analog threshold)

Capture a burst/spike with pre/post-trigger buffering at full 100ksps.

```
ppk2 trigger --threshold 5000 --edge rising
ppk2 trigger --threshold 1000 --edge falling --pre-trigger 200 --post-trigger 500
ppk2 trigger --threshold 100 --edge both --save spike.ppk2
```

Parameters:
- `-t, --threshold`: Current threshold in uA
- `-e, --edge`: `rising`, `falling`, or `both` (default: `rising`)
- `--pre-trigger`: Milliseconds before trigger point (default: 100)
- `--post-trigger`: Milliseconds after trigger point (default: 1000)
- `--save`: Save captured data to .ppk2 file

Output:
```
trigger fired at 10000 samples  captured 110000 samples  duration 2.1s  avg 5003.2uA  power 16511uW
```

## Daemon

Long-running background measurement. Uses `Command::spawn` (no fork) — parent exits immediately, child binds socket and measures in background.

```
ppk2 daemon start                   # Background, prints socket path + PID
ppk2 daemon status                  # Realtime stats as JSON
ppk2 daemon stop                    # Stop and finalize autosave
ppk2 daemon stop --save out.ppk2    # Stop with named .ppk2 file
ppk2 daemon start --rate 100        # Background daemon at 100sps
```

Socket at `~/.local/state/ppk2/<serial>/daemon.sock`. Pidfile at `daemon.pid`. Stderr logged to `daemon.log`.

Status response:
```json
{"elapsed_s":2.5,"samples":228863,"avg_ua":41337.8,"min_ua":-1.4,"max_ua":62631.1}
```

On stop, daemon prints measurement summary to the log file. Autosave writes to disk in paged buffers during measurement (bounded memory, no RAM accumulation).

## Firmware

```
ppk2 firmware info
```

Outputs firmware version and calibration status:
```
firmware: PCA63100 v1.2.4-db16a94 (calibrated)
firmware: 2161 (uncalibrated)
```

## File operations

### File info
```
ppk2 info capture.ppk2
ppk2 info capture.ppk2 --json
```
Reads a .ppk2 file and prints summary (duration, samples, avg current, charge).

### Multi-file report
```
ppk2 report capture1.ppk2 capture2.ppk2
ppk2 report *.ppk2 --json
```
Summary for one or more .ppk2 files (line-delimited JSON).

### Convert to CSV
```
ppk2 convert capture.ppk2                   # → capture.csv
ppk2 convert capture.ppk2 --output /tmp/out.csv
```
Output columns: `timestamp_us,current_ua,D0,D1,D2,D3,D4,D5,D6,D7`.

## Config

Optional file at `~/.config/ppk2/config.toml`. Defaults used if absent.

```
ppk2 config show        # Print current effective config
ppk2 config init        # Create config file with defaults
```

```toml
[defaults]
mode = "source"
voltage_mv = 3300

[behavior]
auto_power = "session"

[autosave]
enabled = true
interval_s = 30
```

Env var overrides: `PPK2_VOLTAGE`, `PPK2_MODE`, `PPK2_AUTOSAVE_DIR`, `PPK2_PORT`.

## Autosave & recovery

Autosave writes session data to a temp raw file in paged buffers (10k frames each, ~60KB pages). Memory bounded regardless of session length. On exit, .ppk2 ZIP archive is created from the raw file.

```
ppk2 recover                          # List orphaned autosaves for all devices
ppk2 recover --serial F057566F0FD6    # List for specific device
ppk2 recover --json                   # JSON output
```

Lost data after crash or disconnect can be recovered from autosave directory.

## Error codes

| Code | Exit | Meaning |
|------|------|---------|
| USER_ERROR | 1 | Device not found, bad args, power not on |
| DEVICE_ERROR | 2 | Device disconnected, timeout |
| INTERNAL_ERROR | 3 | Unexpected internal errors |

## Common workflows

### Measure sleep current of a BLE device
```bash
ppk2 mode source
ppk2 voltage 3000
ppk2 power on
ppk2 measure --rate 100 --duration 300 --save sleep.ppk2
ppk2 info sleep.ppk2
```

### Capture TX burst spike
```bash
ppk2 mode source
ppk2 voltage 3300
ppk2 power on
ppk2 trigger --threshold 15000 --edge rising --pre-trigger 50 --post-trigger 200 --save tx_spike.ppk2
ppk2 convert tx_spike.ppk2 --output tx_spike.csv
```

### Long-term background logging
```bash
ppk2 daemon start --rate 10
# ... hours/days pass ...
ppk2 daemon status
ppk2 daemon stop --save long_run.ppk2
ppk2 info long_run.ppk2
```

### Full workflow with specific device
```bash
export PPK2="--serial F057566F0FD6"
ppk2 list
ppk2 $PPK2 mode source
ppk2 $PPK2 voltage 3300
ppk2 $PPK2 power on
ppk2 $PPK2 measure --duration 10 --json
```
