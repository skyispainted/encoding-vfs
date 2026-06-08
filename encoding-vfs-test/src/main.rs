use std::fs;
use std::io::Write;
use std::path::Path;
use std::thread;
use std::time::Duration;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

fn main() {
    let mut log = String::new();
    let backend = r"C:\projects\fs-fee\fs1-fee-beta-centos-client";

    // Check backend exists
    log.push_str(&format!("Backend exists: {}\n", Path::new(backend).exists()));

    // Create VFS
    let config = encoding_vfs_core::config::EncodingConfig {
        source_encoding: "auto".to_string(),
        target_encoding: "UTF-8".to_string(),
        default_encoding: "GBK".to_string(),
        auto_detect: true,
        detect_sample_bytes: 8192,
        cache_max_entries: 100,
        cache_ttl_seconds: 60,
    };
    let vfs = match encoding_vfs_core::EncodingVfs::new(Path::new(backend), config) {
        Ok(v) => v,
        Err(e) => {
            log.push_str(&format!("Failed to create VFS: {:?}\n", e));
            fs::write("C:\\temp\\test-drive-log.txt", &log).unwrap();
            return;
        }
    };

    log.push_str("VFS created OK\n");

    // Run in a separate thread
    let ready = Arc::new(AtomicBool::new(false));
    let ready_clone = ready.clone();
    let thread_handle = thread::spawn(move || {
        let host = encoding_vfs_windows::WinFspVfsHost::new(vfs);
        // Use drive letter Z to avoid conflict
        let result = encoding_vfs_windows::run_for_test(host, 'Z', ready_clone);
        log.push_str(&format!("Mount thread result: {:?}\n", result));
    });

    // Wait for mount to be ready
    for i in 0..30 {
        thread::sleep(Duration::from_millis(200));
        if ready_clone.load(Ordering::SeqCst) {
            log.push_str(&format!("Mount ready after {}ms\n", (i+1)*200));
            break;
        }
        if i == 29 {
            log.push_str("Mount timed out waiting for ready signal\n");
        }
    }

    thread::sleep(Duration::from_millis(500));

    // Try to access the drive
    let drive = r"Z:\";
    log.push_str(&format!("\n=== Testing access to {} ===\n", drive));

    // Test metadata
    match fs::metadata(drive) {
        Ok(meta) => {
            log.push_str(&format!("metadata OK: is_dir={}\n", meta.is_dir()));
        }
        Err(e) => {
            log.push_str(&format!("metadata FAILED: {}\n", e));
        }
    }

    // Test read_dir
    match fs::read_dir(drive) {
        Ok(entries) => {
            log.push_str("read_dir OK\n");
            for e in entries.take(10) {
                match e {
                    Ok(e) => {
                        log.push_str(&format!("  - {:?}\n", e.file_name()));
                    }
                    Err(e) => {
                        log.push_str(&format!("  - error: {}\n", e));
                    }
                }
            }
        }
        Err(e) => {
            log.push_str(&format!("read_dir FAILED: {}\n", e));
        }
    }

    log.push_str("\n=== Test complete ===\n");
    fs::write("C:\\temp\\test-drive-log.txt", &log).unwrap();

    // Give mount thread time to stop
    thread_handle.join().unwrap();
}
