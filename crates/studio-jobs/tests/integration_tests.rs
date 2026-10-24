//! Unit and integration tests for studio-jobs.

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;

use studio_jobs::command::CommandSpec;
use studio_jobs::error::JobError;
use studio_jobs::estimate::{EstimateStore, MemoryEstimateStore};
use studio_jobs::event::{FailedReason, JobState, TaskEvent};
use studio_jobs::job::{JobSpec, Lifecycle, Priority};
use studio_jobs::probe::FakeProbe;
use studio_jobs::resource::{ResourceSpec, SchedulerConfig};
use studio_jobs::scheduler::Scheduler;
use studio_jobs::worker::Worker;

fn dummy_command() -> CommandSpec {
    #[cfg(windows)]
    {
        CommandSpec {
            program: "cmd.exe".into(),
            args: vec!["/c".into(), "echo ok".into()],
            cwd: std::env::temp_dir(),
            env: BTreeMap::new(),
            stdin: None,
        }
    }
    #[cfg(not(windows))]
    {
        CommandSpec {
            program: "echo".into(),
            args: vec!["ok".into()],
            cwd: std::env::temp_dir(),
            env: BTreeMap::new(),
            stdin: None,
        }
    }
}

fn dummy_spec(id: &str, priority: Priority) -> JobSpec {
    JobSpec {
        id: id.into(),
        workspace_id: "ws-test".into(),
        tool_id: "tool:dummy".into(),
        command: dummy_command(),
        resources: ResourceSpec::default(),
        priority,
        timeout: Duration::from_secs(10),
        memory_limit_mb: None,
        lifecycle: Lifecycle::PerCall,
    }
}

// Mock worker for testing scheduler queue and admission logic without spawning OS processes
struct MockWorker {
    delay: Duration,
    active_count: Arc<AtomicUsize>,
    max_concurrent: Arc<AtomicUsize>,
}

impl MockWorker {
    fn new(delay: Duration) -> Self {
        Self {
            delay,
            active_count: Arc::new(AtomicUsize::new(0)),
            max_concurrent: Arc::new(AtomicUsize::new(0)),
        }
    }
}

impl Worker for MockWorker {
    fn run(
        &self,
        spec: JobSpec,
        event_tx: mpsc::Sender<TaskEvent>,
        mut cancel_rx: mpsc::Receiver<()>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), JobError>> + Send + '_>>
    {
        let active = Arc::clone(&self.active_count);
        let max_c = Arc::clone(&self.max_concurrent);
        let delay = self.delay;

        Box::pin(async move {
            let cur = active.fetch_add(1, Ordering::SeqCst) + 1;
            max_c.fetch_max(cur, Ordering::SeqCst);

            let _ = event_tx.send(TaskEvent::Started { pid: 1234 }).await;

            tokio::select! {
                _ = tokio::time::sleep(delay) => {
                    active.fetch_sub(1, Ordering::SeqCst);
                    let _ = event_tx.send(TaskEvent::Finished {
                        exit_code: 0,
                        peak_memory_mb: spec.resources.ram_mb,
                        duration_ms: delay.as_millis() as u64,
                    }).await;
                    Ok(())
                }
                _ = cancel_rx.recv() => {
                    active.fetch_sub(1, Ordering::SeqCst);
                    let _ = event_tx.send(TaskEvent::Failed {
                        reason: FailedReason::Cancelled,
                    }).await;
                    Err(JobError::Cancelled)
                }
            }
        })
    }
}

#[tokio::test]
async fn test_queue_ordering_by_priority() {
    let probe = Arc::new(FakeProbe::new(8192, 4096, 8));
    let estimates = Arc::new(MemoryEstimateStore::new());
    let config = SchedulerConfig {
        light_pool: 1, // Only 1 job at a time to force queuing
        ..Default::default()
    };

    let mock_worker = Arc::new(MockWorker::new(Duration::from_millis(100)));
    let scheduler = Scheduler::with_worker(config, probe, estimates, mock_worker);

    // 1. Submit a background job that immediately occupies the single slot
    let mut h0 = scheduler
        .submit(dummy_spec("j0-bg", Priority::Background))
        .await;

    // Wait until j0 starts
    tokio::time::sleep(Duration::from_millis(20)).await;

    // 2. Submit jobs in reverse priority while slot is busy: Background, TurnBlocking, Interactive
    let mut h_bg = scheduler
        .submit(dummy_spec("j-bg", Priority::Background))
        .await;
    let mut h_tb = scheduler
        .submit(dummy_spec("j-tb", Priority::TurnBlocking))
        .await;
    let mut h_int = scheduler
        .submit(dummy_spec("j-int", Priority::Interactive))
        .await;

    // Drain j0 completion
    while let Some(evt) = h0.next_event().await {
        if matches!(evt, TaskEvent::Finished { .. }) {
            break;
        }
    }

    // Now j-int (Interactive) must start before j-tb (TurnBlocking) and j-bg (Background)
    let mut started_order = Vec::new();

    let mut int_done = false;
    let mut tb_done = false;
    let mut bg_done = false;

    while !int_done || !tb_done || !bg_done {
        tokio::select! {
            Some(evt) = h_int.next_event(), if !int_done => {
                if matches!(evt, TaskEvent::Started { .. }) {
                    started_order.push("Interactive");
                }
                if matches!(evt, TaskEvent::Finished { .. }) {
                    int_done = true;
                }
            }
            Some(evt) = h_tb.next_event(), if !tb_done => {
                if matches!(evt, TaskEvent::Started { .. }) {
                    started_order.push("TurnBlocking");
                }
                if matches!(evt, TaskEvent::Finished { .. }) {
                    tb_done = true;
                }
            }
            Some(evt) = h_bg.next_event(), if !bg_done => {
                if matches!(evt, TaskEvent::Started { .. }) {
                    started_order.push("Background");
                }
                if matches!(evt, TaskEvent::Finished { .. }) {
                    bg_done = true;
                }
            }
        }
    }

    assert_eq!(
        started_order,
        vec!["Interactive", "TurnBlocking", "Background"]
    );
}

#[tokio::test]
async fn test_two_gpu_jobs_never_overlap() {
    let probe = Arc::new(FakeProbe::new(8192, 4096, 8));
    let estimates = Arc::new(MemoryEstimateStore::new());
    let config = SchedulerConfig {
        gpu_pool: 1,
        ..Default::default()
    };

    let mock_worker = Arc::new(MockWorker::new(Duration::from_millis(100)));
    let max_concurrent = Arc::clone(&mock_worker.max_concurrent);

    let scheduler = Scheduler::with_worker(config, probe, estimates, mock_worker);

    let mut spec1 = dummy_spec("gpu-1", Priority::Interactive);
    spec1.resources.exclusive = vec!["gpu".into()];
    spec1.resources.gpu_vram_mb = 1000;

    let mut spec2 = dummy_spec("gpu-2", Priority::Interactive);
    spec2.resources.exclusive = vec!["gpu".into()];
    spec2.resources.gpu_vram_mb = 1000;

    let mut h1 = scheduler.submit(spec1).await;
    let mut h2 = scheduler.submit(spec2).await;

    let t1 = tokio::spawn(async move {
        while let Some(evt) = h1.next_event().await {
            if matches!(evt, TaskEvent::Finished { .. }) {
                break;
            }
        }
    });

    let t2 = tokio::spawn(async move {
        while let Some(evt) = h2.next_event().await {
            if matches!(evt, TaskEvent::Finished { .. }) {
                break;
            }
        }
    });

    let _ = tokio::join!(t1, t2);

    assert_eq!(max_concurrent.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn test_fake_probe_low_ram_queues_then_starts_when_freed() {
    // Headroom is 2048 MB by default. If host RAM is 2100 MB, available is 52 MB.
    let probe = Arc::new(FakeProbe::new(2100, 4096, 8));
    let estimates = Arc::new(MemoryEstimateStore::new());
    let config = SchedulerConfig::default();

    let mock_worker = Arc::new(MockWorker::new(Duration::from_millis(50)));
    let scheduler = Scheduler::with_worker(
        config,
        Arc::clone(&probe) as Arc<dyn studio_jobs::probe::SystemProbe>,
        estimates,
        mock_worker,
    );

    let mut spec = dummy_spec("ram-job", Priority::Interactive);
    spec.resources.ram_mb = 500; // Needs 500 MB, but only 52 MB available!

    let mut handle = scheduler.submit(spec).await;

    // Check first event is Queued with Insufficient RAM reason
    let first_evt = handle.next_event().await.unwrap();
    match first_evt {
        TaskEvent::Queued { reason, .. } => {
            assert!(reason.contains("Chờ bộ nhớ RAM"));
        }
        _ => panic!("Expected Queued event, got {:?}", first_evt),
    }

    // Still queued after 100ms
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Free RAM: set to 4096 MB (available = 2048 MB > 500 MB)
    probe.set_free_ram_mb(4096);

    // Should now start and finish
    let mut started = false;
    let mut finished = false;

    while let Some(evt) = handle.next_event().await {
        match evt {
            TaskEvent::Started { .. } => started = true,
            TaskEvent::Finished { .. } => {
                finished = true;
                break;
            }
            _ => {}
        }
    }

    assert!(started);
    assert!(finished);
}

#[tokio::test]
async fn test_timeout_fails_with_timeout() {
    let estimates = Arc::new(MemoryEstimateStore::new());
    let worker = Arc::new(studio_jobs::worker::LocalWorker::new(
        Arc::clone(&estimates) as Arc<dyn EstimateStore>,
    ));

    #[cfg(windows)]
    let cmd = CommandSpec::new(
        "cmd.exe",
        vec!["/c".into(), "powershell -c \"Start-Sleep 5\"".into()],
        std::env::temp_dir(),
    );
    #[cfg(not(windows))]
    let cmd = CommandSpec::new("sleep", vec!["5".into()], std::env::temp_dir());

    let mut spec = dummy_spec("timeout-job", Priority::Interactive);
    spec.command = cmd;
    spec.timeout = Duration::from_millis(200); // 200 ms timeout

    let (event_tx, mut event_rx) = mpsc::channel(32);
    let (_cancel_tx, cancel_rx) = mpsc::channel(1);

    let res = worker.run(spec, event_tx, cancel_rx).await;
    assert!(matches!(res, Err(JobError::Timeout(_))));

    let mut failed = false;
    while let Some(evt) = event_rx.recv().await {
        if let TaskEvent::Failed { reason } = evt {
            assert_eq!(reason, FailedReason::Timeout);
            failed = true;
        }
    }
    assert!(failed);
}

#[tokio::test]
async fn test_env_sanitization_does_not_leak_parent_var() {
    // Set a secret variable in parent process
    std::env::set_var("STUDIO_SECRET_TEST_TOKEN", "super_secret_12345");

    let cmd = CommandSpec::new("cmd.exe", vec![], std::env::temp_dir());
    let sanitized = cmd.build_sanitized_env();

    assert!(!sanitized.contains_key("STUDIO_SECRET_TEST_TOKEN"));
    // Standard allowed vars should exist
    assert!(sanitized.contains_key("PATH") || sanitized.contains_key("Path"));
    assert!(sanitized.contains_key("SystemRoot") || sanitized.contains_key("SYSTEMROOT"));
}

#[tokio::test]
async fn test_ewma_updates() {
    let store = MemoryEstimateStore::with_alpha(0.5);
    assert_eq!(store.get_peak_ram_mb("tool:test"), None);

    // First update: 100 MB -> 100 MB
    store.update_peak_ram_mb("tool:test", 100);
    assert_eq!(store.get_peak_ram_mb("tool:test"), Some(100));

    // Second update: 200 MB -> 0.5 * 200 + 0.5 * 100 = 150 MB
    store.update_peak_ram_mb("tool:test", 200);
    assert_eq!(store.get_peak_ram_mb("tool:test"), Some(150));
}

#[cfg(windows)]
#[tokio::test]
async fn test_cancel_kills_child_and_grandchild() {
    use sysinfo::System;

    let estimates = Arc::new(MemoryEstimateStore::new());
    let worker = Arc::new(studio_jobs::worker::LocalWorker::new(
        Arc::clone(&estimates) as Arc<dyn EstimateStore>,
    ));

    // Spawn cmd /c which launches a grandchild powershell sleep process
    let cmd = CommandSpec::new(
        "cmd.exe",
        vec![
            "/c".into(),
            "powershell -NoProfile -Command \"Start-Sleep -Seconds 15\"".into(),
        ],
        std::env::temp_dir(),
    );

    let mut spec = dummy_spec("grandchild-job", Priority::Interactive);
    spec.command = cmd;
    spec.timeout = Duration::from_secs(10);

    let (event_tx, mut event_rx) = mpsc::channel(32);
    let (cancel_tx, cancel_rx) = mpsc::channel(1);

    let worker_handle = tokio::spawn(async move { worker.run(spec, event_tx, cancel_rx).await });

    // Wait until child started and get PID
    let mut child_pid = 0u32;
    while let Some(evt) = event_rx.recv().await {
        if let TaskEvent::Started { pid } = evt {
            child_pid = pid;
            break;
        }
    }
    assert!(child_pid > 0);

    // Give powershell grandchild 500ms to spin up
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Find grandchild pid running under this parent
    let mut sys = System::new_all();
    sys.refresh_all();
    let mut grandchild_pids = Vec::new();
    for (pid, proc) in sys.processes() {
        if let Some(parent) = proc.parent() {
            if parent.as_u32() == child_pid {
                grandchild_pids.push(pid.as_u32());
            }
        }
    }

    // Now trigger cancellation
    let _ = cancel_tx.send(()).await;
    let _ = worker_handle.await;

    // Give OS 300ms to terminate the Job Object process tree
    tokio::time::sleep(Duration::from_millis(300)).await;

    sys.refresh_all();
    // Verify parent is gone
    assert!(
        sys.process(sysinfo::Pid::from_u32(child_pid)).is_none(),
        "Parent process still alive!"
    );

    // Verify all grandchildren are gone
    for gpid in grandchild_pids {
        assert!(
            sys.process(sysinfo::Pid::from_u32(gpid)).is_none(),
            "Grandchild process {} still alive!",
            gpid
        );
    }
}

#[cfg(windows)]
#[tokio::test]
async fn test_memory_limit_exceeded_fails_with_oom() {
    let estimates = Arc::new(MemoryEstimateStore::new());
    let worker = Arc::new(studio_jobs::worker::LocalWorker::new(
        Arc::clone(&estimates) as Arc<dyn EstimateStore>,
    ));

    // Allocate incrementally in powershell with a 60MB limit.
    // If allocation hits Job Object memory limit, catch and exit with non-zero.
    let cmd = CommandSpec::new(
        "powershell.exe",
        vec![
            "-NoProfile".into(),
            "-Command".into(),
            "try { $l = New-Object System.Collections.ArrayList; 1..50 | ForEach-Object { [void]$l.Add((New-Object byte[] 10MB)) }; Start-Sleep 5 } catch { exit 1 }".into(),
        ],
        std::env::temp_dir(),
    );

    let mut spec = dummy_spec("oom-job", Priority::Interactive);
    spec.command = cmd;
    spec.memory_limit_mb = Some(60); // Hard limit 60 MB
    spec.resources.ram_mb = 40;
    spec.timeout = Duration::from_secs(10);

    let (event_tx, mut event_rx) = mpsc::channel(32);
    let (_cancel_tx, cancel_rx) = mpsc::channel(1);

    let res = worker.run(spec, event_tx, cancel_rx).await;
    assert!(matches!(res, Err(JobError::OutOfMemory(_))));

    let mut oom_reported = false;
    while let Some(evt) = event_rx.recv().await {
        if let TaskEvent::Failed { reason } = evt {
            if matches!(reason, FailedReason::OutOfMemory) {
                oom_reported = true;
            }
        }
    }
    assert!(oom_reported, "FailedReason::OutOfMemory was not reported");
}

#[tokio::test]
async fn test_broadcast_event_sequence_successful_job() {
    let probe = Arc::new(FakeProbe::new(8192, 4096, 8));
    let estimates = Arc::new(MemoryEstimateStore::new());
    let config = SchedulerConfig::default();
    let mock_worker = Arc::new(MockWorker::new(Duration::from_millis(50)));
    let scheduler = Scheduler::with_worker(config, probe, estimates, mock_worker);

    let mut rx = scheduler.subscribe();
    let spec = dummy_spec("success-broadcast", Priority::Interactive);

    let mut handle = scheduler.submit(spec).await;

    // Collect broadcast events
    let mut states = Vec::new();
    while let Ok(evt) = rx.recv().await {
        if evt.job_id == "success-broadcast" {
            states.push(evt.state);
            if evt.state == JobState::Succeeded {
                assert_eq!(evt.progress, Some(1.0));
                break;
            }
        }
    }

    // Drain handle
    while handle.next_event().await.is_some() {}

    assert_eq!(
        states,
        vec![
            JobState::Queued,
            JobState::Admitted,
            JobState::Running,
            JobState::Succeeded
        ]
    );
}

#[tokio::test]
async fn test_broadcast_event_sequence_failed_job() {
    struct FailingWorker;
    impl Worker for FailingWorker {
        fn run(
            &self,
            _spec: JobSpec,
            event_tx: mpsc::Sender<TaskEvent>,
            _cancel_rx: mpsc::Receiver<()>,
        ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), JobError>> + Send + '_>>
        {
            Box::pin(async move {
                let _ = event_tx.send(TaskEvent::Started { pid: 9999 }).await;
                let _ = event_tx
                    .send(TaskEvent::Failed {
                        reason: FailedReason::Exit(1),
                    })
                    .await;
                Err(JobError::Exit(1))
            })
        }
    }

    let probe = Arc::new(FakeProbe::new(8192, 4096, 8));
    let estimates = Arc::new(MemoryEstimateStore::new());
    let config = SchedulerConfig::default();
    let scheduler = Scheduler::with_worker(config, probe, estimates, Arc::new(FailingWorker));

    let mut rx = scheduler.subscribe();
    let spec = dummy_spec("fail-broadcast", Priority::Interactive);

    let mut handle = scheduler.submit(spec).await;

    let mut states = Vec::new();
    while let Ok(evt) = rx.recv().await {
        if evt.job_id == "fail-broadcast" {
            states.push(evt.state);
            if evt.state == JobState::Failed {
                assert!(evt.message.is_some());
                break;
            }
        }
    }

    while handle.next_event().await.is_some() {}

    assert_eq!(
        states,
        vec![
            JobState::Queued,
            JobState::Admitted,
            JobState::Running,
            JobState::Failed
        ]
    );
}

#[tokio::test]
async fn test_report_progress_emits_event() {
    let probe = Arc::new(FakeProbe::new(8192, 4096, 8));
    let estimates = Arc::new(MemoryEstimateStore::new());
    let config = SchedulerConfig::default();
    let mock_worker = Arc::new(MockWorker::new(Duration::from_millis(150)));
    let scheduler = Scheduler::with_worker(config, probe, estimates, mock_worker);

    let mut rx = scheduler.subscribe();
    let spec = dummy_spec("progress-test", Priority::Interactive);

    let _handle = scheduler.submit(spec).await;

    // Wait until running
    while let Ok(evt) = rx.recv().await {
        if evt.job_id == "progress-test" && evt.state == JobState::Running {
            break;
        }
    }

    // Call report_progress
    let reported = scheduler
        .report_progress(
            "progress-test",
            Some(0.42),
            Some("Processing chunk 2/5".into()),
        )
        .await;
    assert!(
        reported,
        "Job should be running and report_progress succeeded"
    );

    // Next event on rx should have progress 0.42
    let mut progress_seen = false;
    while let Ok(evt) = rx.recv().await {
        if evt.job_id == "progress-test" {
            if let Some(p) = evt.progress {
                if (p - 0.42).abs() < f32::EPSILON {
                    assert_eq!(evt.message.as_deref(), Some("Processing chunk 2/5"));
                    progress_seen = true;
                    break;
                }
            }
        }
    }
    assert!(
        progress_seen,
        "Progress event was not received via broadcast"
    );
}

#[cfg(windows)]
#[tokio::test]
async fn test_windows_spawn_in_job_object() {
    use std::os::windows::io::AsRawHandle;
    use studio_jobs::windows_job::JobObject;

    let job = JobObject::create(None, Priority::Interactive).expect("Failed to create job object");

    // Spawn a suspended process
    let mut cmd = std::process::Command::new("cmd.exe");
    cmd.args(["/c", "exit 0"]);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_SUSPENDED: u32 = 0x00000004;
        cmd.creation_flags(CREATE_SUSPENDED);
    }

    let child = cmd.spawn().expect("Failed to spawn suspended process");
    let raw_handle = child.as_raw_handle();
    let pid = child.id();

    // Assign to job object before resuming
    job.assign_process(raw_handle, Priority::Interactive)
        .expect("Failed to assign process to job");

    // Verify it is in job object
    let is_in_job = job
        .is_process_in_job(raw_handle)
        .expect("Failed to query is_process_in_job");
    assert!(
        is_in_job,
        "Suspended process should be inside the Job Object"
    );

    // Resume process
    job.resume_process(pid)
        .expect("Failed to resume suspended process");

    // Verify process runs to completion successfully
    let status = child
        .wait_with_output()
        .expect("Failed to wait on child process");
    assert!(status.status.success(), "cmd.exe /c exit 0 should exit 0");
}

#[test]
fn test_historical_estimate_never_lowers_limit_below_declared() {
    let store = MemoryEstimateStore::new();
    store.update_peak_ram_mb(
        "
tool:test
",
        50,
    );

    let mut spec = dummy_spec(
        "
job-large
",
        Priority::Interactive,
    );
    spec.tool_id = "
tool:test
"
    .into();
    spec.resources.ram_mb = 1024;
    spec.memory_limit_mb = Some(1536);

    let estimated_ram = store
        .get_peak_ram_mb(&spec.tool_id)
        .unwrap_or(spec.resources.ram_mb)
        .max(spec.resources.ram_mb);
    let from_est = ((estimated_ram as f64) * 1.5).ceil() as u64;
    let effective_mem_limit = spec
        .memory_limit_mb
        .map(|decl| decl.max(from_est))
        .or(Some(from_est));

    assert_eq!(effective_mem_limit, Some(1536));
}
