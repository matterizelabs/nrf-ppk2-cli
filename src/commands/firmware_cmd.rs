use crate::error::Result;
use crate::firmware;

pub fn run_info(_json: bool, port: Option<&str>, serial: Option<&str>) -> Result<()> {
    firmware::firmware_info(port, serial)
}
