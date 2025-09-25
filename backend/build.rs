use std::env;
use std::process::Command;
use std::path::Path;

fn main() {
    // Build Zig library first
    build_zig_library();
    
    // Then build Tauri
    tauri_build::build()
}

fn build_zig_library() {
    let zig_dir = Path::new("zig-modules");
    let lib_path = zig_dir.join("libmanifest_zig.a");
    
    // Build the Zig static library directly using zig build-lib
    let output = Command::new("zig")
        .args(&[
            "build-lib", 
            "-static", 
            "-O", "ReleaseFast",
            "--name", "manifest_zig",
            "src/lib.zig",
            "-lc"
        ])
        .current_dir(zig_dir)
        .output();

    match output {
        Ok(result) => {
            if !result.status.success() {
                println!("cargo:warning=Zig build-lib failed, using fallback: {}", 
                         String::from_utf8_lossy(&result.stderr));
                println!("cargo:rustc-cfg=feature=\"no_zig\"");
                return;
            }
            
            // Check if the library file was created
            if !lib_path.exists() {
                println!("cargo:warning=Zig library not created, using fallback");
                println!("cargo:rustc-cfg=feature=\"no_zig\"");
                return;
            }
            
            // Link the generated static library
            println!("cargo:rustc-link-search=native={}/zig-modules", 
                     env::var("CARGO_MANIFEST_DIR").unwrap());
            println!("cargo:rustc-link-lib=static=manifest_zig");
            println!("cargo:warning=Zig SIMD optimizations enabled");
        }
        Err(_) => {
            // Fallback: compile without Zig optimizations
            println!("cargo:warning=Zig not found, compiling without SIMD optimizations");
            println!("cargo:rustc-cfg=feature=\"no_zig\"");
        }
    }
    
    // Rerun if Zig files change
    println!("cargo:rerun-if-changed=zig-modules/");
}
