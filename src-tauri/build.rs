fn main() {
    #[cfg(all(windows, target_arch = "x86_64"))]
    {
        println!("cargo:rerun-if-changed=native/scaling");
        println!("cargo:rerun-if-env-changed=CL");
        println!("cargo:rerun-if-env-changed=WAYPOINT_NATIVE_CXXFLAGS");
        let status = std::process::Command::new("powershell")
            // Windows PowerShell needs its own module path when Cargo's parent uses pwsh.
            .env_remove("PSModulePath")
            .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-File", "native/scaling/build.ps1"])
            .status().expect("could not run native scaling build");
        assert!(status.success(), "native scaling build failed");
    }
    println!("cargo:rerun-if-changed=icons/icon.ico");
    println!("cargo:rerun-if-changed=tauri.conf.json");
    tauri_build::build()
}
