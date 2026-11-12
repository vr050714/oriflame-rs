//! Embeds the application icon into the Windows executable.

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=assets/icon.ico");
    if std::env::var_os("CARGO_CFG_WINDOWS").is_none() {
        return;
    }
    let mut resource = winresource::WindowsResource::new();
    resource.set_icon("assets/icon.ico");
    // A missing resource compiler must not break the build: the game still
    // sets its window icon at run time.
    if let Err(error) = resource.compile() {
        println!("cargo:warning=cannot embed the executable icon: {error}");
    }
}
