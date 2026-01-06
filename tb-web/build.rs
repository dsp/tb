//! Build script for tb-web.
//!
//! Compiles TypeScript frontend before embedding assets.

use std::env;
use std::path::Path;
use std::process::Command;

fn main() {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let frontend_dir = Path::new(&manifest_dir).join("frontend");

    // Rerun if frontend sources change
    println!("cargo:rerun-if-changed=frontend/src/");
    println!("cargo:rerun-if-changed=frontend/index.html");
    println!("cargo:rerun-if-changed=frontend/style.css");
    println!("cargo:rerun-if-changed=frontend/package.json");

    // Check if node_modules exists, if not run npm install
    let node_modules = frontend_dir.join("node_modules");
    if !node_modules.exists() {
        println!("cargo:warning=Installing frontend dependencies...");
        let status = Command::new("npm")
            .arg("install")
            .current_dir(&frontend_dir)
            .status()
            .expect("Failed to run npm install. Is npm installed?");

        if !status.success() {
            panic!("npm install failed");
        }
    }

    // Run npm build
    println!("cargo:warning=Building frontend...");
    let status = Command::new("npm")
        .arg("run")
        .arg("build")
        .current_dir(&frontend_dir)
        .status()
        .expect("Failed to run npm build. Is npm installed?");

    if !status.success() {
        panic!("npm build failed");
    }
}
