#[cfg(unix)]
mod unix {
    use crate::device::Ppk2Device;
    use crate::error::Result;
    use crate::config::Config;
    use std::io::{BufRead, BufReader, Write};
    use std::os::unix::net::{UnixListener, UnixStream};
    use std::path::PathBuf;

    pub fn socket_path(serial: &str) -> PathBuf {
        Config::state_dir().join(serial).join("daemon.sock")
    }

    pub fn run_daemon(port_path: &str, serial: &str) -> Result<()> {
        let sock_path = socket_path(serial);
        if let Some(parent) = sock_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let _ = std::fs::remove_file(&sock_path);

        let _listener = UnixListener::bind(&sock_path)?;
        println!("daemon listening on {}", sock_path.display());
        println!("pid: {}", std::process::id());

        let _device = Ppk2Device::open(port_path)?;
        Ok(())
    }

    pub fn send_command(serial: &str, cmd: &str) -> Result<String> {
        let sock_path = socket_path(serial);
        let mut stream = UnixStream::connect(&sock_path)?;
        stream.write_all(cmd.as_bytes())?;
        stream.write_all(b"\n")?;
        let mut reader = BufReader::new(stream);
        let mut response = String::new();
        reader.read_line(&mut response)?;
        Ok(response)
    }
}

#[cfg(windows)]
mod windows {
    use crate::error::Result;
    use std::path::PathBuf;

    pub fn socket_path(serial: &str) -> PathBuf {
        PathBuf::from(format!(r"\\.\pipe\ppk2-{}", serial))
    }

    pub fn run_daemon(port_path: &str, serial: &str) -> Result<()> {
        println!("daemon mode on Windows: connect to {}", socket_path(serial).display());
        println!("pid: {}", std::process::id());
        Ok(())
    }

    pub fn send_command(serial: &str, cmd: &str) -> Result<String> {
        Ok("{}".to_string())
    }
}

#[cfg(unix)]
pub use unix::*;

#[cfg(windows)]
pub use windows::*;
