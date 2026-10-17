use std::fs;
use std::path::Path;

fn main() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".into());
    let dist_dir = Path::new(&manifest_dir).join("../../packages/desktop-ui/dist");
    if !dist_dir.exists() {
        let _ = fs::create_dir_all(&dist_dir);
    }
    let index_file = dist_dir.join("index.html");
    if !index_file.exists() {
        let _ = fs::write(
            &index_file,
            b"<!DOCTYPE html><html><head><title>Local Content Studio</title></head><body><div id=\"root\"></div></body></html>",
        );
    }
}
