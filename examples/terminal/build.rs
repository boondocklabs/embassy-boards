pub fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    println!("cargo:rustc-link-arg-bins=--nmagic");
    println!("cargo:rustc-link-arg-bins=-Tlink.x");
    if let Ok(_) = std::env::var("CARGO_RUNTIME_RP") {
        println!("cargo:warning=Adding link-rp.x");
        println!("cargo:rustc-link-arg-bins=-Tlink-rp.x");
    }
    println!("cargo:rustc-link-arg-bins=-Tdefmt.x");
}
