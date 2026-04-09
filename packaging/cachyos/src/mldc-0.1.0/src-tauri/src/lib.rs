mod clone;
mod device;
mod fake;
mod models;
mod warnings;

use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

use clone::{build_clone_config, spawn_clone};
use device::{discover_disks, ensure_tool_available};
use fake::{fake_disks, spawn_fake_clone};
use models::{AppMode, DiskDevice, RuntimeStatus, StartCloneResponse, WarningItem};
use tauri::State;
use warnings::build_clone_warnings;

pub struct CloneRunState {
    pub running: Mutex<bool>,
    pub active_pid: Mutex<Option<i32>>,
    pub cancel_requested: AtomicBool,
}

#[tauri::command]
fn get_runtime_status() -> RuntimeStatus {
    let mode = resolve_mode();
    if mode == AppMode::Fake {
        return RuntimeStatus {
            mode,
            is_root: true,
            has_lsblk: true,
            has_dd: true,
            errors: Vec::new(),
        };
    }

    let has_lsblk = ensure_tool_available("lsblk").is_ok();
    let has_dd = ensure_tool_available("dd").is_ok();
    let is_root = running_as_root();
    let mut errors = Vec::new();

    if !is_root {
        errors.push(
            "The app is not running as root. Device discovery still works, but cloning will likely fail."
                .to_owned(),
        );
    }
    if !has_lsblk {
        errors.push("`lsblk` is unavailable or not executable.".to_owned());
    }
    if !has_dd {
        errors.push("`dd` is unavailable or not executable.".to_owned());
    }

    RuntimeStatus {
        mode,
        is_root,
        has_lsblk,
        has_dd,
        errors,
    }
}

#[tauri::command]
fn list_devices() -> Result<Vec<DiskDevice>, String> {
    devices_for_mode(resolve_mode())
}

#[tauri::command]
fn get_clone_warnings(
    source_path: String,
    target_path: String,
) -> Result<Vec<WarningItem>, String> {
    let devices = devices_for_mode(resolve_mode())?;
    let source = find_device(&devices, &source_path)?;
    let target = find_device(&devices, &target_path)?;

    Ok(build_clone_warnings(source, target))
}

#[tauri::command]
fn start_clone(
    app: tauri::AppHandle,
    state: State<'_, CloneRunState>,
    source_path: String,
    target_path: String,
    verify_after_clone: bool,
) -> Result<StartCloneResponse, String> {
    let mode = resolve_mode();
    state.cancel_requested.store(false, Ordering::SeqCst);
    if let Ok(mut active_pid) = state.active_pid.lock() {
        *active_pid = None;
    }
    let mut running = state
        .running
        .lock()
        .map_err(|_| "Failed to lock clone state.".to_owned())?;
    if *running {
        return Err("A clone is already running.".to_owned());
    }

    let devices = devices_for_mode(mode)?;
    let source = find_device(&devices, &source_path)?;
    let target = find_device(&devices, &target_path)?;
    let warnings = build_clone_warnings(source, target);
    if warnings.iter().any(|warning| warning.code == "same-device") {
        return Err("Source and target cannot be the same device.".to_owned());
    }
    if mode == AppMode::Real && !running_as_root() {
        return Err("The app must be run as root before cloning can start.".to_owned());
    }

    let config = build_clone_config(
        &source.path,
        &target.path,
        source.size_bytes,
        verify_after_clone,
    );
    *running = true;
    drop(running);

    if mode == AppMode::Fake {
        Ok(spawn_fake_clone(app, config))
    } else {
        spawn_clone(app, config).map_err(|err| err.message)
    }
}

#[tauri::command]
fn stop_clone(state: State<'_, CloneRunState>) -> Result<(), String> {
    let is_running = *state
        .running
        .lock()
        .map_err(|_| "Failed to lock clone state.".to_owned())?;

    if !is_running {
        return Err("No clone is currently running.".to_owned());
    }

    state.cancel_requested.store(true, Ordering::SeqCst);

    if let Some(pid) = *state
        .active_pid
        .lock()
        .map_err(|_| "Failed to lock clone process state.".to_owned())?
    {
        #[cfg(target_family = "unix")]
        {
            // SAFETY: libc::kill has no Rust-level safety preconditions. We pass a pid captured from a spawned child.
            let result = unsafe { libc::kill(pid, libc::SIGTERM) };
            if result != 0 {
                return Err(format!(
                    "Failed to stop the running clone process: {}",
                    std::io::Error::last_os_error()
                ));
            }
        }
    }

    Ok(())
}

#[tauri::command]
fn save_run_report(report_text: String) -> Result<String, String> {
    let target_dir = std::env::var_os("HOME")
        .map(std::path::PathBuf::from)
        .map(|path| path.join("Downloads"))
        .filter(|path| path.exists())
        .unwrap_or_else(|| {
            std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."))
        });

    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0);
    let file_path = target_dir.join(format!("mldc-run-report-{timestamp}.txt"));

    std::fs::write(&file_path, report_text)
        .map_err(|err| format!("Failed to save run report: {err}"))?;

    Ok(file_path.display().to_string())
}

fn find_device<'a>(devices: &'a [DiskDevice], path: &str) -> Result<&'a DiskDevice, String> {
    devices
        .iter()
        .find(|device| device.path == path)
        .ok_or_else(|| format!("Device `{path}` was not found. Refresh the device list."))
}

fn running_as_root() -> bool {
    #[cfg(target_family = "unix")]
    {
        // SAFETY: libc::geteuid has no preconditions and simply returns the current effective uid.
        unsafe { libc::geteuid() == 0 }
    }

    #[cfg(not(target_family = "unix"))]
    {
        false
    }
}

fn resolve_mode() -> AppMode {
    match std::env::var("MLDC_DEV_MODE")
        .ok()
        .as_deref()
        .map(resolve_mode_from_str)
    {
        Some(mode) => mode,
        None => AppMode::Real,
    }
}

fn resolve_mode_from_str(value: &str) -> AppMode {
    match value {
        value if value.eq_ignore_ascii_case("fake") => AppMode::Fake,
        _ => AppMode::Real,
    }
}

fn devices_for_mode(mode: AppMode) -> Result<Vec<DiskDevice>, String> {
    match mode {
        AppMode::Real => discover_disks().map_err(|err| err.message),
        AppMode::Fake => Ok(fake_disks()),
    }
}

pub fn run() {
    tauri::Builder::default()
        .manage(CloneRunState {
            running: Mutex::new(false),
            active_pid: Mutex::new(None),
            cancel_requested: AtomicBool::new(false),
        })
        .invoke_handler(tauri::generate_handler![
            get_runtime_status,
            list_devices,
            get_clone_warnings,
            start_clone,
            stop_clone,
            save_run_report
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    #[test]
    fn resolves_fake_mode_from_string() {
        assert_eq!(
            super::resolve_mode_from_str("fake"),
            crate::models::AppMode::Fake
        );
        assert_eq!(
            super::resolve_mode_from_str("real"),
            crate::models::AppMode::Real
        );
    }
}
