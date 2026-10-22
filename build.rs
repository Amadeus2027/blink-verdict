fn main() {
    println!("cargo:rerun-if-changed=app.manifest");
    if std::env::var("TARGET").is_ok_and(|target| target.ends_with("windows-msvc")) {
        let manifest = std::path::Path::new(&std::env::var("CARGO_MANIFEST_DIR").unwrap())
            .join("app.manifest");
        println!("cargo:rustc-link-arg-bin=blink-verdict=/MANIFEST:EMBED");
        println!(
            "cargo:rustc-link-arg-bin=blink-verdict=/MANIFESTINPUT:{}",
            manifest.display()
        );
    }
}
