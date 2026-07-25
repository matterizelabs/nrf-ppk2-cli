use crate::error::Result;
use crate::transport::find_ppk2_ports;

pub fn run(json: bool) -> Result<()> {
    let devices = find_ppk2_ports();

    if json {
        let entries: Vec<String> = devices
            .iter()
            .map(|d| {
                format!(
                    r#"{{"serial":"{}","port":"{}"}}"#,
                    d.serial, d.control_port,
                )
            })
            .collect();
        println!("[{}]", entries.join(","));
    } else {
        if devices.is_empty() {
            println!("no PPK2 devices found");
            return Ok(());
        }
        let sn_w = devices.iter().map(|d| d.serial.len()).max().unwrap_or(7);
        for d in &devices {
            println!("{:<sn_w$}  {}", d.serial, d.control_port);
        }
    }

    Ok(())
}
