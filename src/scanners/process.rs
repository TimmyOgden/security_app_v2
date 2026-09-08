//! Shared helper for running a scanner's child process in a way that can
//! actually be interrupted by a "Stop" request. Every scanner used to just
//! `.output().await` its command, which blocks until the process exits on
//! its own — the scan-level cancellation flag (SCAN_CANCEL) was only ever
//! checked *between* tools, so a hung or slow tool couldn't be stopped.
//!
//! `run_cancellable` spawns the process, reads its stdout/stderr in the
//! background, and races the process's exit against a periodic check of
//! SCAN_CANCEL — killing the child and returning `None` if the scan was
//! cancelled while this tool was still running.

use super::runner::SCAN_CANCEL;
use std::process::Stdio;
use tokio::io::AsyncReadExt;
use tokio::process::Command;

const POLL_INTERVAL_MS: u64 = 500;

pub struct CancellableOutput {
    pub status_success: bool,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

/// Runs `cmd` to completion, or kills it and returns `Ok(None)` if
/// `scan_job_id` is marked cancelled while it's still running. `Err` means
/// the process itself failed to spawn (e.g. the binary is missing) — kept
/// distinct from cancellation so callers can log the right thing.
pub async fn run_cancellable(mut cmd: Command, scan_job_id: i64) -> std::io::Result<Option<CancellableOutput>> {
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    let mut child = cmd.spawn()?;

    let stdout_pipe = child.stdout.take();
    let stderr_pipe = child.stderr.take();

    let stdout_task = tokio::spawn(async move {
        let mut buf = Vec::new();
        if let Some(mut p) = stdout_pipe {
            let _ = p.read_to_end(&mut buf).await;
        }
        buf
    });
    let stderr_task = tokio::spawn(async move {
        let mut buf = Vec::new();
        if let Some(mut p) = stderr_pipe {
            let _ = p.read_to_end(&mut buf).await;
        }
        buf
    });

    loop {
        tokio::select! {
            status = child.wait() => {
                let stdout = stdout_task.await.unwrap_or_default();
                let stderr = stderr_task.await.unwrap_or_default();
                return Ok(status.ok().map(|s| CancellableOutput {
                    status_success: s.success(),
                    stdout,
                    stderr,
                }));
            }
            _ = tokio::time::sleep(std::time::Duration::from_millis(POLL_INTERVAL_MS)) => {
                let cancelled = SCAN_CANCEL.read().await.get(&scan_job_id).copied().unwrap_or(false);
                if cancelled {
                    let _ = child.kill().await;
                    stdout_task.abort();
                    stderr_task.abort();
                    return Ok(None);
                }
            }
        }
    }
}
