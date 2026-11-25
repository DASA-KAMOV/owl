// SPDX-License-Identifier: GPL-3.0-or-later

// There MUST be a better way of doing this. It seems like a lot of work just to get an icon working consistently.

fn main() {
    // Only Windows executables use embedded Win32 icon/version resources.
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    if target_os != "windows" {
        return;
    }

    let target_env = std::env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default();
    let target_arch = std::env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default();
    let is_cross = std::env::var("HOST").ok() != std::env::var("TARGET").ok();

    let manifest_dir = std::path::PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let out_dir = std::path::PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let icon_path = manifest_dir.join("owl.ico");
    let rc_path = out_dir.join("resource.rc");

    println!("cargo:rerun-if-changed={}", icon_path.display());

    let icon_rc_path = icon_path.display().to_string().replace('\\', "/");
    let rc_contents = format!("1 ICON \"{}\"\n", icon_rc_path);
    std::fs::write(&rc_path, rc_contents).expect("Failed to write resource.rc");

    if target_env == "gnu" {
        // MinGW/GNU toolchains expect a compiled COFF object (.o) passed directly to the linker.
        let res_obj = out_dir.join("resource.o");
        let windres_cmd = if is_cross && cfg!(unix) {
            match target_arch.as_str() {
                "x86" => "i686-w64-mingw32-windres",
                _ => "x86_64-w64-mingw32-windres",
            }
        } else {
            "windres"
        };

        let status = std::process::Command::new(windres_cmd)
            .args(["-i"])
            .arg(&rc_path)
            .args(["-o"])
            .arg(&res_obj)
            .args(["-O", "coff"])
            .status();

        match status {
            Ok(s) if s.success() => {
                println!("cargo:rustc-link-arg={}", res_obj.display());
            }
            Ok(s) => panic!("{windres_cmd} exited with status: {s}"),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                println!(
                    "cargo:warning='{windres_cmd}' not found; skipping .exe icon embedding. \
                     Install mingw-w64 or ensure windres is in PATH."
                );
            }
            Err(e) => panic!("Failed to execute '{windres_cmd}': {e}"),
        }
    } else if target_env == "msvc" {
        // MSVC toolchains compile .rc -> .res with rc.exe and link the .res file directly.
        let res_file = out_dir.join("resource.res");
        let status = std::process::Command::new("rc")
            .args(["/nologo", "/fo"])
            .arg(&res_file)
            .arg(&rc_path)
            .status();

        match status {
            Ok(s) if s.success() => {
                println!("cargo:rustc-link-arg={}", res_file.display());
            }
            Ok(s) => panic!("rc.exe exited with status: {s}"),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                // Make sure rc.exe is in the Windows PATH if you see this.
                // Default install path is usually C:\Program Files (x86)\Windows Kits\10\bin\VERSION_NUMBER_HERE\x64
                println!(
                    "cargo:warning=rc.exe not found; skipping .exe icon embedding. Make sure rc.exe is in the Windows PATH."
                );
            }
            Err(e) => panic!("Failed to execute rc.exe: {e}"),
        }
    }
}
