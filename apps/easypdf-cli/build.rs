//! Windows only: embed version info (VERSIONINFO) into `easypdf-cli.exe`, so Explorer's
//! Details tab, `(Get-Item easypdf-cli.exe).VersionInfo` and inventory tools can read the version
//! and publisher, as they can for `easypdf.exe` (`apps/easypdf/build.rs`).
//!
//! On every other target this does nothing. A missing resource compiler is a warning, so a
//! cross-compile from macOS or Linux still links, unless `EASYPDF_REQUIRE_WINRES=1` turns it
//! into an error (for release builds).

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=EASYPDF_REQUIRE_WINRES");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    let mut res = winresource::WindowsResource::new();
    res.set("ProductName", "easyPDF")
        .set("FileDescription", "easyPDF command-line tool")
        .set("LegalCopyright", "MIT OR Apache-2.0; third-party notices in NOTICE.")
        .set("OriginalFilename", "easypdf-cli.exe")
        .set("InternalName", "easypdf-cli");
    if let Err(e) = res.compile() {
        if std::env::var_os("EASYPDF_REQUIRE_WINRES").is_some() {
            println!("cargo::error=embedding Windows resources failed: {e}");
            return;
        }
        println!("cargo:warning=easypdf-cli.exe built without version resources: {e}");
    }
}
