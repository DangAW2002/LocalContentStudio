use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::{broadcast, mpsc, Mutex, Notify};

use crate::estimate::EstimateStore;
use crate::event::{FailedReason, JobBroadcastEvent, JobState, TaskEvent};
use crate::handle::JobHandle;
use crate::job::{JobId, JobSpec, JobStatus, JobWaitReason};
use crate::probe::SystemProbe;
use crate::resource::{CpuClass, SchedulerConfig};
use crate::warm::WarmPool;
use crate::worker::{LocalWorker, Worker};

/// Internal queued job item.
struct QueuedJob {
    spec: JobSpec,
    submitted_at: Instant,
    event_tx: mpsc::Sender<TaskEvent>,
    cancel_rx: Option<mpsc::Receiver<()>>,
    wait_reason: JobWaitReason,
}

/// Internal active running job state.
struct RunningJob {
    spec: JobSpec,
    pid: u32,
    started_at: Instant,
    cancel_tx: mpsc::Sender<()>,
}

/// Global scheduler state.
struct SchedulerState {
    queued: Vec<QueuedJob>,
    running: HashMap<JobId, RunningJob>,
    finished: HashMap<JobId, JobStatus>,
    exclusive_held: HashSet<String>,
}

/// Multi-pool, priority-aware, resource-constrained job scheduler.
pub struct Scheduler {
    config: SchedulerConfig,
    probe: Arc<dyn SystemProbe>,
    estimates: Arc<dyn EstimateStore>,
    worker: Arc<dyn Worker>,
    warm_pool: Arc<WarmPool>,
    state: Arc<Mutex<SchedulerState>>,
    notify: Arc<Notify>,
    broadcast_tx: broadcast::Sender<JobBroadcastEvent>,
}

impl Scheduler {
    /// Create a new `Scheduler` with the default `LocalWorker`.
    pub fn new(
        config: SchedulerConfig,
        probe: Arc<dyn SystemProbe>,
        estimates: Arc<dyn EstimateStore>,
    ) -> Arc<Self> {
        let worker = Arc::new(LocalWorker::new(Arc::clone(&estimates)));
        Self::with_worker(config, probe, estimates, worker)
    }

    /// Create a new `Scheduler` with a custom `Worker` implementation.
    pub fn with_worker(
        config: SchedulerConfig,
        probe: Arc<dyn SystemProbe>,
        estimates: Arc<dyn EstimateStore>,
        worker: Arc<dyn Worker>,
    ) -> Arc<Self> {
        let warm_pool = Arc::new(WarmPool::new());
        let state = Arc::new(Mutex::new(SchedulerState {
            queued: Vec::new(),
            running: HashMap::new(),
            finished: HashMap::new(),
            exclusive_held: HashSet::new(),
        }));
        let notify = Arc::new(Notify::new());
        let (broadcast_tx, _) = broadcast::channel(512);

        let scheduler = Arc::new(Self {
            config,
            probe,
            estimates,
            worker,
            warm_pool,
            state,
            notify,
            broadcast_tx,
        });

        // Spawn background admission loop
        let s_clone = Arc::clone(&scheduler);
        tokio::spawn(async move {
            s_clone.admission_loop().await;
        });

        scheduler
    }

    /// Subscribe to lifecycle events and progress updates across all jobs.
    pub fn subscribe(&self) -> broadcast::Receiver<JobBroadcastEvent> {
        self.broadcast_tx.subscribe()
    }

    /// Report progress for a running job.
    pub async fn report_progress(
        &self,
        job_id: &str,
        progress: Option<f32>,
        message: Option<String>,
    ) -> bool {
        let state = self.state.lock().await;
        if let Some(running) = state.running.get(job_id) {
            let pool = match running.spec.resources.cpu {
                CpuClass::Heavy => "cpu_heavy".to_string(),
                CpuClass::Light => {
                    if running.spec.tool_id.starts_with("engine:") {
                        "engine".to_string()
                    } else if running.spec.resources.gpu_vram_mb > 0
                        || running.spec.resources.exclusive.iter().any(|e| e == "gpu")
                    {
                        "gpu".to_string()
                    } else {
                        "light".to_string()
                    }
                }
            };
            let _ = self.broadcast_tx.send(JobBroadcastEvent {
                job_id: job_id.to_string(),
                kind: running.spec.tool_id.clone(),
                pool,
                state: JobState::Running,
                progress,
                message,
                peak_memory_bytes: None,
            });
            true
        } else {
            false
        }
    }

    /// Submit a job to the scheduler.
    ///
    /// The job enters the priority-ordered queue and will be admitted when pool slots,
    /// exclusive resources, and system RAM/VRAM are available.
    pub async fn submit(&self, spec: JobSpec) -> JobHandle {
        let (event_tx, rx) = mpsc::channel(128);
        let (cancel_tx, cancel_rx) = mpsc::channel(1);

        let job_id = spec.id.clone();
        let initial_reason = JobWaitReason::PoolFull("Đang phân bổ".into());

        let pool = match spec.resources.cpu {
            CpuClass::Heavy => "cpu_heavy".to_string(),
            CpuClass::Light => {
                if spec.tool_id.starts_with("engine:") {
                    "engine".to_string()
                } else if spec.resources.gpu_vram_mb > 0
                    || spec.resources.exclusive.iter().any(|e| e == "gpu")
                {
                    "gpu".to_string()
                } else {
                    "light".to_string()
                }
            }
        };

        // Emit initial Queued broadcast event
        let _ = self.broadcast_tx.send(JobBroadcastEvent {
            job_id: job_id.clone(),
            kind: spec.tool_id.clone(),
            pool,
            state: JobState::Queued,
            progress: None,
            message: Some(initial_reason.to_vietnamese()),
            peak_memory_bytes: None,
        });

        {
            let mut state = self.state.lock().await;
            state.queued.push(QueuedJob {
                spec,
                submitted_at: Instant::now(),
                event_tx: event_tx.clone(),
                cancel_rx: Some(cancel_rx),
                wait_reason: initial_reason.clone(),
            });

            // Re-sort queue by Priority descending, then submitted_at ascending (FIFO)
            Self::sort_queue(&mut state.queued);
        }

        // Wake admission loop
        self.notify.notify_one();

        JobHandle {
            id: job_id,
            rx,
            cancel_tx,
        }
    }

    /// Request cancellation of a job by ID (whether queued or running).
    pub async fn cancel(&self, job_id: &str) -> bool {
        let mut state = self.state.lock().await;

        // Check if queued
        if let Some(pos) = state.queued.iter().position(|q| q.spec.id == job_id) {
            let item = state.queued.remove(pos);
            let pool = match item.spec.resources.cpu {
                CpuClass::Heavy => "cpu_heavy".to_string(),
                CpuClass::Light => {
                    if item.spec.tool_id.starts_with("engine:") {
                        "engine".to_string()
                    } else if item.spec.resources.gpu_vram_mb > 0
                        || item.spec.resources.exclusive.iter().any(|e| e == "gpu")
                    {
                        "gpu".to_string()
                    } else {
                        "light".to_string()
                    }
                }
            };
            let _ = self.broadcast_tx.send(JobBroadcastEvent {
                job_id: job_id.to_string(),
                kind: item.spec.tool_id.clone(),
                pool,
                state: JobState::Cancelled,
                progress: None,
                message: Some("Tác vụ đã bị hủy khi đang chờ".to_string()),
                peak_memory_bytes: None,
            });
            let _ = item
                .event_tx
                .send(TaskEvent::Failed {
                    reason: FailedReason::Cancelled,
                })
                .await;
            state.finished.insert(
                job_id.to_string(),
                JobStatus::Failed {
                    id: job_id.to_string(),
                    reason: FailedReason::Cancelled,
                },
            );
            return true;
        }

        // Check if running
        if let Some(running) = state.running.get(job_id) {
            let _ = running.cancel_tx.send(()).await;
            return true;
        }

        false
    }

    /// Take a point-in-time snapshot of all tracked jobs (queued, running, finished).
    pub async fn snapshot(&self) -> Vec<JobStatus> {
        let state = self.state.lock().await;
        let mut list = Vec::new();

        for (idx, q) in state.queued.iter().enumerate() {
            list.push(JobStatus::Queued {
                id: q.spec.id.clone(),
                position: idx + 1,
                reason: q.wait_reason.to_vietnamese(),
                priority: q.spec.priority,
            });
        }

        for (id, r) in state.running.iter() {
            list.push(JobStatus::Running {
                id: id.clone(),
                pid: r.pid,
                elapsed_ms: r.started_at.elapsed().as_millis() as u64,
            });
        }

        for (_, f) in state.finished.iter() {
            list.push(f.clone());
        }

        list
    }

    /// Main admission loop running every 2 seconds or triggered on job completion / submission.
    async fn admission_loop(self: Arc<Self>) {
        loop {
            // Reap expired warm workers
            self.warm_pool.reap_idle().await;

            // Attempt admission
            self.try_admit_next().await;

            // Wait for next trigger or 2-second timeout
            tokio::select! {
                _ = self.notify.notified() => {}
                _ = tokio::time::sleep(std::time::Duration::from_secs(2)) => {}
            }
        }
    }

    /// Evaluate queued jobs and admit eligible jobs into execution.
    async fn try_admit_next(&self) {
        let mut state = self.state.lock().await;
        if state.queued.is_empty() {
            return;
        }

        // Calculate current active pool usages
        let mut active_engine = 0usize;
        let mut active_gpu = 0usize;
        let mut active_cpu_heavy = 0usize;
        let mut active_light = 0usize;
        let mut active_rate_keys: HashMap<String, usize> = HashMap::new();

        for (_, r) in state.running.iter() {
            if r.spec.tool_id.starts_with("engine:") {
                active_engine += 1;
            }
            if r.spec.resources.gpu_vram_mb > 0
                || r.spec.resources.exclusive.iter().any(|e| e == "gpu")
            {
                active_gpu += 1;
            }
            match r.spec.resources.cpu {
                CpuClass::Heavy => active_cpu_heavy += 1,
                CpuClass::Light => active_light += 1,
            }
            if let Some(ref rk) = r.spec.resources.rate_key {
                *active_rate_keys.entry(rk.clone()).or_insert(0) += 1;
            }
        }

        let free_ram = self.probe.free_ram_mb();
        let free_vram = self.probe.free_vram_mb();

        let mut admitted_indices = Vec::new();
        let mut currently_exclusive = state.exclusive_held.clone();

        for (idx, item) in state.queued.iter_mut().enumerate() {
            let is_engine = item.spec.tool_id.starts_with("engine:");
            let is_gpu = item.spec.resources.gpu_vram_mb > 0
                || item.spec.resources.exclusive.iter().any(|e| e == "gpu");
            let is_cpu_heavy = item.spec.resources.cpu == CpuClass::Heavy;

            // 1. Check pool capacities
            if is_engine && active_engine >= self.config.engine_pool {
                item.wait_reason = JobWaitReason::PoolFull("Engine".into());
                continue;
            }
            if is_gpu && active_gpu >= self.config.gpu_pool {
                item.wait_reason = JobWaitReason::PoolFull("GPU".into());
                continue;
            }
            if is_cpu_heavy && active_cpu_heavy >= self.config.cpu_heavy_pool {
                item.wait_reason = JobWaitReason::PoolFull("CPU nặng".into());
                continue;
            }
            if !is_cpu_heavy && active_light >= self.config.light_pool {
                item.wait_reason = JobWaitReason::PoolFull("Tác vụ nhẹ".into());
                continue;
            }

            // 2. Check rate limit
            if let Some(ref rk) = item.spec.resources.rate_key {
                let current_rk = active_rate_keys.get(rk).copied().unwrap_or(0);
                if current_rk >= self.config.per_rate_key_limit {
                    item.wait_reason = JobWaitReason::RateLimit(rk.clone());
                    continue;
                }
            }

            // 3. Check exclusive resources
            let mut exclusive_conflict = None;
            for ex in &item.spec.resources.exclusive {
                if currently_exclusive.contains(ex) {
                    exclusive_conflict = Some(ex.clone());
                    break;
                }
            }
            if let Some(conflict) = exclusive_conflict {
                item.wait_reason = JobWaitReason::ExclusiveOccupied(conflict);
                continue;
            }

            // 4. Check RAM headroom & estimate
            let estimated_ram = self
                .estimates
                .get_peak_ram_mb(&item.spec.tool_id)
                .unwrap_or(item.spec.resources.ram_mb)
                .max(item.spec.resources.ram_mb);

            let available_ram = free_ram.saturating_sub(self.config.ram_headroom_mb);
            if available_ram < estimated_ram {
                item.wait_reason = JobWaitReason::InsufficientRam {
                    needed_mb: estimated_ram,
                    available_mb: available_ram,
                };
                continue;
            }

            // 5. Check VRAM headroom & estimate
            let estimated_vram = item.spec.resources.gpu_vram_mb;
            let available_vram = free_vram.saturating_sub(self.config.vram_headroom_mb);
            if estimated_vram > 0 && available_vram < estimated_vram {
                item.wait_reason = JobWaitReason::InsufficientVram {
                    needed_mb: estimated_vram,
                    available_mb: available_vram,
                };
                continue;
            }

            // Admission granted!
            admitted_indices.push(idx);

            // Reserve pool slot for this iteration
            if is_engine {
                active_engine += 1;
            }
            if is_gpu {
                active_gpu += 1;
            }
            if is_cpu_heavy {
                active_cpu_heavy += 1;
            } else {
                active_light += 1;
            }
            if let Some(ref rk) = item.spec.resources.rate_key {
                *active_rate_keys.entry(rk.clone()).or_insert(0) += 1;
            }
            for ex in &item.spec.resources.exclusive {
                currently_exclusive.insert(ex.clone());
            }
        }

        // Apply updated exclusive set
        state.exclusive_held = currently_exclusive;

        // Send Queued updates to remaining jobs
        for (pos, q) in state.queued.iter().enumerate() {
            let _ = q
                .event_tx
                .send(TaskEvent::Queued {
                    position: pos + 1,
                    reason: q.wait_reason.to_vietnamese(),
                })
                .await;
        }

        // Pop admitted jobs from highest index to lowest
        admitted_indices.sort_by(|a, b| b.cmp(a));
        for idx in admitted_indices {
            let mut item = state.queued.remove(idx);
            let cancel_rx = item.cancel_rx.take().unwrap();
            let (running_cancel_tx, running_cancel_rx) = mpsc::channel(1);

            let job_id = item.spec.id.clone();
            let spec_clone = item.spec.clone();

            let pool = match spec_clone.resources.cpu {
                CpuClass::Heavy => "cpu_heavy".to_string(),
                CpuClass::Light => {
                    if spec_clone.tool_id.starts_with("engine:") {
                        "engine".to_string()
                    } else if spec_clone.resources.gpu_vram_mb > 0
                        || spec_clone.resources.exclusive.iter().any(|e| e == "gpu")
                    {
                        "gpu".to_string()
                    } else {
                        "light".to_string()
                    }
                }
            };

            // Emit Admitted broadcast event
            let _ = self.broadcast_tx.send(JobBroadcastEvent {
                job_id: job_id.clone(),
                kind: spec_clone.tool_id.clone(),
                pool: pool.clone(),
                state: JobState::Admitted,
                progress: None,
                message: None,
                peak_memory_bytes: None,
            });

            state.running.insert(
                job_id.clone(),
                RunningJob {
                    spec: item.spec.clone(),
                    pid: 0,
                    started_at: Instant::now(),
                    cancel_tx: running_cancel_tx,
                },
            );

            // Spawn execution task
            let scheduler_state = Arc::clone(&self.state);
            let worker = Arc::clone(&self.worker);
            let notify = Arc::clone(&self.notify);
            let broadcast_tx = self.broadcast_tx.clone();

            tokio::spawn(async move {
                // Forward handle cancellation to running cancel channel
                let forward_cancel_tx = {
                    let st = scheduler_state.lock().await;
                    st.running.get(&job_id).map(|r| r.cancel_tx.clone())
                };

                let mut handle_cancel_rx = cancel_rx;
                if let Some(f_tx) = forward_cancel_tx {
                    tokio::spawn(async move {
                        if handle_cancel_rx.recv().await.is_some() {
                            let _ = f_tx.send(()).await;
                        }
                    });
                }

                // Proxy event stream to intercept Started and terminal events
                let (internal_tx, mut internal_rx) = mpsc::channel::<TaskEvent>(128);
                let client_tx = item.event_tx;
                let jid_clone = job_id.clone();
                let state_clone = Arc::clone(&scheduler_state);
                let b_tx = broadcast_tx.clone();
                let tool_id_clone = spec_clone.tool_id.clone();
                let pool_clone = pool.clone();

                tokio::spawn(async move {
                    while let Some(evt) = internal_rx.recv().await {
                        match &evt {
                            TaskEvent::Started { pid } => {
                                {
                                    let mut st = state_clone.lock().await;
                                    if let Some(r) = st.running.get_mut(&jid_clone) {
                                        r.pid = *pid;
                                    }
                                }
                                let _ = b_tx.send(JobBroadcastEvent {
                                    job_id: jid_clone.clone(),
                                    kind: tool_id_clone.clone(),
                                    pool: pool_clone.clone(),
                                    state: JobState::Running,
                                    progress: None,
                                    message: None,
                                    peak_memory_bytes: None,
                                });
                            }
                            TaskEvent::Progress { progress, message } => {
                                let _ = b_tx.send(JobBroadcastEvent {
                                    job_id: jid_clone.clone(),
                                    kind: tool_id_clone.clone(),
                                    pool: pool_clone.clone(),
                                    state: JobState::Running,
                                    progress: *progress,
                                    message: message.clone(),
                                    peak_memory_bytes: None,
                                });
                            }
                            TaskEvent::Finished {
                                exit_code,
                                peak_memory_mb,
                                duration_ms,
                            } => {
                                {
                                    let mut st = state_clone.lock().await;
                                    st.finished.insert(
                                        jid_clone.clone(),
                                        JobStatus::Finished {
                                            id: jid_clone.clone(),
                                            exit_code: *exit_code,
                                            peak_memory_mb: *peak_memory_mb,
                                            duration_ms: *duration_ms,
                                        },
                                    );
                                }
                                let _ = b_tx.send(JobBroadcastEvent {
                                    job_id: jid_clone.clone(),
                                    kind: tool_id_clone.clone(),
                                    pool: pool_clone.clone(),
                                    state: JobState::Succeeded,
                                    progress: Some(1.0),
                                    message: None,
                                    peak_memory_bytes: Some(*peak_memory_mb * 1024 * 1024),
                                });
                            }
                            TaskEvent::Failed { reason } => {
                                {
                                    let mut st = state_clone.lock().await;
                                    st.finished.insert(
                                        jid_clone.clone(),
                                        JobStatus::Failed {
                                            id: jid_clone.clone(),
                                            reason: reason.clone(),
                                        },
                                    );
                                }
                                let (state, msg) = match reason {
                                    FailedReason::Cancelled => {
                                        (JobState::Cancelled, "Tác vụ đã bị hủy".to_string())
                                    }
                                    FailedReason::OutOfMemory => (
                                        JobState::Failed,
                                        "Vượt quá giới hạn bộ nhớ RAM".to_string(),
                                    ),
                                    FailedReason::Timeout => (
                                        JobState::Failed,
                                        "Quá thời gian chờ (timeout)".to_string(),
                                    ),
                                    FailedReason::SpawnError(err) => {
                                        (JobState::Failed, format!("Lỗi khởi tạo process: {}", err))
                                    }
                                    FailedReason::Exit(code) => (
                                        JobState::Failed,
                                        format!("Tiến trình kết thúc với mã lỗi {}", code),
                                    ),
                                };
                                let _ = b_tx.send(JobBroadcastEvent {
                                    job_id: jid_clone.clone(),
                                    kind: tool_id_clone.clone(),
                                    pool: pool_clone.clone(),
                                    state,
                                    progress: None,
                                    message: Some(msg),
                                    peak_memory_bytes: None,
                                });
                            }
                            _ => {}
                        }
                        let _ = client_tx.send(evt).await;
                    }
                });

                // Run worker
                let _ = worker
                    .run(spec_clone.clone(), internal_tx, running_cancel_rx)
                    .await;

                // Cleanup on finish
                {
                    let mut st = scheduler_state.lock().await;
                    st.running.remove(&job_id);
                    for ex in &spec_clone.resources.exclusive {
                        st.exclusive_held.remove(ex);
                    }
                }

                // Notify admission loop to evaluate waiting jobs
                notify.notify_one();
            });
        }
    }

    /// Sort queued jobs by priority (Interactive > TurnBlocking > Background), then FIFO by submission time.
    fn sort_queue(queue: &mut [QueuedJob]) {
        queue.sort_by(|a, b| {
            b.spec
                .priority
                .cmp(&a.spec.priority)
                .then_with(|| a.submitted_at.cmp(&b.submitted_at))
        });
    }
}
