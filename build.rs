use std::{env, path::PathBuf};
use winresource::WindowsResource;

fn main() {
    println!(
        "cargo:rustc-env=BUILD_TARGET={}",
        env::var("TARGET").unwrap_or_default()
    );

    // Only set the icon if building for Windows
    if env::var_os("CARGO_CFG_WINDOWS").is_some() {
        let manifest_dir = match env::var("CARGO_MANIFEST_DIR") {
            Ok(dir) => dir,
            Err(e) => {
                println!("Build Warning: Could not read CARGO_MANIFEST_DIR: {}", e);
                return;
            }
        };
        let icon_path = PathBuf::from(manifest_dir).join("budget_tracker_icon.ico");
        if icon_path.exists() {
            WindowsResource::new()
                .set_icon(match icon_path.to_str() {
                    Some(s) => s,
                    None => {
                        println!(
                            "Build Warning: Icon path contains invalid unicode, skipping icon"
                        );
                        return;
                    }
                })
                .compile()
                .expect("Failed to compile Windows resources");
            println!("Build: Windows icon set successfully from {:?}.", icon_path);
        } else {
            println!(
                "Build Warning: budget_tracker_icon.ico not found at {:?}, skipping icon setting.",
                icon_path
            );
        }
    }
}
