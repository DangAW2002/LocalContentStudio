//! Worker trait and LocalWorker implementation.

use std::process::Stdio;
use std::sync::Arc;
use std::time::Instant;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;
use tokio::sync::mpsc;

use crate::error::JobError;
use crate::estimate::EstimateStore;
use crate::event::{FailedReason, TaskEvent};
use crate::job::JobSpec;

#[cfg(windows)]
use crate::windows_job::JobObject;

/// Trait implemented by job execution backends.
pub trait Worker: Send + Sync {
    /// Execute a job according to its specification.
    ///
    /// Progress, stdout, stderr, and lifecycle events are streamed via `event_tx`.
    /// Cancellation can be signalled via `cancel_rx`.
    fn run(
        &self,
        spec: JobSpec,
        event_tx: mpsc::Sender<TaskEvent>,
        cancel_rx: mpsc::Receiver<()>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), JobError>> + Send + '_>>;
}

/// Local process execution worker.
///
/// On Windows, executes each job inside an isolated Job Object, enforcing memory and CPU limits,
/// and ensuring the entire process tree terminates on exit, cancel, or timeout.
pub struct LocalWorker {
    estimate_store: Arc<dyn EstimateStore>,
}

impl LocalWorker {
    /// Create a new `LocalWorker` backed by the given estimate store.
    pub fn new(estimate_store: Arc<dyn EstimateStore>) -> Self {
        Self { estimate_store }
    }
}

impl Worker for LocalWorker {
    fn run(
        &self,
        spec: JobSpec,
        event_tx: mpsc::Sender<TaskEvent>,
        mut cancel_rx: mpsc::Receiver<()>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), JobError>> + Send + '_>>
    {
        Box::pin(async move {
            let start_time = Instant::now();
            let tool_id = spec.tool_id.clone();

            // Calculate effective memory limit: declared memory_limit_mb, or 1.5x estimate
            let estimated_ram = self
                .estimate_store
                .get_peak_ram_mb(&tool_id)
                .unwrap_or(spec.resources.ram_mb)
                .max(spec.resources.ram_mb);

            let effective_mem_limit = spec
                .memory_limit_mb
                .or_else(|| Some(((estimated_ram as f64) * 1.5).ceil() as u64));

            #[cfg(windows)]
            let job_obj = match JobObject::create(effective_mem_limit, spec.priority) {
                Ok(jo) => jo,
                Err(err) => {
                    let _ = event_tx
                        .send(TaskEvent::Failed {
                            reason: FailedReason::SpawnError(err.to_string()),
                        })
                        .await;
                    return Err(err);
                }
            };

            // Setup sanitized command
            let sanitized_env = spec.command.build_sanitized_env();

            let mut cmd = Command::new(&spec.command.program);
            cmd.args(&spec.command.args)
                .current_dir(&spec.command.cwd)
                .env_clear()
                .envs(&sanitized_env)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .stdin(if spec.command.stdin.is_some() {
                    Stdio::piped()
                } else {
                    Stdio::null()
                });

            #[cfg(windows)]
            {
                use windows_sys::Win32::System::Threading::CREATE_SUSPENDED;
                cmd.creation_flags(CREATE_SUSPENDED);
            }

            let mut child = match cmd.spawn() {
                Ok(child) => child,
                Err(err) => {
                    let err_msg =
                        format!("Failed to spawn child '{}': {}", spec.command.program, err);
                    let _ = event_tx
                        .send(TaskEvent::Failed {
                            reason: FailedReason::SpawnError(err_msg.clone()),
                        })
                        .await;
                    return Err(JobError::SpawnError(err_msg));
                }
            };

            let pid = child.id().unwrap_or(0);

            #[cfg(windows)]
            if let Some(raw_handle) = child.raw_handle() {
                if let Err(err) = job_obj.assign_process(raw_handle, spec.priority) {
                    let _ = child.kill().await;
                    let err_msg = format!("Failed to assign process to Job Object: {}", err);
                    let _ = event_tx
                        .send(TaskEvent::Failed {
                            reason: FailedReason::SpawnError(err_msg.clone()),
                        })
                        .await;
                    return Err(JobError::SpawnError(err_msg));
                }

                // Resume the suspended process threads after assignment to close the race condition
                if let Err(err) = job_obj.resume_process(pid) {
                    let _ = child.kill().await;
                    let err_msg = format!("Failed to resume suspended process: {}", err);
                    let _ = event_tx
                        .send(TaskEvent::Failed {
                            reason: FailedReason::SpawnError(err_msg.clone()),
                        })
                        .await;
                    return Err(JobError::SpawnError(err_msg));
                }
            }

            let _ = event_tx.send(TaskEvent::Started { pid }).await;

            // Handle stdin if provided
            if let Some(stdin_bytes) = spec.command.stdin {
                if let Some(mut stdin) = child.stdin.take() {
                    tokio::spawn(async move {
                        let _ = stdin.write_all(&stdin_bytes).await;
                        let _ = stdin.flush().await;
                    });
                }
            }

            // Stream stdout
            let stdout_tx = event_tx.clone();
            if let Some(stdout) = child.stdout.take() {
                tokio::spawn(async move {
                    let mut reader = BufReader::new(stdout).lines();
                    while let Ok(Some(line)) = reader.next_line().await {
                        if stdout_tx.send(TaskEvent::Stdout(line)).await.is_err() {
                            break;
                        }
                    }
                });
            }

            // Stream stderr
            let stderr_tx = event_tx.clone();
            if let Some(stderr) = child.stderr.take() {
                tokio::spawn(async move {
                    let mut reader = BufReader::new(stderr).lines();
                    while let Ok(Some(line)) = reader.next_line().await {
                        if stderr_tx.send(TaskEvent::Stderr(line)).await.is_err() {
                            break;
                        }
                    }
                });
            }

            // Wait for completion, timeout, or cancellation
            let timeout_duration = spec.timeout;
            let timeout_fut = tokio::time::sleep(timeout_duration);
            tokio::pin!(timeout_fut);

            let exit_status;
            let mut failed_reason = None;

            tokio::select! {
                status_res = child.wait() => {
                    match status_res {
                        Ok(st) => exit_status = Some(st),
                        Err(err) => {
                            failed_reason = Some(FailedReason::SpawnError(err.to_string()));
                            exit_status = None;
                        }
                    }
                }
                _ = &mut timeout_fut => {
                    let _ = child.kill().await;
                    failed_reason = Some(FailedReason::Timeout);
                    exit_status = None;
                }
                _ = cancel_rx.recv() => {
                    let _ = child.kill().await;
                    failed_reason = Some(FailedReason::Cancelled);
                    exit_status = None;
                }
            }

            // Collect memory usage
            #[cfg(windows)]
            let peak_mb = job_obj.query_peak_memory_mb();
            #[cfg(not(windows))]
            let peak_mb = 0u64;

            if peak_mb > 0 {
                self.estimate_store.update_peak_ram_mb(&tool_id, peak_mb);
            }

            let duration_ms = start_time.elapsed().as_millis() as u64;

            if let Some(reason) = failed_reason {
                let _ = event_tx
                    .send(TaskEvent::Failed {
                        reason: reason.clone(),
                    })
                    .await;
                return match reason {
                    FailedReason::Timeout => Err(JobError::Timeout(timeout_duration)),
                    FailedReason::Cancelled => Err(JobError::Cancelled),
                    FailedReason::SpawnError(msg) => Err(JobError::SpawnError(msg)),
                    FailedReason::OutOfMemory => {
                        Err(JobError::OutOfMemory(effective_mem_limit.unwrap_or(0)))
                    }
                    FailedReason::Exit(c) => Err(JobError::Exit(c)),
                };
            }

            if let Some(st) = exit_status {
                let code = st.code().unwrap_or(-1);

                // On Windows, when a process hits the Job Object memory limit,
                // the operating system terminates the process with STATUS_QUOTA_EXCEEDED (0xC0000044)
                // or code -1073741756, or memory allocations fail with OOM errors.
                let is_oom = {
                    #[cfg(windows)]
                    {
                        code == -1073741756
                            || (code as u32) == 0xC0000044
                            || (effective_mem_limit.is_some()
                                && peak_mb + 5 >= effective_mem_limit.unwrap()
                                && !st.success())
                            || (effective_mem_limit.is_some()
                                && peak_mb >= effective_mem_limit.unwrap())
                    }
                    #[cfg(not(windows))]
                    {
                        false
                    }
                };

                if is_oom {
                    let _ = event_tx
                        .send(TaskEvent::Failed {
                            reason: FailedReason::OutOfMemory,
                        })
                        .await;
                    return Err(JobError::OutOfMemory(effective_mem_limit.unwrap_or(0)));
                }

                if st.success() {
                    let _ = event_tx
                        .send(TaskEvent::Finished {
                            exit_code: code,
                            peak_memory_mb: peak_mb,
                            duration_ms,
                        })
                        .await;
                    Ok(())
                } else {
                    let _ = event_tx
                        .send(TaskEvent::Failed {
                            reason: FailedReason::Exit(code),
                        })
                        .await;
                    Err(JobError::Exit(code))
                }
            } else {
                let _ = event_tx
                    .send(TaskEvent::Failed {
                        reason: FailedReason::SpawnError("Process exited without status".into()),
                    })
                    .await;
                Err(JobError::SpawnError("Process exited without status".into()))
            }
        })
    }
}
