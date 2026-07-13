use std::process::Command;

// TODO: Imlement targeted unload
// pactl unload-module 63
// pactl unload-module 62

pub fn setup_virtual_microphone() {
    #[cfg(target_os = "linux")]
    {
        // Load the null sink and remap source for Linux
        Command::new("pactl")
            .args(&[
                "load-module",
                "module-null-sink",
                "sink_name=slime_sink",
                "sink_properties=device.description=Slime_Sink",
            ])
            .output()
            .expect("Failed to load null sink module");

        Command::new("pactl")
            .args(&[
                "load-module",
                "module-remap-source",
                "master=slime_sink.monitor",
                "source_name=slime_source",
                "source_properties=device.description=Slime_Microphone",
            ])
            .output()
            .expect("Failed to load remap source module");
    }

    #[cfg(target_os = "windows")]
    {
        // Windows virtual microphone setup would go here
        println!("Virtual microphone setup for Windows is not implemented yet.");
    }

    #[cfg(target_os = "macos")]
    {
        // macOS virtual audio device setup would go here
        println!("Virtual audio device setup for macOS is not implemented yet.");
    }
}

pub fn unload_virtual_microphone() {
    #[cfg(target_os = "linux")]
    {
        // Unload the null sink and remap source for Linux
        Command::new("pactl")
            .args(&["unload-module", "module-remap-source"])
            .output()
            .expect("Failed to unload remap source module");

        Command::new("pactl")
            .args(&["unload-module", "module-null-sink"])
            .output()
            .expect("Failed to unload null sink module");
    }

    #[cfg(target_os = "windows")]
    {
        // Windows virtual microphone teardown would go here
        println!("Virtual microphone teardown for Windows is not implemented yet.");
    }

    #[cfg(target_os = "macos")]
    {
        // macOS virtual audio device teardown would go here
        println!("Virtual audio device teardown for macOS is not implemented yet.");
    }
}
