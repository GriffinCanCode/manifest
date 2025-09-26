//! ManifestRustTS Build Script
//!
//! Orchestrates the build process between Rust (Cargo) and Zig components,
//! ensuring optimal integration and cross-platform compatibility.
//!
//! Build Architecture:
//! 1. Zig Math/SIMD Library → Static Archive (.a)
//! 2. Rust FFI Integration → Native Linking  
//! 3. Tauri Desktop Application → Final Binary
//!
//! This build script prioritizes:
//! - Performance: Uses optimized Zig SIMD operations when available
//! - Reliability: Graceful fallbacks when Zig toolchain unavailable  
//! - Development: Fast incremental builds with proper dependency tracking
//! - Cross-compilation: Supports all major platforms

use std::env;
use std::process::Command;
use std::path::Path;
use std::fs;

fn main() {
    // ========================================================================
    // BUILD ORCHESTRATION PIPELINE
    // ========================================================================
    
    // Only watch actual source files, not cache directories
    println!("cargo:rerun-if-changed=zig-modules/build.zig");
    println!("cargo:rerun-if-changed=zig-modules/build.zig.zon");
    
    // Watch specific source files to avoid cache directory triggers
    if Path::new("zig-modules/src").exists() {
        // Walk through src directory and watch only .zig files
        if let Ok(entries) = fs::read_dir("zig-modules/src") {
            for entry in entries {
                if let Ok(entry) = entry {
                    let path = entry.path();
                    if path.is_file() && path.extension().map_or(false, |ext| ext == "zig") {
                        println!("cargo:rerun-if-changed={}", path.display());
                    } else if path.is_dir() {
                        // Recursively watch subdirectories for .zig files
                        watch_zig_files_recursively(&path);
                    }
                }
            }
        }
    }
    
    // Step 1: Build optimized Zig math/SIMD library
    let zig_success = build_zig_library();
    
    // Step 2: Configure Rust linking based on Zig availability
    configure_rust_linking(zig_success);
    
    // Step 3: Build Tauri application framework
    tauri_build::build();
    
    println!("cargo:warning=ManifestRustTS build orchestration complete");
}

/// Recursively watch .zig files in a directory, ignoring cache files
fn watch_zig_files_recursively(dir: &Path) {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries {
            if let Ok(entry) = entry {
                let path = entry.path();
                let file_name = path.file_name().unwrap_or_default().to_string_lossy();
                
                // Skip cache directories and other non-source directories
                if file_name.starts_with('.') || 
                   file_name == "zig-cache" || 
                   file_name == "zig-out" {
                    continue;
                }
                
                if path.is_file() && path.extension().map_or(false, |ext| ext == "zig") {
                    println!("cargo:rerun-if-changed={}", path.display());
                } else if path.is_dir() {
                    watch_zig_files_recursively(&path);
                }
            }
        }
    }
}

/// Build Zig Math & SIMD Library Using Optimized Build System
/// 
/// Uses our custom build.zig configuration that creates both:
/// - libmanifest_zig.o (object file) 
/// - libmanifest_zig.a (static archive)
/// 
/// The build system automatically handles:
/// - Cross-compilation target matching
/// - Optimization level selection  
/// - C bridge compilation with SIMD flags
/// - Dependency tracking and incremental builds
fn build_zig_library() -> bool {
    let zig_dir = Path::new("zig-modules");
    
    if !zig_dir.exists() {
        println!("cargo:warning=Zig modules directory not found: {}", zig_dir.display());
        return false;
    }
    
    // ========================================================================
    // ZIG BUILD SYSTEM INTEGRATION
    // ========================================================================
    
    // Check if Zig toolchain is available
    let zig_version = Command::new("zig").arg("version").output();
    match zig_version {
        Ok(output) if output.status.success() => {
            let version = String::from_utf8_lossy(&output.stdout);
            println!("cargo:warning=Using Zig toolchain version: {}", version.trim());
        }
        _ => {
            println!("cargo:warning=Zig toolchain not available, using fallback implementations");
            return false;
        }
    }
    
    // Configure Zig build to match Cargo's target and optimization
    let mut zig_cmd = Command::new("zig");
    zig_cmd.arg("build").current_dir(zig_dir);
    
    // Use system temp directory for Zig cache AND output to avoid Tauri file watcher conflicts
    let temp_base = env::var("TMPDIR").or_else(|_| env::var("TMP")).or_else(|_| env::var("TEMP"))
        .unwrap_or_else(|_| "/tmp".to_string());
    let temp_base = temp_base.trim_end_matches('/');
    
    let zig_cache_dir = format!("{}/.manifest-zig-cache", temp_base);
    let zig_output_dir = format!("{}/.manifest-zig-output", temp_base);
    
    zig_cmd.arg("--cache-dir").arg(&zig_cache_dir);
    zig_cmd.arg("--prefix-lib-dir").arg(&zig_output_dir);
    
    println!("cargo:warning=Using Zig cache directory: {}", zig_cache_dir);
    println!("cargo:warning=Using Zig output directory: {}", zig_output_dir);
    
    // ========================================================================
    // CROSS-COMPILATION TARGET MATCHING  
    // ========================================================================
    
    // Match Cargo's target triple with Zig's target system
    if let Ok(target) = env::var("TARGET") {
        // Convert Cargo target triple to Zig target format
        let zig_target = match target.as_str() {
            // Common development targets
            "x86_64-apple-darwin" => Some("x86_64-macos"),
            "aarch64-apple-darwin" => Some("aarch64-macos"),
            "x86_64-unknown-linux-gnu" => Some("x86_64-linux-gnu"),
            "aarch64-unknown-linux-gnu" => Some("aarch64-linux-gnu"),
            
            // Windows targets  
            "x86_64-pc-windows-msvc" => Some("x86_64-windows"),
            "x86_64-pc-windows-gnu" => Some("x86_64-windows-gnu"),
            "aarch64-pc-windows-msvc" => Some("aarch64-windows"),
            
            // WebAssembly
            "wasm32-unknown-unknown" => Some("wasm32-freestanding"),
            "wasm32-wasi" => Some("wasm32-wasi"),
            
            // Let Zig auto-detect for other targets
            _ => {
                println!("cargo:warning=Unknown target '{}', letting Zig auto-detect", target);
                None
            }
        };
        
        if let Some(zig_target) = zig_target {
            zig_cmd.arg(format!("-Dtarget={}", zig_target));
            println!("cargo:warning=Cross-compiling Zig library for target: {}", zig_target);
        }
    }
    
    // ========================================================================
    // OPTIMIZATION LEVEL MATCHING
    // ========================================================================
    
    // Match Cargo's optimization level with Zig's build modes
    let zig_optimize = match env::var("OPT_LEVEL").as_deref() {
        Ok("0") => "Debug",           // Cargo debug builds
        Ok("1") => "ReleaseSafe",     // Cargo opt-level 1
        Ok("2") => "ReleaseFast",     // Cargo opt-level 2 (default release)
        Ok("3") => "ReleaseFast",     // Cargo opt-level 3
        Ok("s") => "ReleaseSmall",    // Cargo size optimization
        Ok("z") => "ReleaseSmall",    // Cargo aggressive size optimization
        _ => {
            // Default based on Cargo profile
            if cfg!(debug_assertions) {
                "ReleaseSafe" // Safe optimizations for development
            } else {
                "ReleaseFast" // Maximum performance for production
            }
        }
    };
    
    zig_cmd.arg(format!("-Doptimize={}", zig_optimize));
    println!("cargo:warning=Building Zig library with optimization: {}", zig_optimize);
    
    // ========================================================================
    // BUILD EXECUTION WITH ERROR HANDLING
    // ========================================================================
    
    println!("cargo:warning=Executing Zig build in: {}", zig_dir.display());
    
    let build_result = zig_cmd.output();
    match build_result {
        Ok(output) => {
            if output.status.success() {
                // Check build output for warnings/info
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);
                
                if !stdout.is_empty() {
                    println!("cargo:warning=Zig build output: {}", stdout);
                }
                if !stderr.is_empty() {
                    println!("cargo:warning=Zig build info: {}", stderr);
                }
                
                // Verify artifacts were created in temp directory
                let temp_base = env::var("TMPDIR").or_else(|_| env::var("TMP")).or_else(|_| env::var("TEMP"))
                    .unwrap_or_else(|_| "/tmp".to_string());
                let temp_base = temp_base.trim_end_matches('/');
                let zig_output_dir = format!("{}/.manifest-zig-output", temp_base);
                
                let lib_path = Path::new(&zig_output_dir).join("libmanifest_zig.a");
                let obj_path = Path::new(&zig_output_dir).join("libmanifest_zig.o");
                
                if lib_path.exists() {
                    println!("cargo:warning=Zig static archive created: {}", lib_path.display());
                    
                    // Display file size for build verification
                    if let Ok(metadata) = fs::metadata(&lib_path) {
                        println!("cargo:warning=Archive size: {} bytes", metadata.len());
                    }
                    
                    // Sync the library file to ensure it's completely written
                    // This helps prevent Tauri from detecting partial writes
                    if let Ok(file) = std::fs::File::open(&lib_path) {
                        let _ = file.sync_all();
                    }
                    
                    // Small delay to let filesystem operations settle before Tauri file watcher
                    std::thread::sleep(std::time::Duration::from_millis(100));
                    
                    return true;
                } else if obj_path.exists() {
                    println!("cargo:warning=Zig object file created: {}", obj_path.display());
                    return true;
                } else {
                    println!("cargo:warning=Zig build succeeded but no artifacts found");
                    return false;
                }
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                println!("cargo:warning=Zig build failed with error: {}", stderr);
                return false;
            }
        }
        Err(e) => {
            println!("cargo:warning=Failed to execute Zig build: {}", e);
            return false;
        }
    }
}

/// Configure Rust Linking Based on Zig Library Availability
/// 
/// Sets up the appropriate linking configuration:
/// - With Zig: Links static archive and enables optimized FFI
/// - Without Zig: Enables fallback feature flag for pure Rust implementations
fn configure_rust_linking(zig_available: bool) {
    if zig_available {
        // ====================================================================
        // ZIG LIBRARY LINKING CONFIGURATION
        // ====================================================================
        
        // Use the same temp directory as the build process
        let temp_base = env::var("TMPDIR").or_else(|_| env::var("TMP")).or_else(|_| env::var("TEMP"))
            .unwrap_or_else(|_| "/tmp".to_string());
        let temp_base = temp_base.trim_end_matches('/');
        let zig_output_dir = format!("{}/.manifest-zig-output", temp_base);
        
        let lib_path = Path::new(&zig_output_dir).join("libmanifest_zig.a");
        let obj_path = Path::new(&zig_output_dir).join("libmanifest_zig.o");
        
        // Determine which artifact to link
        let (link_type, link_name, artifact_path) = if lib_path.exists() {
            ("static", "manifest_zig", lib_path)
        } else if obj_path.exists() {
            // For object files, we need to handle linking differently
            println!("cargo:warning=Object file linking not yet implemented, using fallback");
            configure_fallback_mode();
            return;
        } else {
            println!("cargo:warning=No Zig artifacts found, using fallback");
            configure_fallback_mode(); 
            return;
        };
        
        // Configure linker search paths
        let search_path = artifact_path.parent().unwrap();
        println!("cargo:rustc-link-search=native={}", search_path.display());
        println!("cargo:rustc-link-lib={}={}", link_type, link_name);
        
        // Link system libraries that Zig depends on
        println!("cargo:rustc-link-lib=c");
        
        // Platform-specific system libraries
        #[cfg(target_os = "macos")]
        {
            println!("cargo:rustc-link-lib=framework=Accelerate");
        }
        
        #[cfg(target_os = "linux")]
        {
            println!("cargo:rustc-link-lib=m"); // Math library
        }
        
        #[cfg(target_os = "windows")]
        {
            // Windows doesn't typically need additional math libraries
        }
        
        println!("cargo:warning=Zig SIMD optimizations ENABLED");
        
    } else {
        configure_fallback_mode();
    }
}

/// Configure Pure Rust Fallback Mode
/// 
/// Enables the 'no_zig' feature flag, causing the FFI module to use
/// pure Rust implementations of all mathematical operations.
fn configure_fallback_mode() {
    println!("cargo:rustc-cfg=feature=\"no_zig\"");
    println!("cargo:warning=Using PURE RUST fallback implementations (no SIMD acceleration)");
    println!("cargo:warning=For optimal performance, install Zig toolchain: https://ziglang.org/");
}

// ============================================================================
// BUILD SYSTEM INTEGRATION NOTES FOR FUTURE DEVELOPMENT
// ============================================================================
//
// PERFORMANCE MONITORING:
// - Add build timing measurements
// - Track artifact sizes across builds
// - Monitor incremental build performance
//
// ENHANCED CROSS-COMPILATION:
// - Add support for custom target files
// - Integrate with cross-compilation toolchains
// - Add Docker-based cross-compilation
//
// CI/CD INTEGRATION:
// - Cache Zig artifacts across builds
// - Parallel Zig + Rust compilation
// - Artifact validation and testing
//
// DEVELOPMENT WORKFLOW:
// - Watch mode for Zig source changes
// - Hot reloading for math/SIMD functions
// - Benchmark integration for performance regression detection
//
// ADVANCED FFI:
// - Automatic C header generation (cbindgen)
// - Bidirectional Rust ↔ Zig callbacks  
// - Memory layout validation across language boundaries
//