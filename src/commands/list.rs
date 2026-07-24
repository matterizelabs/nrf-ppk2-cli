use crate::error::Result;
use crate::transport::find_ppk2_ports;

pub fn run(json: bool) -> Result<()> {
    let devices = find_ppk2_ports();

    if json {
        let entries: Vec<String> = devices
            .iter()
            .map(|(sn, path)| format!(r#"{{"serial":"{}","path":"{}"}}"#, sn, path))
            .collect();
        println!("[{}]", entries.join(","));
    } else {
        if devices.is_empty() {
            println!("no PPK2 devices found");
        }
        for (sn, path) in &devices {
            println!("{}  {}", sn, path);
        }
    }

    Ok(())
}
