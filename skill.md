---
name: ppk2-cli
description: Use for controlling Nordic Power Profiler Kit II via the ppk2 CLI. Covers device setup, measurement (live status, JSON output), analog trigger, daemon mode with IPC, firmware info, file operations (info/report/convert), autosave recovery, config management, and common workflows.
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
Returns serial with control and data ports. Output:
```
F057566F0FD6  control=/dev/ttyACM0  data=/dev/ttyACM1
```
JSON: `[{"serial":"F057566F0FD6","control":"/dev/ttyACM0","data":"/dev/ttyACM1"}]`

### Set measurement mode
```
ppk2 mode source    # DUT powered by PPK2 (up to ~600mA)
ppk2 mode ampere    # DUT powered externally (measures current only)
```
Source mode: PPK2 supplies VDD set by `voltage`. Ampere mode: PPK2 measures current passing through its Ampere Meter jack.

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

### Basic measurement
```
ppk2 measure                      # Run until Ctrl+C (live stats on stderr)
ppk2 measure --duration 5         # Measure for 5 seconds
ppk2 measure --duration 10 --save out.ppk2
ppk2 measure --duration 5 --json  # JSON summary only (no live output)
```

Live status line updates in-place every 500ms on stderr:
```
2.5s  avg 41.9mA  138.4mW
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
- `samples`: Total sample count (100ksps = 100,000 samples/sec)
- `min_ua` / `max_ua`: Min/max current observed

### Saved path
Autosave file is always written to disk (asynchronous, non-blocking). Path printed after summary.
With `--save`, the file is copied to the requested path and the autosave is removed.

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

Capture a burst/spike with pre/post-trigger buffering.

```
ppk2 trigger --threshold 5000 --edge rising                              # Fire when current exceeds 5mA
ppk2 trigger --threshold 1000 --edge falling --pre-trigger 200 --post-trigger 500  # Capture on 1mA fall
ppk2 trigger --threshold 100 --edge both --save spike.ppk2               # Capture any edge crossing 100uA
```

Parameters:
- `-t, --threshold`: Current threshold in uA
- `-e, --edge`: `rising`, `falling`, or `both` (default: `rising`)
- `--pre-trigger`: Samples (ms) before trigger point (default: 100)
- `--post-trigger`: Samples (ms) after trigger point (default: 1000)
- `--save`: Save captured data to .ppk2 file

Output:
```
trigger fired at 10000 samples  captured 110000 samples  duration 2.1s  avg 5003.2uA  power 16511uW
```

## Daemon

Long-running background measurement with realtime IPC via Unix socket.

```
ppk2 daemon start                   # background, prints socket path + PID
ppk2 daemon status                  # realtime stats as JSON
ppk2 daemon stop                    # stop and finalize autosave
ppk2 daemon stop --save out.ppk2    # stop with named .ppk2 file
```

Socket at `~/.local/state/ppk2/<serial>/daemon.sock`.

Status response:
```json
{"elapsed_s":2.5,"samples":228863,"avg_ua":41337.8,"min_ua":-1.4,"max_ua":62631.1}
```

On stop, the daemon prints measurement summary and saved path. Autosave runs in background during measurement (non-blocking periodic flush).

## Firmware

```
ppk2 firmware info
```

Reads firmware version from device metadata. Example: `firmware: 2161`.

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
ppk2 config show --json # Machine-readable
ppk2 config init        # Create config file with defaults
```

```
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

Autosave writes .ppk2 files periodically during measurement (configurable interval, default 30s).
Background writes via `Arc` + thread — never blocks the measurement loop.

```
ppk2 recover                          # List orphaned autosaves for all devices
ppk2 recover --serial F057566F0FD6    # List for specific device
ppk2 recover --json                   # JSON count
```

Lost data after crash or disconnect can be recovered from autosave directory.

## Error codes

| Code | Exit | Meaning |
|------|------|---------|
| USER_ERROR | 1 | Device not found, bad args, power not on |
| DEVICE_ERROR | 2 | Device busy, disconnected, timeout, firmware mismatch |
| INTERNAL_ERROR | 3 | Unexpected internal errors |

## Common workflows

### Measure sleep current of a BLE device
```bash
ppk2 mode source
ppk2 voltage 3000
ppk2 power on
ppk2 measure --duration 30 --save sleep.ppk2
ppk2 info sleep.ppk2
```

### Capture TX burst spike
```bash
ppk2 mode source
ppk2 voltage 3300
ppk2 power on
ppk2 trigger --threshold 15000 --edge rising --pre-trigger 50 --post-trigger 200 --save tx_spike.ppk2
ppk2 info tx_spike.ppk2
```

### Export for analysis
```bash
ppk2 convert tx_spike.ppk2 --output tx_spike.csv
# Import into Excel, Python pandas, or nRF Connect Power Profiler
```

### Long-term logging with daemon
```bash
ppk2 daemon start
# ... hours/days pass ...
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
