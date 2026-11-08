use serde::Serialize;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessSample {
    pid: u32,
    name: String,
    working_set_bytes: Option<u64>,
    private_bytes: Option<u64>,
    cpu_seconds: Option<f64>,
    created_at_100ns: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceSample {
    processes: Vec<ProcessSample>,
    sampled_at_ms: u64,
    logical_processors: usize,
    unavailable: usize,
}

#[cfg(windows)]
pub fn environment_pids(
    environment: &webview2_com::Microsoft::Web::WebView2::Win32::ICoreWebView2Environment,
) -> Result<Vec<u32>, String> {
    use webview2_com::Microsoft::Web::WebView2::Win32::ICoreWebView2Environment8;
    use windows::core::Interface;
    let environment = environment
        .cast::<ICoreWebView2Environment8>()
        .map_err(|e| e.to_string())?;
    unsafe {
        let processes = environment.GetProcessInfos().map_err(|e| e.to_string())?;
        let mut count = 0;
        processes.Count(&mut count).map_err(|e| e.to_string())?;
        if count > 4096 {
            return Err("Browser process inventory exceeds its limit.".into());
        }
        (0..count)
            .map(|index| {
                let process = processes
                    .GetValueAtIndex(index)
                    .map_err(|e| e.to_string())?;
                let mut pid = 0;
                process.ProcessId(&mut pid).map_err(|e| e.to_string())?;
                u32::try_from(pid).map_err(|e| e.to_string())
            })
            .collect()
    }
}

/// An on-demand snapshot of the owned process tree. Shared browser processes are counted once.
#[cfg(windows)]
pub fn sample_roots(roots: &[u32]) -> Result<ResourceSample, String> {
    use std::{collections::BTreeSet, mem::size_of};
    use windows::Win32::{
        Foundation::{CloseHandle, FILETIME},
        System::{
            Diagnostics::ToolHelp::{
                CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW,
                TH32CS_SNAPPROCESS,
            },
            ProcessStatus::{
                K32GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS, PROCESS_MEMORY_COUNTERS_EX,
            },
            Threading::{
                GetProcessTimes, OpenProcess, PROCESS_QUERY_INFORMATION,
                PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_VM_READ,
            },
        },
    };
    struct Handle(windows::Win32::Foundation::HANDLE);
    impl Drop for Handle {
        fn drop(&mut self) {
            unsafe {
                let _ = CloseHandle(self.0);
            }
        }
    }
    let snapshot = Handle(
        unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) }.map_err(|e| e.to_string())?,
    );
    let mut entry = PROCESSENTRY32W {
        dwSize: size_of::<PROCESSENTRY32W>() as u32,
        ..Default::default()
    };
    let mut inventory = Vec::new();
    let mut more = unsafe { Process32FirstW(snapshot.0, &mut entry) }.is_ok();
    while more {
        if inventory.len() >= 32768 {
            return Err("Process inventory exceeds its limit.".into());
        }
        let end = entry
            .szExeFile
            .iter()
            .position(|c| *c == 0)
            .unwrap_or(entry.szExeFile.len());
        inventory.push((
            entry.th32ProcessID,
            entry.th32ParentProcessID,
            String::from_utf16_lossy(&entry.szExeFile[..end]),
        ));
        more = unsafe { Process32NextW(snapshot.0, &mut entry) }.is_ok();
    }
    let mut owned: BTreeSet<u32> = roots.iter().copied().collect();
    loop {
        let count = owned.len();
        for (pid, parent, _) in &inventory {
            if owned.contains(parent) {
                owned.insert(*pid);
            }
        }
        if owned.len() == count {
            break;
        }
    }
    let mut unavailable = 0;
    let processes = inventory
        .into_iter()
        .filter(|(pid, _, _)| owned.contains(pid))
        .map(|(pid, _, name)| {
            let handle =
                unsafe { OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_VM_READ, false, pid) }
                    .or_else(|_| unsafe {
                        OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid)
                    })
                    .ok()
                    .map(Handle);
            let mut sample = ProcessSample {
                pid,
                name,
                working_set_bytes: None,
                private_bytes: None,
                cpu_seconds: None,
                created_at_100ns: None,
            };
            if let Some(handle) = handle {
                let mut memory = PROCESS_MEMORY_COUNTERS_EX {
                    cb: size_of::<PROCESS_MEMORY_COUNTERS_EX>() as u32,
                    ..Default::default()
                };
                if unsafe {
                    K32GetProcessMemoryInfo(
                        handle.0,
                        &mut memory as *mut _ as *mut PROCESS_MEMORY_COUNTERS,
                        memory.cb,
                    )
                }
                .as_bool()
                {
                    sample.working_set_bytes = Some(memory.WorkingSetSize as u64);
                    sample.private_bytes = Some(memory.PrivateUsage as u64);
                }
                let (mut created, mut exited, mut kernel, mut user) = (
                    FILETIME::default(),
                    FILETIME::default(),
                    FILETIME::default(),
                    FILETIME::default(),
                );
                if unsafe {
                    GetProcessTimes(handle.0, &mut created, &mut exited, &mut kernel, &mut user)
                }
                .is_ok()
                {
                    let ticks = |time: FILETIME| {
                        ((time.dwHighDateTime as u64) << 32) | time.dwLowDateTime as u64
                    };
                    sample.cpu_seconds = Some((ticks(kernel) + ticks(user)) as f64 / 10_000_000.0);
                    sample.created_at_100ns = Some(ticks(created).to_string());
                }
            }
            if sample.private_bytes.is_none() || sample.cpu_seconds.is_none() {
                unavailable += 1;
            }
            sample
        })
        .collect();
    Ok(ResourceSample {
        processes,
        unavailable,
        sampled_at_ms: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| e.to_string())?
            .as_millis() as u64,
        logical_processors: std::thread::available_parallelism()
            .map(usize::from)
            .unwrap_or(1),
    })
}

#[cfg(not(windows))]
pub fn sample() -> Result<ResourceSample, String> {
    Err("Resource sampling is supported in the Windows app.".into())
}
