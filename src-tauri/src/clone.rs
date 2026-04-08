use std::fs::File;
use std::io::{BufRead, BufReader, Read};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use tauri::{AppHandle, Emitter, Manager};

use crate::device::{DeviceError, ensure_tool_available};
use crate::models::{CloneConfig, CloneProgress, CloneResult, RunPhase, StartCloneResponse};

const VERIFY_BUFFER_SIZE: usize = 4 * 1024 * 1024;

pub fn build_clone_config(
    source_path: &str,
    target_path: &str,
    total_bytes: u64,
    verify_after_clone: bool,
) -> CloneConfig {
    let block_size = "64K".to_owned();
    let command_preview =
        format!("dd if={source_path} of={target_path} bs={block_size} conv=fsync status=progress");

    CloneConfig {
        source_path: source_path.to_owned(),
        target_path: target_path.to_owned(),
        total_bytes,
        block_size,
        verify_after_clone,
        command_preview,
    }
}

pub fn spawn_clone(app: AppHandle, config: CloneConfig) -> Result<StartCloneResponse, DeviceError> {
    ensure_tool_available("dd")?;

    let run_id = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().to_string())
        .unwrap_or_else(|_| "clone-run".to_owned());

    let response = StartCloneResponse {
        run_id,
        command_preview: config.command_preview.clone(),
        total_bytes: config.total_bytes,
        verify_after_clone: config.verify_after_clone,
    };

    std::thread::spawn(move || {
        let result = run_clone_process(&app, &config);
        let _ = app.emit("clone-finished", result);
        clear_running_flag(&app);
    });

    Ok(response)
}

fn run_clone_process(app: &AppHandle, config: &CloneConfig) -> CloneResult {
    let started_at = Instant::now();
    let mut progress = CloneProgress::new(RunPhase::Clone, config.total_bytes);

    let mut command = Command::new("dd");
    command
        .arg(format!("if={}", config.source_path))
        .arg(format!("of={}", config.target_path))
        .arg(format!("bs={}", config.block_size))
        .arg("conv=fsync")
        .arg("status=progress")
        .stderr(Stdio::piped())
        .stdout(Stdio::null());

    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(err) => {
            return result_from_progress(
                &progress,
                RunPhase::Clone,
                false,
                started_at.elapsed(),
                format!("Failed to start `dd`: {err}"),
                vec![config.command_preview.clone()],
                config.verify_after_clone,
                false,
            );
        }
    };

    if let Some(state) = app.try_state::<crate::CloneRunState>() {
        if let Ok(mut active_pid) = state.active_pid.lock() {
            *active_pid = Some(child.id() as i32);
        }
    }

    let stderr = match child.stderr.take() {
        Some(stderr) => stderr,
        None => {
            return result_from_progress(
                &progress,
                RunPhase::Clone,
                false,
                started_at.elapsed(),
                "Failed to capture `dd` stderr.".to_owned(),
                vec![config.command_preview.clone()],
                config.verify_after_clone,
                false,
            );
        }
    };

    let mut reader = BufReader::new(stderr);
    loop {
        let mut line = String::new();
        let bytes = match reader.read_line(&mut line) {
            Ok(bytes) => bytes,
            Err(err) => {
                progress
                    .raw_output
                    .push(format!("Failed to read dd output: {err}"));
                return result_from_progress(
                    &progress,
                    RunPhase::Clone,
                    false,
                    started_at.elapsed(),
                    "Failed while reading dd output.".to_owned(),
                    progress.raw_output.clone(),
                    config.verify_after_clone,
                    false,
                );
            }
        };

        if bytes == 0 {
            break;
        }

        if cancellation_requested(app) {
            let _ = child.kill();
        }

        let trimmed = line.trim().to_owned();
        if trimmed.is_empty() {
            continue;
        }

        progress.raw_output.push(trimmed.clone());
        progress.elapsed_secs = started_at.elapsed().as_secs();

        if let Some(parsed) = parse_progress_line(&trimmed) {
            progress.bytes_copied = parsed.bytes_copied;
            progress.speed_bytes_per_sec = parsed.speed_bytes_per_sec;
            progress.percent = compute_percent(progress.bytes_copied, config.total_bytes);
            progress.eta_secs = compute_eta(
                progress.bytes_copied,
                config.total_bytes,
                progress.speed_bytes_per_sec,
            )
            .map(|duration| duration.as_secs());
        }

        let _ = app.emit("clone-progress", &progress);
    }

    let status = match child.wait() {
        Ok(status) => status,
        Err(err) => {
            progress
                .raw_output
                .push(format!("Failed waiting for dd: {err}"));
            return result_from_progress(
                &progress,
                RunPhase::Clone,
                false,
                started_at.elapsed(),
                "Failed waiting for dd to exit.".to_owned(),
                progress.raw_output.clone(),
                config.verify_after_clone,
                false,
            );
        }
    };

    if let Some(state) = app.try_state::<crate::CloneRunState>() {
        if let Ok(mut active_pid) = state.active_pid.lock() {
            *active_pid = None;
        }
    }

    progress.bytes_copied = config.total_bytes;
    progress.percent = 1.0;
    progress.eta_secs = None;
    progress.elapsed_secs = started_at.elapsed().as_secs();

    if !status.success() {
        let final_message = if cancellation_requested(app) {
            "Clone stopped by user.".to_owned()
        } else {
            format!("Clone failed with exit status: {status}")
        };
        return result_from_progress(
            &progress,
            RunPhase::Clone,
            false,
            started_at.elapsed(),
            final_message,
            progress.raw_output.clone(),
            config.verify_after_clone,
            false,
        );
    }

    if !config.verify_after_clone {
        return result_from_progress(
            &progress,
            RunPhase::Clone,
            true,
            started_at.elapsed(),
            "Clone completed successfully.".to_owned(),
            progress.raw_output.clone(),
            false,
            false,
        );
    }

    match run_verification(app, config, started_at, progress.raw_output.clone()) {
        Ok(result) => result,
        Err(result) => result,
    }
}

fn run_verification(
    app: &AppHandle,
    config: &CloneConfig,
    started_at: Instant,
    initial_output: Vec<String>,
) -> Result<CloneResult, CloneResult> {
    let source = match File::open(&config.source_path) {
        Ok(file) => file,
        Err(err) => {
            return Err(CloneResult {
                phase: RunPhase::Verify,
                success: false,
                bytes_copied: config.total_bytes,
                elapsed_secs: started_at.elapsed().as_secs(),
                average_speed: average_speed(config.total_bytes, started_at.elapsed()),
                final_message: format!("Verification failed opening source: {err}"),
                verify_requested: true,
                verify_completed: false,
                raw_output: append_output(
                    initial_output,
                    format!("Verification failed opening source: {err}"),
                ),
            });
        }
    };

    let target = match File::open(&config.target_path) {
        Ok(file) => file,
        Err(err) => {
            return Err(CloneResult {
                phase: RunPhase::Verify,
                success: false,
                bytes_copied: config.total_bytes,
                elapsed_secs: started_at.elapsed().as_secs(),
                average_speed: average_speed(config.total_bytes, started_at.elapsed()),
                final_message: format!("Verification failed opening target: {err}"),
                verify_requested: true,
                verify_completed: false,
                raw_output: append_output(
                    initial_output,
                    format!("Verification failed opening target: {err}"),
                ),
            });
        }
    };

    let verify_started = Instant::now();
    match verify_readers(
        source,
        target,
        config.total_bytes,
        initial_output,
        || cancellation_requested(app),
        |progress| {
            let mut emitted = progress;
            emitted.elapsed_secs = started_at.elapsed().as_secs();
            let _ = app.emit("clone-progress", &emitted);
        },
    ) {
        Ok(summary) => Ok(CloneResult {
            phase: RunPhase::Verify,
            success: true,
            bytes_copied: config.total_bytes,
            elapsed_secs: started_at.elapsed().as_secs(),
            average_speed: average_speed(config.total_bytes, verify_started.elapsed()),
            final_message: "Clone completed and verification passed.".to_owned(),
            verify_requested: true,
            verify_completed: true,
            raw_output: summary.raw_output,
        }),
        Err(summary) => Err(CloneResult {
            phase: RunPhase::Verify,
            success: false,
            bytes_copied: config.total_bytes,
            elapsed_secs: started_at.elapsed().as_secs(),
            average_speed: average_speed(config.total_bytes, verify_started.elapsed()),
            final_message: summary.message,
            verify_requested: true,
            verify_completed: false,
            raw_output: summary.raw_output,
        }),
    }
}

#[derive(Debug)]
struct VerificationSummary {
    message: String,
    raw_output: Vec<String>,
}

fn verify_readers<R1: Read, R2: Read, F: FnMut(CloneProgress)>(
    mut source: R1,
    mut target: R2,
    total_bytes: u64,
    mut raw_output: Vec<String>,
    should_cancel: impl Fn() -> bool,
    mut emit_progress: F,
) -> Result<VerificationSummary, VerificationSummary> {
    let started_at = Instant::now();
    let mut source_buffer = vec![0_u8; VERIFY_BUFFER_SIZE];
    let mut target_buffer = vec![0_u8; VERIFY_BUFFER_SIZE];
    let mut verified_bytes = 0_u64;
    let mut progress = CloneProgress::new(RunPhase::Verify, total_bytes);

    raw_output.push("Starting full-device verification pass.".to_owned());

    loop {
        if should_cancel() {
            let message = "Verification stopped by user.".to_owned();
            raw_output.push(message.clone());
            return Err(VerificationSummary {
                message,
                raw_output,
            });
        }

        let source_read = match source.read(&mut source_buffer) {
            Ok(bytes) => bytes,
            Err(err) => {
                let message = format!("Verification failed reading source: {err}");
                raw_output.push(message.clone());
                return Err(VerificationSummary {
                    message,
                    raw_output,
                });
            }
        };

        if source_read == 0 {
            break;
        }

        let mut target_read = 0;
        while target_read < source_read {
            match target.read(&mut target_buffer[target_read..source_read]) {
                Ok(0) => {
                    let message =
                        "Verification failed: target ended before the source data.".to_owned();
                    raw_output.push(message.clone());
                    return Err(VerificationSummary {
                        message,
                        raw_output,
                    });
                }
                Ok(bytes) => {
                    target_read += bytes;
                }
                Err(err) => {
                    let message = format!("Verification failed reading target: {err}");
                    raw_output.push(message.clone());
                    return Err(VerificationSummary {
                        message,
                        raw_output,
                    });
                }
            }
        }

        if source_buffer[..source_read] != target_buffer[..source_read] {
            let message = format!(
                "Verification failed: mismatch detected at byte offset {}.",
                verified_bytes
            );
            raw_output.push(message.clone());
            return Err(VerificationSummary {
                message,
                raw_output,
            });
        }

        verified_bytes = verified_bytes.saturating_add(source_read as u64);
        progress.bytes_copied = verified_bytes.min(total_bytes);
        progress.percent = compute_percent(progress.bytes_copied, total_bytes);
        progress.elapsed_secs = started_at.elapsed().as_secs();
        progress.speed_bytes_per_sec = average_speed(progress.bytes_copied, started_at.elapsed());
        progress.eta_secs = compute_eta(
            progress.bytes_copied,
            total_bytes,
            progress.speed_bytes_per_sec,
        )
        .map(|duration| duration.as_secs());

        let log_line = format!(
            "Verified {} of {}.",
            crate::models::format_bytes(progress.bytes_copied),
            crate::models::format_bytes(total_bytes)
        );
        raw_output.push(log_line);
        progress.raw_output = raw_output.clone();
        emit_progress(progress.clone());
    }

    raw_output.push("Verification completed successfully.".to_owned());
    Ok(VerificationSummary {
        message: "Verification completed successfully.".to_owned(),
        raw_output,
    })
}

fn append_output(mut raw_output: Vec<String>, line: String) -> Vec<String> {
    raw_output.push(line);
    raw_output
}

fn result_from_progress(
    progress: &CloneProgress,
    phase: RunPhase,
    success: bool,
    elapsed: Duration,
    final_message: String,
    raw_output: Vec<String>,
    verify_requested: bool,
    verify_completed: bool,
) -> CloneResult {
    CloneResult {
        phase,
        success,
        bytes_copied: progress.bytes_copied,
        elapsed_secs: elapsed.as_secs(),
        average_speed: average_speed(progress.bytes_copied, elapsed),
        final_message,
        verify_requested,
        verify_completed,
        raw_output,
    }
}

fn clear_running_flag(app: &AppHandle) {
    if let Some(state) = app.try_state::<crate::CloneRunState>() {
        if let Ok(mut running) = state.running.lock() {
            *running = false;
        }
        if let Ok(mut active_pid) = state.active_pid.lock() {
            *active_pid = None;
        }
        state
            .cancel_requested
            .store(false, std::sync::atomic::Ordering::SeqCst);
    }
}

fn cancellation_requested(app: &AppHandle) -> bool {
    app.try_state::<crate::CloneRunState>()
        .map(|state| {
            state
                .cancel_requested
                .load(std::sync::atomic::Ordering::SeqCst)
        })
        .unwrap_or(false)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ParsedProgress {
    bytes_copied: u64,
    speed_bytes_per_sec: Option<u64>,
}

fn parse_progress_line(line: &str) -> Option<ParsedProgress> {
    let bytes_section = line.split_whitespace().next()?;
    let bytes_copied = bytes_section.replace(',', "").parse().ok()?;

    let speed_segment = line.rsplit(", ").next()?;
    let speed_bytes_per_sec = parse_speed(speed_segment);

    Some(ParsedProgress {
        bytes_copied,
        speed_bytes_per_sec,
    })
}

fn parse_speed(segment: &str) -> Option<u64> {
    let mut parts = segment.split_whitespace();
    let raw_value = parts.next()?.replace(',', ".");
    let unit = parts.next()?;

    let value: f64 = raw_value.parse().ok()?;
    let multiplier = match unit {
        "B/s" => 1.0,
        "kB/s" => 1_000.0,
        "MB/s" => 1_000_000.0,
        "GB/s" => 1_000_000_000.0,
        "TB/s" => 1_000_000_000_000.0,
        "KiB/s" => 1024.0,
        "MiB/s" => 1024.0 * 1024.0,
        "GiB/s" => 1024.0 * 1024.0 * 1024.0,
        _ => return None,
    };

    Some((value * multiplier) as u64)
}

fn compute_percent(bytes_copied: u64, total_bytes: u64) -> f32 {
    if total_bytes == 0 {
        0.0
    } else {
        (bytes_copied as f32 / total_bytes as f32).clamp(0.0, 1.0)
    }
}

fn compute_eta(bytes_copied: u64, total_bytes: u64, speed: Option<u64>) -> Option<Duration> {
    let speed = speed?;
    if bytes_copied == 0 || total_bytes <= bytes_copied || speed == 0 {
        return None;
    }

    let remaining = total_bytes - bytes_copied;
    Some(Duration::from_secs(remaining / speed))
}

fn average_speed(bytes_copied: u64, elapsed: Duration) -> Option<u64> {
    if bytes_copied == 0 || elapsed.is_zero() {
        None
    } else {
        Some((bytes_copied as f64 / elapsed.as_secs_f64()) as u64)
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    #[test]
    fn parses_standard_dd_progress_line() {
        let parsed =
            super::parse_progress_line("1048576 bytes (1.0 MB, 1.0 MiB) copied, 1.234 s, 850 kB/s")
                .unwrap();

        assert_eq!(parsed.bytes_copied, 1_048_576);
        assert_eq!(parsed.speed_bytes_per_sec, Some(850_000));
    }

    #[test]
    fn ignores_non_progress_output() {
        assert!(super::parse_progress_line("dd: failed to open '/dev/sdb'").is_none());
    }

    #[test]
    fn computes_eta_from_progress() {
        let eta = super::compute_eta(500, 1000, Some(100)).unwrap();
        assert_eq!(eta.as_secs(), 5);
    }

    #[test]
    fn verification_succeeds_for_matching_streams() {
        let source = Cursor::new(vec![1_u8, 2, 3, 4, 5, 6]);
        let target = Cursor::new(vec![1_u8, 2, 3, 4, 5, 6]);

        let result = super::verify_readers(source, target, 6, Vec::new(), || false, |_| {});
        assert!(result.is_ok());
    }

    #[test]
    fn verification_fails_on_mismatch() {
        let source = Cursor::new(vec![1_u8, 2, 3, 4, 5, 6]);
        let target = Cursor::new(vec![1_u8, 2, 9, 4, 5, 6]);

        let result = super::verify_readers(source, target, 6, Vec::new(), || false, |_| {});
        assert!(result.is_err());
    }
}
