use std::sync::atomic::Ordering;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use tauri::{AppHandle, Emitter, Manager};

use crate::models::{
    CloneConfig, CloneProgress, CloneResult, DiskDevice, PartitionInfo, RunPhase,
    StartCloneResponse,
};

pub fn fake_disks() -> Vec<DiskDevice> {
    vec![
        DiskDevice {
            path: "/dev/nvme0n1".to_owned(),
            name: "nvme0n1".to_owned(),
            size_bytes: 1_000_204_886_016,
            size_human: crate::models::format_bytes(1_000_204_886_016),
            model: Some("Samsung 990 PRO".to_owned()),
            serial: Some("SIM-SRC-001".to_owned()),
            removable: false,
            transport: Some("nvme".to_owned()),
            mounted_partitions: vec![PartitionInfo {
                path: "/dev/nvme0n1p2".to_owned(),
                mountpoint: Some("/".to_owned()),
                fstype: Some("ext4".to_owned()),
            }],
        },
        DiskDevice {
            path: "/dev/sda".to_owned(),
            name: "sda".to_owned(),
            size_bytes: 512_110_190_592,
            size_human: crate::models::format_bytes(512_110_190_592),
            model: Some("Crucial MX500".to_owned()),
            serial: Some("SIM-TGT-002".to_owned()),
            removable: false,
            transport: Some("sata".to_owned()),
            mounted_partitions: Vec::new(),
        },
        DiskDevice {
            path: "/dev/sdb".to_owned(),
            name: "sdb".to_owned(),
            size_bytes: 256_060_514_304,
            size_human: crate::models::format_bytes(256_060_514_304),
            model: Some("USB SSD enclosure".to_owned()),
            serial: Some("SIM-USB-003".to_owned()),
            removable: true,
            transport: Some("usb".to_owned()),
            mounted_partitions: vec![PartitionInfo {
                path: "/dev/sdb1".to_owned(),
                mountpoint: Some("/run/media/demo".to_owned()),
                fstype: Some("ext4".to_owned()),
            }],
        },
        DiskDevice {
            path: "/dev/mmcblk0".to_owned(),
            name: "mmcblk0".to_owned(),
            size_bytes: 63_912_615_936,
            size_human: crate::models::format_bytes(63_912_615_936),
            model: None,
            serial: None,
            removable: true,
            transport: Some("mmc".to_owned()),
            mounted_partitions: Vec::new(),
        },
    ]
}

pub fn spawn_fake_clone(app: AppHandle, config: CloneConfig) -> StartCloneResponse {
    let run_id = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| format!("fake-{}", duration.as_millis()))
        .unwrap_or_else(|_| "fake-clone-run".to_owned());

    let response = StartCloneResponse {
        run_id,
        command_preview: config.command_preview.clone(),
        total_bytes: config.total_bytes,
        verify_after_clone: config.verify_after_clone,
    };

    thread::spawn(move || {
        let total_steps = 18_u64;
        let speed_bytes_per_sec = 158_000_000_u64;
        let mut progress = CloneProgress::new(RunPhase::Clone, config.total_bytes);

        for step in 1..=total_steps {
            thread::sleep(Duration::from_millis(260));

            if cancellation_requested(&app) {
                let mut raw_output = progress.raw_output.clone();
                raw_output.push("Simulation stopped by user.".to_owned());
                let _ = app.emit(
                    "clone-finished",
                    CloneResult {
                        phase: RunPhase::Clone,
                        success: false,
                        bytes_copied: progress.bytes_copied,
                        elapsed_secs: progress.elapsed_secs,
                        average_speed: Some(speed_bytes_per_sec),
                        final_message: "Simulation stopped by user.".to_owned(),
                        verify_requested: config.verify_after_clone,
                        verify_completed: false,
                        raw_output,
                    },
                );
                clear_running_flag(&app);
                return;
            }

            progress.bytes_copied = config.total_bytes.saturating_mul(step) / total_steps;
            progress.percent = if config.total_bytes == 0 {
                0.0
            } else {
                (progress.bytes_copied as f32 / config.total_bytes as f32).clamp(0.0, 1.0)
            };
            progress.speed_bytes_per_sec = Some(speed_bytes_per_sec);
            progress.elapsed_secs = step;

            let remaining_bytes = config.total_bytes.saturating_sub(progress.bytes_copied);
            progress.eta_secs = if speed_bytes_per_sec == 0 || remaining_bytes == 0 {
                None
            } else {
                Some(remaining_bytes / speed_bytes_per_sec)
            };

            progress.raw_output.push(format!(
                "{} bytes ({}) copied, {} s, {}/s",
                progress.bytes_copied,
                crate::models::format_bytes(progress.bytes_copied),
                progress.elapsed_secs,
                crate::models::format_bytes(speed_bytes_per_sec),
            ));

            let _ = app.emit("clone-progress", &progress);
        }

        if config.verify_after_clone {
            let verify_steps = 12_u64;
            let verify_speed_bytes_per_sec = 221_000_000_u64;
            let mut verify_progress = CloneProgress::new(RunPhase::Verify, config.total_bytes);
            verify_progress.raw_output = progress.raw_output.clone();
            verify_progress
                .raw_output
                .push("Starting simulated verification pass.".to_owned());

            for step in 1..=verify_steps {
                thread::sleep(Duration::from_millis(220));

                if cancellation_requested(&app) {
                    let mut raw_output = verify_progress.raw_output.clone();
                    raw_output.push("Simulation verification stopped by user.".to_owned());
                    let _ = app.emit(
                        "clone-finished",
                        CloneResult {
                            phase: RunPhase::Verify,
                            success: false,
                            bytes_copied: verify_progress.bytes_copied,
                            elapsed_secs: verify_progress.elapsed_secs,
                            average_speed: Some(verify_speed_bytes_per_sec),
                            final_message: "Simulation verification stopped by user.".to_owned(),
                            verify_requested: true,
                            verify_completed: false,
                            raw_output,
                        },
                    );
                    clear_running_flag(&app);
                    return;
                }

                verify_progress.bytes_copied =
                    config.total_bytes.saturating_mul(step) / verify_steps;
                verify_progress.percent = if config.total_bytes == 0 {
                    0.0
                } else {
                    (verify_progress.bytes_copied as f32 / config.total_bytes as f32)
                        .clamp(0.0, 1.0)
                };
                verify_progress.speed_bytes_per_sec = Some(verify_speed_bytes_per_sec);
                verify_progress.elapsed_secs = total_steps + step;

                let remaining_bytes = config
                    .total_bytes
                    .saturating_sub(verify_progress.bytes_copied);
                verify_progress.eta_secs =
                    if verify_speed_bytes_per_sec == 0 || remaining_bytes == 0 {
                        None
                    } else {
                        Some(remaining_bytes / verify_speed_bytes_per_sec)
                    };

                verify_progress.raw_output.push(format!(
                    "Verified {} of {} at {}/s",
                    crate::models::format_bytes(verify_progress.bytes_copied),
                    crate::models::format_bytes(config.total_bytes),
                    crate::models::format_bytes(verify_speed_bytes_per_sec),
                ));

                let _ = app.emit("clone-progress", &verify_progress);
            }

            let result = CloneResult {
                phase: RunPhase::Verify,
                success: true,
                bytes_copied: config.total_bytes,
                elapsed_secs: total_steps + verify_steps,
                average_speed: Some(verify_speed_bytes_per_sec),
                final_message: "Simulation completed and verification passed.".to_owned(),
                verify_requested: true,
                verify_completed: true,
                raw_output: verify_progress.raw_output,
            };
            let _ = app.emit("clone-finished", result);
        } else {
            let result = CloneResult {
                phase: RunPhase::Clone,
                success: true,
                bytes_copied: config.total_bytes,
                elapsed_secs: total_steps,
                average_speed: Some(speed_bytes_per_sec),
                final_message: "Simulation completed successfully.".to_owned(),
                verify_requested: false,
                verify_completed: false,
                raw_output: progress.raw_output,
            };
            let _ = app.emit("clone-finished", result);
        }

        clear_running_flag(&app);
    });

    response
}

fn cancellation_requested(app: &AppHandle) -> bool {
    app.try_state::<crate::CloneRunState>()
        .map(|state| state.cancel_requested.load(Ordering::SeqCst))
        .unwrap_or(false)
}

fn clear_running_flag(app: &AppHandle) {
    if let Some(state) = app.try_state::<crate::CloneRunState>() {
        if let Ok(mut running) = state.running.lock() {
            *running = false;
        }
        if let Ok(mut active_pid) = state.active_pid.lock() {
            *active_pid = None;
        }
        state.cancel_requested.store(false, Ordering::SeqCst);
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn fake_disk_fixture_is_populated() {
        let disks = super::fake_disks();
        assert!(disks.len() >= 4);
        assert!(disks.iter().any(|disk| disk.path == "/dev/nvme0n1"));
        assert!(disks.iter().any(|disk| disk.removable));
    }
}
