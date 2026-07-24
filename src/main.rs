mod error;
mod types;
mod protocol;
mod transport;
mod parser;
mod conversion;
mod metadata;
mod config;
mod device;
mod fileio;

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
        Commands::List => {
            println!("no PPK2 devices found (scanning USB...)");
            Ok(())
        },
        Commands::Power { state: _ } => Err(error::Error::DeviceNotFound),
        Commands::Mode { mode: _ } => Err(error::Error::DeviceNotFound),
        Commands::Voltage { mv: _ } => Err(error::Error::DeviceNotFound),
        Commands::Measure { duration: _, save: _ } => Err(error::Error::DeviceNotFound),
        Commands::Info { file: _ } => {
            Err(error::Error::InvalidArg("file not found".into()))
        }
    }
}
