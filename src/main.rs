mod autosave;
mod commands;
mod config;
mod conversion;
mod daemon;
mod device;
mod error;
mod fileio;
mod firmware;
mod metadata;
mod parser;
mod protocol;
mod transport;
mod trigger;
mod types;

use clap::{Parser, Subcommand};
use error::Result;

#[derive(Parser)]
#[command(name = "ppk2", about = "Power Profiler Kit II CLI")]
struct Cli {
    #[arg(short = 'p', long, help = "Serial port path")]
    port: Option<String>,

    #[arg(short = 's', long, help = "Device serial number")]
    serial: Option<String>,

    #[arg(long, global = true, help = "Output as JSON")]
    json: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    List,
    #[command(name = "power")]
    Power {
        #[arg(value_enum)]
        state: PowerState,
    },
    Mode {
        #[arg(value_enum)]
        mode: DeviceMode,
    },
    Voltage {
        mv: u16,
    },
    Measure {
        #[arg(short = 'd', long)]
        duration: Option<f64>,
        #[arg(long)]
        save: Option<String>,
    },
    Info {
        file: String,
    },
    Trigger {
        #[arg(short = 't', long)]
        threshold: f64,
        #[arg(short = 'e', long, default_value = "rising")]
        edge: String,
        #[arg(long, default_value = "100")]
        pre_trigger: u64,
        #[arg(long, default_value = "1000")]
        post_trigger: u64,
        #[arg(long)]
        save: Option<String>,
    },
    Report {
        #[arg(required = true)]
        files: Vec<String>,
    },
    #[command(subcommand)]
    Firmware(FirmwareCmd),
    #[command(subcommand)]
    Daemon(DaemonCmd),
    Recover {
        serial: Option<String>,
    },
    Convert {
        #[arg(required = true)]
        file: String,
        #[arg(short = 'o', long)]
        output: Option<String>,
    },
    #[command(name = "spike-filter")]
    SpikeFilter {
        #[arg(value_enum)]
        state: SpikeFilterState,
    },
    #[command(name = "range")]
    Range {
        value: u8,
    },
    #[command(name = "avg-num")]
    AvgNum {
        count: u8,
    },
    #[command(subcommand)]
    SwitchPoint(SwitchPointCmd),
    #[command(name = "cal-set")]
    CalSet {
        range: u8,
        ohms: f32,
    },
    #[command(subcommand)]
    FwTrigger(FwTriggerCmd),
    #[command(name = "trigger-ext")]
    TriggerExt,
    Reset,
}

#[derive(Subcommand)]
enum FirmwareCmd {
    Info,
}

#[derive(Subcommand)]
enum DaemonCmd {
    Start,
    Stop {
        #[arg(long)]
        save: Option<String>,
    },
    Status,
}

#[derive(Subcommand)]
enum FwTriggerCmd {
    Set { ua: u16 },
    Window { val: u8 },
    Interval { val: u8 },
    Single,
    Stop,
}

#[derive(Subcommand)]
enum SwitchPointCmd {
    Down { value: u8 },
    Up { value: u8 },
}

#[derive(clap::ValueEnum, Clone)]
enum SpikeFilterState {
    On,
    Off,
}

#[derive(clap::ValueEnum, Clone)]
enum PowerState {
    On,
    Off,
}

#[derive(clap::ValueEnum, Clone)]
enum DeviceMode {
    Source,
    Ampere,
}

fn main() {
    let cli = Cli::parse();
    let json = cli.json;

    match run(cli) {
        Ok(()) => std::process::exit(0),
        Err(e) => {
            if json {
                let code = match &e {
                    error::Error::DeviceNotFound
                    | error::Error::InvalidArg(_)
                    | error::Error::PowerNotOn => "USER_ERROR",
                    error::Error::DeviceBusy(_)
                    | error::Error::Disconnected(_)
                    | error::Error::Timeout(_)
                    | error::Error::FirmwareMismatch { .. } => "DEVICE_ERROR",
                    _ => "INTERNAL_ERROR",
                };
                eprintln!(r#"{{"error":"{}","code":"{}"}}"#, e, code);
            } else {
                eprintln!("error: {}", e);
            }
            let code = match &e {
                error::Error::DeviceNotFound
                | error::Error::InvalidArg(_)
                | error::Error::PowerNotOn => 1,
                error::Error::DeviceBusy(_)
                | error::Error::Disconnected(_)
                | error::Error::Timeout(_)
                | error::Error::FirmwareMismatch { .. } => 2,
                _ => 3,
            };
            std::process::exit(code);
        }
    }
}

fn run(cli: Cli) -> Result<()> {
    match cli.command {
        Commands::List => commands::list::run(cli.json),
        Commands::Power { state } => {
            let state_str = match state {
                PowerState::On => "on",
                PowerState::Off => "off",
            };
            commands::power::run(
                cli.json,
                state_str,
                cli.port.as_deref(),
                cli.serial.as_deref(),
            )
        }
        Commands::Mode { mode } => {
            let mode_str = match mode {
                DeviceMode::Source => "source",
                DeviceMode::Ampere => "ampere",
            };
            commands::mode::run(
                cli.json,
                mode_str,
                cli.port.as_deref(),
                cli.serial.as_deref(),
            )
        }
        Commands::Voltage { mv } => {
            commands::voltage::run(cli.json, mv, cli.port.as_deref(), cli.serial.as_deref())
        }
        Commands::Measure { duration, save } => commands::measure::run(
            cli.json,
            duration,
            save.as_deref(),
            cli.port.as_deref(),
            cli.serial.as_deref(),
        ),
        Commands::Info { file } => commands::info::run(cli.json, &file),
        Commands::Trigger {
            threshold,
            edge,
            pre_trigger,
            post_trigger,
            save,
        } => commands::trigger_cmd::run(
            cli.json,
            threshold,
            &edge,
            pre_trigger,
            post_trigger,
            save.as_deref(),
            cli.port.as_deref(),
            cli.serial.as_deref(),
        ),
        Commands::Report { files } => commands::report::run(cli.json, &files),
        Commands::Firmware(cmd) => match cmd {
            FirmwareCmd::Info => commands::firmware_cmd::run_info(
                cli.json,
                cli.port.as_deref(),
                cli.serial.as_deref(),
            ),
        },
        Commands::Daemon(cmd) => match cmd {
            DaemonCmd::Start => commands::daemon_cmd::run_start(
                cli.json,
                cli.port.as_deref(),
                cli.serial.as_deref(),
            ),
            DaemonCmd::Stop { save } => {
                commands::daemon_cmd::run_stop(cli.json, save.as_deref(), cli.serial.as_deref())
            }
            DaemonCmd::Status => commands::daemon_cmd::run_status(cli.json, cli.serial.as_deref()),
        },
        Commands::Recover { serial } => commands::recover::run(cli.json, serial.as_deref()),
        Commands::Convert { file, output } => {
            let out = output.unwrap_or_else(|| {
                let f = file.clone();
                if f.ends_with(".ppk2") {
                    f.replace(".ppk2", ".csv")
                } else {
                    f + ".csv"
                }
            });
            crate::fileio::export_csv(&file, &out)
        }
        Commands::SpikeFilter { state } => {
            let on = match state {
                SpikeFilterState::On => true,
                SpikeFilterState::Off => false,
            };
            commands::spike_filter::run(cli.json, on, cli.port.as_deref(), cli.serial.as_deref())
        }
        Commands::Range { value } => {
            commands::range::run(cli.json, value, cli.port.as_deref(), cli.serial.as_deref())
        }
        Commands::AvgNum { count } => {
            commands::avg_num::run(cli.json, count, cli.port.as_deref(), cli.serial.as_deref())
        }
        Commands::SwitchPoint(cmd) => match cmd {
            SwitchPointCmd::Down { value } => commands::switch_point::run_down(
                cli.json,
                value,
                cli.port.as_deref(),
                cli.serial.as_deref(),
            ),
            SwitchPointCmd::Up { value } => commands::switch_point::run_up(
                cli.json,
                value,
                cli.port.as_deref(),
                cli.serial.as_deref(),
            ),
        },
        Commands::CalSet { range, ohms } => commands::cal_set::run(
            cli.json,
            range,
            ohms,
            cli.port.as_deref(),
            cli.serial.as_deref(),
        ),
        Commands::FwTrigger(cmd) => match cmd {
            FwTriggerCmd::Set { ua } => commands::fw_trigger::run_set(
                cli.json,
                ua,
                cli.port.as_deref(),
                cli.serial.as_deref(),
            ),
            FwTriggerCmd::Window { val } => commands::fw_trigger::run_window(
                cli.json,
                val,
                cli.port.as_deref(),
                cli.serial.as_deref(),
            ),
            FwTriggerCmd::Interval { val } => commands::fw_trigger::run_interval(
                cli.json,
                val,
                cli.port.as_deref(),
                cli.serial.as_deref(),
            ),
            FwTriggerCmd::Single => commands::fw_trigger::run_single(
                cli.json,
                cli.port.as_deref(),
                cli.serial.as_deref(),
            ),
            FwTriggerCmd::Stop => {
                commands::fw_trigger::run_stop(cli.json, cli.port.as_deref(), cli.serial.as_deref())
            }
        },
        Commands::TriggerExt => {
            commands::trigger_ext::run(cli.json, cli.port.as_deref(), cli.serial.as_deref())
        }
        Commands::Reset => {
            commands::reset::run(cli.json, cli.port.as_deref(), cli.serial.as_deref())
        }
    }
}
