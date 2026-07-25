use std::process::{Command, Output};
use std::sync::{Mutex, OnceLock};

static DEVICE_LOCK: Mutex<()> = Mutex::new(());

fn ppk2(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_ppk2"))
        .args(args)
        .output()
        .expect("ppk2 command failed")
}

fn ppk2_ok(args: &[&str]) -> Output {
    let out = ppk2(args);
    if !out.status.success() {
        panic!(
            "ppk2 {} failed ({}):\n{}",
            args.join(" "),
            out.status,
            String::from_utf8_lossy(&out.stderr)
        );
    }
    out
}

fn stdout(out: &Output) -> String {
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

fn device_connected() -> bool {
    static CONNECTED: OnceLock<bool> = OnceLock::new();
    *CONNECTED.get_or_init(|| {
        let out = ppk2(&["list"]);
        out.status.success()
            && !String::from_utf8_lossy(&out.stdout).contains("no PPK2")
            && !String::from_utf8_lossy(&out.stdout).trim().is_empty()
    })
}

fn require_device() -> std::sync::MutexGuard<'static, ()> {
    let guard = DEVICE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    if !device_connected() {
        drop(guard);
        panic!("SKIP: no PPK2 device connected");
    }
    guard
}

fn device_serial() -> String {
    let out = ppk2(&["list"]);
    stdout(&out)
        .split_whitespace()
        .next()
        .unwrap_or("unknown")
        .to_string()
}

#[test]
fn list_shows_device() {
    let _guard = require_device();
    let out = ppk2_ok(&["list"]);
    let text = stdout(&out);
    assert!(!text.is_empty(), "list output empty");
    assert!(
        !text.contains("unknown"),
        "serial should not be unknown: {}",
        text
    );
    assert!(
        text.contains("/dev/"),
        "should show device port: {}",
        text
    );
}

#[test]
fn list_json_has_serial_and_ports() {
    let _guard = require_device();
    let out = ppk2_ok(&["list", "--json"]);
    let text = stdout(&out);
    assert!(text.contains("\"serial\""), "json missing serial: {}", text);
    assert!(text.contains("\"port\""), "json missing port: {}", text);
}

#[test]
fn mode_set_and_read() {
    let _guard = require_device();
    let sn = device_serial();
    ppk2_ok(&["--serial", &sn, "mode", "source"]);
    ppk2_ok(&["--serial", &sn, "mode", "ampere"]);
    ppk2_ok(&["--serial", &sn, "mode", "source"]);
}

#[test]
fn voltage_set() {
    let _guard = require_device();
    let sn = device_serial();
    ppk2_ok(&["--serial", &sn, "mode", "source"]);
    ppk2_ok(&["--serial", &sn, "voltage", "3300"]);
    ppk2_ok(&["--serial", &sn, "voltage", "3000"]);
    ppk2_ok(&["--serial", &sn, "voltage", "3300"]);
}

#[test]
fn power_toggle() {
    let _guard = require_device();
    let sn = device_serial();
    ppk2_ok(&["--serial", &sn, "mode", "source"]);
    ppk2_ok(&["--serial", &sn, "voltage", "3300"]);
    ppk2_ok(&["--serial", &sn, "power", "on"]);
    ppk2_ok(&["--serial", &sn, "power", "off"]);
    ppk2_ok(&["--serial", &sn, "power", "on"]);
}

#[test]
fn measure_produces_samples() {
    let _guard = require_device();
    let sn = device_serial();
    ppk2_ok(&["--serial", &sn, "mode", "source"]);
    ppk2_ok(&["--serial", &sn, "voltage", "3300"]);
    ppk2_ok(&["--serial", &sn, "power", "on"]);

    let out = ppk2_ok(&["--serial", &sn, "measure", "--duration", "1"]);
    let text = stdout(&out);
    assert!(text.contains("samples"), "missing samples: {}", text);
    assert!(text.contains("avg"), "missing avg: {}", text);
}

#[test]
fn measure_json_output() {
    let _guard = require_device();
    let sn = device_serial();
    ppk2_ok(&["--serial", &sn, "mode", "source"]);
    ppk2_ok(&["--serial", &sn, "voltage", "3300"]);
    ppk2_ok(&["--serial", &sn, "power", "on"]);

    let out = ppk2_ok(&["--serial", &sn, "measure", "--duration", "1", "--json"]);
    let text = stdout(&out);
    assert!(text.contains("\"avg_ua\""), "json missing avg_ua: {}", text);
    assert!(
        text.contains("\"samples\""),
        "json missing samples: {}",
        text
    );
}

#[test]
fn measure_save_and_verify() {
    let tmp = "/tmp/ppk2_test_save.ppk2";
    let _ = std::fs::remove_file(tmp);

    let _guard = require_device();
    let sn = device_serial();
    ppk2_ok(&["--serial", &sn, "mode", "source"]);
    ppk2_ok(&["--serial", &sn, "voltage", "3300"]);
    ppk2_ok(&["--serial", &sn, "power", "on"]);

    ppk2_ok(&["--serial", &sn, "measure", "--duration", "1", "--save", tmp]);

    let out = ppk2_ok(&["info", tmp]);
    let text = stdout(&out);
    assert!(text.contains("samples"), "info missing samples: {}", text);
    assert!(text.contains("duration"), "info missing duration: {}", text);

    let _ = ppk2_ok(&["convert", tmp, "--output", "/tmp/ppk2_test.csv"]);
    let csv = std::fs::read_to_string("/tmp/ppk2_test.csv").expect("csv not written");
    assert!(csv.contains("timestamp_us"), "csv missing header");
    assert!(csv.lines().count() > 1, "csv has no data rows");

    let _ = std::fs::remove_file(tmp);
    let _ = std::fs::remove_file("/tmp/ppk2_test.csv");
}

#[test]
fn measure_downsampled() {
    let tmp = "/tmp/ppk2_test_ds.ppk2";
    let _ = std::fs::remove_file(tmp);

    let _guard = require_device();
    let sn = device_serial();
    ppk2_ok(&["--serial", &sn, "mode", "source"]);
    ppk2_ok(&["--serial", &sn, "voltage", "3300"]);
    ppk2_ok(&["--serial", &sn, "power", "on"]);

    ppk2_ok(&[
        "--serial",
        &sn,
        "measure",
        "--duration",
        "1",
        "--save",
        tmp,
        "--rate",
        "10000",
    ]);

    let out = ppk2_ok(&["info", tmp]);
    let text = stdout(&out);
    assert!(text.contains("duration"), "info missing duration: {}", text);

    let out = ppk2_ok(&["report", tmp]);
    let text = stdout(&out);
    assert!(text.contains("samples"), "report missing samples: {}", text);

    let _ = std::fs::remove_file(tmp);
}

#[test]
fn info_on_saved_file() {
    let tmp = "/tmp/ppk2_test_info.ppk2";
    let _ = std::fs::remove_file(tmp);

    let _guard = require_device();
    let sn = device_serial();
    ppk2_ok(&["--serial", &sn, "mode", "source"]);
    ppk2_ok(&["--serial", &sn, "voltage", "3300"]);
    ppk2_ok(&["--serial", &sn, "power", "on"]);
    ppk2_ok(&["--serial", &sn, "measure", "--duration", "1", "--save", tmp]);

    let out = ppk2_ok(&["info", tmp]);
    let text = stdout(&out);
    assert!(text.contains("avg"), "info missing avg: {}", text);
    assert!(text.contains("charge"), "info missing charge: {}", text);

    let out = ppk2_ok(&["info", tmp, "--json"]);
    let text = stdout(&out);
    assert!(
        text.contains("\"avg_ua\""),
        "info json missing avg_ua: {}",
        text
    );

    let _ = std::fs::remove_file(tmp);
}

#[test]
fn firmware_info() {
    let _guard = require_device();
    let sn = device_serial();
    let out = ppk2_ok(&["--serial", &sn, "firmware", "info"]);
    let text = stdout(&out);
    assert!(!text.is_empty(), "firmware info empty");
}

#[test]
fn trigger_fires_on_threshold() {
    let _guard = require_device();
    let sn = device_serial();
    ppk2_ok(&["--serial", &sn, "mode", "source"]);
    ppk2_ok(&["--serial", &sn, "voltage", "3300"]);
    ppk2_ok(&["--serial", &sn, "power", "on"]);

    let out = ppk2_ok(&[
        "--serial",
        &sn,
        "trigger",
        "--threshold",
        "100",
        "--edge",
        "rising",
    ]);
    let text = stdout(&out);
    assert!(
        text.contains("trigger") || text.contains("fired") || text.contains("interrupted"),
        "trigger output unexpected: {}",
        text
    );
}

#[test]
fn report_multiple_files() {
    let tmp1 = "/tmp/ppk2_test_a.ppk2";
    let tmp2 = "/tmp/ppk2_test_b.ppk2";
    let _ = std::fs::remove_file(tmp1);
    let _ = std::fs::remove_file(tmp2);

    let _guard = require_device();
    let sn = device_serial();
    ppk2_ok(&["--serial", &sn, "mode", "source"]);
    ppk2_ok(&["--serial", &sn, "voltage", "3300"]);
    ppk2_ok(&["--serial", &sn, "power", "on"]);
    ppk2_ok(&[
        "--serial",
        &sn,
        "measure",
        "--duration",
        "1",
        "--save",
        tmp1,
    ]);
    ppk2_ok(&[
        "--serial",
        &sn,
        "measure",
        "--duration",
        "1",
        "--save",
        tmp2,
    ]);

    let out = ppk2_ok(&["report", tmp1, tmp2]);
    let text = stdout(&out);
    assert!(!text.is_empty(), "report output empty");

    let out = ppk2_ok(&["report", tmp1, tmp2, "--json"]);
    let text = stdout(&out);
    assert!(
        text.contains("\"file\""),
        "report json missing file field: {}",
        text
    );

    let _ = std::fs::remove_file(tmp1);
    let _ = std::fs::remove_file(tmp2);
}
