use std::fs;
use std::path::Path;

fn main() {
    let drive = "Y:\\";
    println!("Testing access to {}", drive);

    // Test 1: Does the path exist?
    match fs::metadata(drive) {
        Ok(meta) => {
            println!("metadata OK: is_dir={}", meta.is_dir());
        }
        Err(e) => {
            println!("metadata FAILED: {}", e);
        }
    }

    // Test 2: Read directory
    match fs::read_dir(drive) {
        Ok(entries) => {
            println!("read_dir OK, entries:");
            for e in entries {
                match e {
                    Ok(e) => {
                        println!("  - {:?}", e.file_name());
                    }
                    Err(e) => {
                        println!("  - error reading entry: {}", e);
                    }
                }
            }
        }
        Err(e) => {
            println!("read_dir FAILED: {}", e);
        }
    }

    // Test 3: Try the UNC path
    let unc = "\\\\?\\Y:\\";
    println!("\nTesting UNC path: {}", unc);
    match fs::read_dir(unc) {
        Ok(entries) => {
            println!("read_dir UNC OK, entries:");
            for e in entries {
                match e {
                    Ok(e) => {
                        println!("  - {:?}", e.file_name());
                    }
                    Err(e) => {
                        println!("  - error reading entry: {}", e);
                    }
                }
            }
        }
        Err(e) => {
            println!("read_dir UNC FAILED: {}", e);
        }
    }
}
