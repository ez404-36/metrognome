use std::{env, path::PathBuf};

fn main() {
    // The audio-only smoke-test binary does not use GTK assets. Avoid making
    // a glib development tool mandatory in that minimal configuration.
    if env::var_os("CARGO_FEATURE_UI").is_none() {
        return;
    }

    let root = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let resource_xml = root.join("src/app/icons.gresource.xml");
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    let resource_bundle = out_dir.join("icons.gresource");

    let status = std::process::Command::new("glib-compile-resources")
        .arg("--sourcedir")
        .arg(root.join("assets/brand/icons"))
        .arg("--target")
        .arg(&resource_bundle)
        .arg(&resource_xml)
        .status()
        .expect("glib-compile-resources should be available when the ui feature is enabled");

    if !status.success() {
        panic!("failed to compile application icon resources");
    }

    println!("cargo:rerun-if-changed={}", resource_xml.display());
    println!(
        "cargo:rerun-if-changed={}",
        root.join("assets/brand/icons/takto-symbolic.svg").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        root.join("assets/brand/icons/takto-header.svg").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        root.join("assets/brand/icons/takto-256.png").display()
    );
}
