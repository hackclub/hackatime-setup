use std::process::Command;

pub fn is_process_running(process_name: &str) -> bool {
    #[cfg(target_os = "windows")]
    {
        if let Ok(output) = Command::new("tasklist")
            .args(["/FI", &format!("IMAGENAME eq {}.exe", process_name)])
            .output()
        {
            let stdout = String::from_utf8_lossy(&output.stdout);
            return stdout.contains(&format!("{}.exe", process_name));
        }
        false
    }

    #[cfg(not(target_os = "windows"))]
    {
        if let Ok(output) = Command::new("pgrep").arg("-i").arg(process_name).output() {
            return output.status.success();
        }
        false
    }
}
