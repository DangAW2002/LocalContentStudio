//! Windows Job Object abstractions and helpers.

use std::mem::size_of;
use std::os::windows::io::RawHandle;
use windows_sys::Win32::Foundation::{CloseHandle, HANDLE, INVALID_HANDLE_VALUE};
use windows_sys::Win32::System::JobObjects::{
    AssignProcessToJobObject, CreateJobObjectW, JobObjectCpuRateControlInformation,
    JobObjectExtendedLimitInformation, QueryInformationJobObject, SetInformationJobObject,
    JOBOBJECT_CPU_RATE_CONTROL_INFORMATION, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
    JOB_OBJECT_CPU_RATE_CONTROL_ENABLE, JOB_OBJECT_CPU_RATE_CONTROL_HARD_CAP,
    JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE, JOB_OBJECT_LIMIT_PROCESS_MEMORY,
};
use windows_sys::Win32::System::Threading::{SetPriorityClass, BELOW_NORMAL_PRIORITY_CLASS};

use crate::error::JobError;
use crate::job::Priority;

/// Safe RAII wrapper around a Windows Job Object HANDLE.
pub struct JobObject {
    handle: HANDLE,
}

// Windows HANDLE for Job Objects is safe to send across threads.
unsafe impl Send for JobObject {}
unsafe impl Sync for JobObject {}

impl JobObject {
    /// Create a new Job Object with `KILL_ON_JOB_CLOSE` configured.
    pub fn create(memory_limit_mb: Option<u64>, priority: Priority) -> Result<Self, JobError> {
        let handle = unsafe { CreateJobObjectW(std::ptr::null(), std::ptr::null()) };
        if handle.is_null() || handle == INVALID_HANDLE_VALUE {
            return Err(JobError::SpawnError(format!(
                "Failed to create Job Object: {}",
                std::io::Error::last_os_error()
            )));
        }

        let job = Self { handle };

        // Configure limits
        let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = unsafe { std::mem::zeroed() };
        let mut limit_flags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;

        if let Some(limit_mb) = memory_limit_mb {
            limit_flags |= JOB_OBJECT_LIMIT_PROCESS_MEMORY;
            info.ProcessMemoryLimit = (limit_mb as usize) * 1024 * 1024;
        }

        info.BasicLimitInformation.LimitFlags = limit_flags;

        let ret = unsafe {
            SetInformationJobObject(
                handle,
                JobObjectExtendedLimitInformation,
                &info as *const _ as *const _,
                size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
            )
        };

        if ret == 0 {
            return Err(JobError::SpawnError(format!(
                "Failed to set Job Object extended limits: {}",
                std::io::Error::last_os_error()
            )));
        }

        // Configure CPU throttling for Background priority
        if priority == Priority::Background {
            let mut cpu_info: JOBOBJECT_CPU_RATE_CONTROL_INFORMATION =
                unsafe { std::mem::zeroed() };
            cpu_info.ControlFlags =
                JOB_OBJECT_CPU_RATE_CONTROL_ENABLE | JOB_OBJECT_CPU_RATE_CONTROL_HARD_CAP;
            // 50% cap = 5000 basis points (10000 = 100%)
            cpu_info.Anonymous.CpuRate = 5000;

            let _ret_cpu = unsafe {
                SetInformationJobObject(
                    handle,
                    JobObjectCpuRateControlInformation,
                    &cpu_info as *const _ as *const _,
                    size_of::<JOBOBJECT_CPU_RATE_CONTROL_INFORMATION>() as u32,
                )
            };
        }

        Ok(job)
    }

    /// Assign a process by its OS handle to this Job Object and adjust priority if background.
    pub fn assign_process(
        &self,
        process_handle: RawHandle,
        priority: Priority,
    ) -> Result<(), JobError> {
        let ret = unsafe { AssignProcessToJobObject(self.handle, process_handle as HANDLE) };
        if ret == 0 {
            return Err(JobError::SpawnError(format!(
                "Failed to assign process to Job Object: {}",
                std::io::Error::last_os_error()
            )));
        }

        if priority == Priority::Background {
            unsafe {
                SetPriorityClass(process_handle as HANDLE, BELOW_NORMAL_PRIORITY_CLASS);
            }
        }

        Ok(())
    }

    /// Query the peak process memory observed across processes in this Job Object in megabytes.
    pub fn query_peak_memory_mb(&self) -> u64 {
        let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = unsafe { std::mem::zeroed() };
        let mut ret_len = 0u32;

        let ret = unsafe {
            QueryInformationJobObject(
                self.handle,
                JobObjectExtendedLimitInformation,
                &mut info as *mut _ as *mut _,
                size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
                &mut ret_len,
            )
        };

        if ret != 0 {
            let peak_bytes = info.PeakProcessMemoryUsed.max(info.PeakJobMemoryUsed);
            (peak_bytes / (1024 * 1024)) as u64
        } else {
            0
        }
    }
}

impl Drop for JobObject {
    fn drop(&mut self) {
        if !self.handle.is_null() && self.handle != INVALID_HANDLE_VALUE {
            unsafe {
                CloseHandle(self.handle);
            }
        }
    }
}
