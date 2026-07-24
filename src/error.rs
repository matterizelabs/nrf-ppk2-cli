use std::fmt;

#[derive(Debug)]
pub enum Error {
    DeviceNotFound,
    #[allow(dead_code)]
    DeviceBusy(String),
    Disconnected(f64),
    Timeout(String),
    InvalidArg(String),
    #[allow(dead_code)]
    FirmwareMismatch {
        actual: String,
        max: String,
    },
    PartialCapture {
        samples: u64,
        duration: f64,
    },
    PowerNotOn,
    Io(std::io::Error),
    Serial(serialport::Error),
    Other(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DeviceNotFound => write!(f, "device not found"),
            Self::DeviceBusy(s) => write!(f, "device busy: {}", s),
            Self::Disconnected(t) => write!(f, "device disconnected at {:.1}s", t),
            Self::Timeout(s) => write!(f, "device not responding: {}", s),
            Self::InvalidArg(s) => write!(f, "invalid argument: {}", s),
            Self::FirmwareMismatch { actual, max } => {
                write!(
                    f,
                    "firmware {} may be incompatible (tested up to {})",
                    actual, max
                )
            }
            Self::PartialCapture { samples, duration } => {
                write!(f, "partial capture: {} samples ({:.1}s)", samples, duration)
            }
            Self::PowerNotOn => write!(
                f,
                "power must be on (auto_power=never), run 'ppk2 power on' first"
            ),
            Self::Io(e) => write!(f, "{}", e),
            Self::Serial(e) => write!(f, "{}", e),
            Self::Other(s) => write!(f, "{}", s),
        }
    }
}

impl std::error::Error for Error {}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

impl From<serialport::Error> for Error {
    fn from(e: serialport::Error) -> Self {
        Self::Serial(e)
    }
}

impl From<zip::result::ZipError> for Error {
    fn from(e: zip::result::ZipError) -> Self {
        Self::Io(std::io::Error::other(e))
    }
}

impl From<ctrlc::Error> for Error {
    fn from(e: ctrlc::Error) -> Self {
        Self::Other(e.to_string())
    }
}

pub type Result<T> = std::result::Result<T, Error>;
