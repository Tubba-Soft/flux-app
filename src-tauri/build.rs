fn main() {
    // Embed Windows Application Manifest to always require Administrator elevation
    #[cfg(windows)]
    {
        let mut res = tauri_build::WindowsAttributes::new();
        // Use our custom manifest that sets requestedExecutionLevel = "requireAdministrator"
        res = res.app_manifest(include_str!("netflow-studio.exe.manifest"));
        tauri_build::try_build(
            tauri_build::Attributes::new().windows_attributes(res),
        )
        .expect("failed to run tauri-build");
    }

    #[cfg(not(windows))]
    {
        tauri_build::build();
    }
}
