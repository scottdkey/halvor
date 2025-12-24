//! Log tailing functionality

use std::fs::File;
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

const OPENVPN_LOG: &str = "/var/log/openvpn/openvpn.log";
const PRIVOXY_LOG: &str = "/var/log/privoxy/logfile";

/// Tail logs from both OpenVPN and Privoxy
pub fn tail_logs(
    running: std::sync::Arc<AtomicBool>,
    _openvpn_pid: u32,
    _privoxy_pid: u32,
    _config_path: &Path,
) {
    // Wait a moment for log files to be created
    thread::sleep(Duration::from_secs(2));

    let running_openvpn = running.clone();
    let running_privoxy = running.clone();

    // Tail OpenVPN logs
    if Path::new(OPENVPN_LOG).exists() {
        let openvpn_log_path = OPENVPN_LOG.to_string();
        thread::spawn(move || {
            tail_file(&openvpn_log_path, "[OpenVPN] ", running_openvpn);
        });
    }

    // Tail Privoxy logs
    if Path::new(PRIVOXY_LOG).exists() {
        let privoxy_log_path = PRIVOXY_LOG.to_string();
        thread::spawn(move || {
            tail_file(&privoxy_log_path, "[Privoxy] ", running_privoxy);
        });
    }
}

fn tail_file(path: &str, prefix: &str, running: std::sync::Arc<AtomicBool>) {
    // First, read existing content from the end
    if let Ok(mut file) = File::open(path) {
        // Seek to end to start tailing from current position
        if let Ok(_) = file.seek(SeekFrom::End(0)) {
            // Read any remaining content
            let reader = BufReader::new(file);
            for line in reader.lines() {
                if !running.load(Ordering::SeqCst) {
                    return;
                }
                if let Ok(line) = line {
                    println!("{}{}", prefix, line);
                }
            }
        }
    }

    // Now continuously check for new lines
    while running.load(Ordering::SeqCst) {
        if let Ok(mut file) = File::open(path) {
            if let Ok(pos) = file.seek(SeekFrom::End(0)) {
                // Read new content
                if pos > 0 {
                    if let Ok(_) = file.seek(SeekFrom::Start(pos)) {
                        let reader = BufReader::new(file);
                        for line in reader.lines() {
                            if !running.load(Ordering::SeqCst) {
                                return;
                            }
                            if let Ok(line) = line {
                                println!("{}{}", prefix, line);
                            }
                        }
                    }
                }
            }
        }
        thread::sleep(Duration::from_millis(500));
    }
}
