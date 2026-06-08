fn main() {
    // Add local lib directory for winfsp-x64.lib
    // This is a fallback when WinFsp's lib directory doesn't exist
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let local_lib = format!("{}/lib", manifest_dir);
    println!("cargo:rustc-link-search={}", local_lib);
}
