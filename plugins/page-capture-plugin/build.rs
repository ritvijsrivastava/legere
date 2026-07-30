fn main() {
    tauri_plugin::Builder::new(&[]).android_path("android").build();

    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap();
    let mobile = target_os == "android";
    alias("desktop", !mobile);
    alias("mobile", mobile);
}

// creates a cfg alias if `has_feature` is true.
fn alias(alias: &str, has_feature: bool) {
    println!("cargo:rustc-check-cfg=cfg({alias})");
    if has_feature {
        println!("cargo:rustc-cfg={alias}");
    }
}
