//! Frames per second of the game in front, read the way frame counters do it: Windows
//! reports every Present call of Direct3D 9 to 12 through Event Tracing, so Luna never
//! touches the game itself. Reading those reports needs Luna to run as administrator.
//! Vulkan and OpenGL games present through other paths and are not counted.

use crate::games::FpsReading;
use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{LazyLock, Mutex, MutexGuard};
use windows::core::{GUID, PCWSTR, PWSTR};
use windows::Win32::Foundation::{ERROR_ACCESS_DENIED, ERROR_ALREADY_EXISTS, ERROR_SUCCESS, WIN32_ERROR};
use windows::Win32::System::Diagnostics::Etw::{
    CloseTrace, ControlTraceW, EnableTraceEx2, OpenTraceW, ProcessTrace, StartTraceW, CONTROLTRACE_HANDLE,
    EVENT_CONTROL_CODE_ENABLE_PROVIDER, EVENT_RECORD, EVENT_TRACE_CONTROL_STOP, EVENT_TRACE_LOGFILEW,
    EVENT_TRACE_PROPERTIES, EVENT_TRACE_REAL_TIME_MODE, PROCESSTRACE_HANDLE, PROCESS_TRACE_MODE_EVENT_RECORD,
    PROCESS_TRACE_MODE_REAL_TIME, TRACE_LEVEL_INFORMATION, WNODE_FLAG_TRACED_GUID,
};
use windows::Win32::System::Performance::{QueryPerformanceCounter, QueryPerformanceFrequency};

const SESSION: &str = "Luna FPS";
/// Microsoft-Windows-DXGI: event 42 is the start of a Present (Direct3D 10 to 12).
const DXGI: GUID = GUID::from_u128(0xCA11C036_0102_4A2D_A6AD_F03CFED5D3C9);
/// Microsoft-Windows-D3D9: event 1 is the start of a Present.
const D3D9: GUID = GUID::from_u128(0x783ACA0A_790E_4D7F_8451_AA850511C6B9);
/// Our own session, so it can be found and stopped by name.
const SESSION_GUID: GUID = GUID::from_u128(0x6c756e61_0f95_4d2b_9a7e_3f1c0e5a9b21);
/// How long frame times are kept, in seconds.
const KEEP_SECS: f64 = 5.0;

static WANT: AtomicBool = AtomicBool::new(false);
static FREQ: LazyLock<i64> = LazyLock::new(|| {
    let mut f = 0i64;
    unsafe {
        let _ = QueryPerformanceFrequency(&mut f);
    }
    f.max(1)
});
/// Present times per process, in performance counter ticks.
static STAMPS: LazyLock<Mutex<HashMap<u32, VecDeque<i64>>>> = LazyLock::new(|| Mutex::new(HashMap::new()));
static RUN: LazyLock<Mutex<Run>> = LazyLock::new(|| Mutex::new(Run::default()));

#[derive(Default)]
struct Run {
    trace: Option<PROCESSTRACE_HANDLE>,
    /// Why the session could not start; cleared when the counter is turned off.
    failed: Option<&'static str>,
}

fn lock<T>(m: &'static Mutex<T>) -> MutexGuard<'static, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

fn now() -> i64 {
    let mut t = 0i64;
    unsafe {
        let _ = QueryPerformanceCounter(&mut t);
    }
    t
}

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

unsafe extern "system" fn on_event(rec: *mut EVENT_RECORD) {
    let Some(rec) = (unsafe { rec.as_ref() }) else { return };
    let h = &rec.EventHeader;
    let id = h.EventDescriptor.Id;
    let present = (h.ProviderId == DXGI && id == 42) || (h.ProviderId == D3D9 && id == 1);
    if !present {
        return;
    }
    let keep = (KEEP_SECS * *FREQ as f64) as i64;
    let mut map = lock(&STAMPS);
    let list = map.entry(h.ProcessId).or_default();
    list.push_back(h.TimeStamp);
    while list.front().is_some_and(|&t| h.TimeStamp - t > keep) {
        list.pop_front();
    }
}

/// Properties with room for the session name after them, as the calls want it.
struct Props(Vec<u64>);

impl Props {
    fn new() -> Props {
        let name = wide(SESSION);
        let head = std::mem::size_of::<EVENT_TRACE_PROPERTIES>();
        let bytes = head + name.len() * 2 + 1024;
        let mut buf = vec![0u64; bytes.div_ceil(8)];
        unsafe {
            let p = buf.as_mut_ptr() as *mut EVENT_TRACE_PROPERTIES;
            (*p).Wnode.BufferSize = bytes as u32;
            (*p).Wnode.Flags = WNODE_FLAG_TRACED_GUID;
            (*p).Wnode.ClientContext = 1; // time stamps from the performance counter
            (*p).Wnode.Guid = SESSION_GUID;
            (*p).BufferSize = 64; // KB
            (*p).FlushTimer = 1;
            (*p).LogFileMode = EVENT_TRACE_REAL_TIME_MODE;
            (*p).LoggerNameOffset = head as u32;
            let dst = (buf.as_mut_ptr() as *mut u8).add(head) as *mut u16;
            std::ptr::copy_nonoverlapping(name.as_ptr(), dst, name.len());
        }
        Props(buf)
    }

    fn ptr(&mut self) -> *mut EVENT_TRACE_PROPERTIES {
        self.0.as_mut_ptr() as *mut EVENT_TRACE_PROPERTIES
    }
}

fn stop_by_name() {
    let name = wide(SESSION);
    let mut props = Props::new();
    unsafe {
        let _ = ControlTraceW(CONTROLTRACE_HANDLE { Value: 0 }, PCWSTR(name.as_ptr()), props.ptr(), EVENT_TRACE_CONTROL_STOP);
    }
}

fn start_session() -> Result<PROCESSTRACE_HANDLE, &'static str> {
    let name = wide(SESSION);
    let mut handle = CONTROLTRACE_HANDLE { Value: 0 };
    let mut props = Props::new();
    let mut r: WIN32_ERROR = unsafe { StartTraceW(&mut handle, PCWSTR(name.as_ptr()), props.ptr()) };
    if r == ERROR_ALREADY_EXISTS {
        // Left over from a run that did not quit cleanly.
        stop_by_name();
        props = Props::new();
        r = unsafe { StartTraceW(&mut handle, PCWSTR(name.as_ptr()), props.ptr()) };
    }
    if r == ERROR_ACCESS_DENIED {
        return Err("needsAdmin");
    }
    if r != ERROR_SUCCESS {
        return Err("error");
    }
    let mut any = false;
    for provider in [DXGI, D3D9] {
        let e = unsafe {
            EnableTraceEx2(
                handle,
                &provider,
                EVENT_CONTROL_CODE_ENABLE_PROVIDER.0,
                TRACE_LEVEL_INFORMATION as u8,
                0,
                0,
                0,
                None,
            )
        };
        any |= e == ERROR_SUCCESS;
    }
    if !any {
        stop_by_name();
        return Err("error");
    }
    let mut logger = name.clone();
    let mut log = EVENT_TRACE_LOGFILEW { LoggerName: PWSTR(logger.as_mut_ptr()), ..Default::default() };
    log.Anonymous1.ProcessTraceMode = PROCESS_TRACE_MODE_REAL_TIME | PROCESS_TRACE_MODE_EVENT_RECORD;
    log.Anonymous2.EventRecordCallback = Some(on_event);
    let trace = unsafe { OpenTraceW(&mut log) };
    if trace.Value == u64::MAX {
        stop_by_name();
        return Err("error");
    }
    let spawned = std::thread::Builder::new().name("luna-fps-trace".into()).spawn(move || unsafe {
        // Returns once the session stops or the handle closes.
        let _ = ProcessTrace(&[trace], None, None);
    });
    if spawned.is_err() {
        unsafe {
            let _ = CloseTrace(trace);
        }
        stop_by_name();
        return Err("error");
    }
    Ok(trace)
}

fn stop_session(run: &mut Run) {
    if let Some(trace) = run.trace.take() {
        stop_by_name();
        unsafe {
            let _ = CloseTrace(trace);
        }
        lock(&STAMPS).clear();
    }
}

/// The counter is on in the settings. The session starts and stops in `poll`.
pub fn set_enabled(on: bool) {
    WANT.store(on, Ordering::Relaxed);
}

/// Stops the session, for quitting.
pub fn stop() {
    WANT.store(false, Ordering::Relaxed);
    if let Ok(mut run) = RUN.try_lock() {
        stop_session(&mut run);
    } else {
        stop_by_name();
    }
}

/// What the counter shows now, None while it is off.
pub fn poll() -> Option<FpsReading> {
    let want = WANT.load(Ordering::Relaxed);
    let mut run = lock(&RUN);
    if !want {
        stop_session(&mut run);
        run.failed = None;
        return None;
    }
    if let Some(state) = run.failed {
        return Some(FpsReading { state: state.into(), ..Default::default() });
    }
    if run.trace.is_none() {
        match start_session() {
            Ok(t) => run.trace = Some(t),
            Err(state) => {
                run.failed = Some(state);
                return Some(FpsReading { state: state.into(), ..Default::default() });
            }
        }
    }
    drop(run);

    let pid = super::games::foreground_pid();
    let exe = super::games::exe_of(pid);
    let freq = *FREQ as f64;
    let t = now();
    let mut map = lock(&STAMPS);
    let keep = (KEEP_SECS * freq) as i64;
    map.retain(|_, list| list.back().is_some_and(|&last| t - last < keep));
    let Some(list) = map.get(&pid) else {
        return Some(FpsReading { state: "running".into(), exe, ..Default::default() });
    };
    Some(measure(list, t, freq, exe))
}

/// Frames in the last second, the average frame time and the 1% low over five seconds.
fn measure(list: &VecDeque<i64>, now: i64, freq: f64, exe: String) -> FpsReading {
    let mut r = FpsReading { state: "running".into(), exe, ..Default::default() };
    let Some(&last) = list.back() else { return r };
    // Nothing for a while: the game is paused or in a loading screen.
    if (now - last) as f64 / freq > 2.5 {
        return r;
    }
    let second: Vec<i64> = list.iter().copied().filter(|&s| (now - s) as f64 / freq <= 1.0).collect();
    if second.len() >= 2 {
        let span = (second[second.len() - 1] - second[0]) as f64 / freq;
        if span > 0.0 {
            r.fps = (second.len() - 1) as f32 / span as f32;
            r.ms = 1000.0 * span as f32 / (second.len() - 1) as f32;
        }
    }
    let mut gaps: Vec<f64> = list.iter().zip(list.iter().skip(1)).map(|(a, b)| (b - a) as f64 / freq).collect();
    if gaps.len() >= 10 {
        gaps.sort_by(|a, b| b.total_cmp(a));
        let worst = gaps[(gaps.len() / 100).min(gaps.len() - 1)];
        if worst > 0.0 {
            r.low = (1.0 / worst) as f32;
        }
    }
    r
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn measures_a_steady_sixty() {
        let freq = 1000.0;
        let list: VecDeque<i64> = (0..300).map(|i| i * 1000 / 60).collect();
        let now = *list.back().unwrap();
        let r = measure(&list, now, freq, "game.exe".into());
        assert!((r.fps - 60.0).abs() < 1.5, "{}", r.fps);
        assert!((r.ms - 16.7).abs() < 0.5, "{}", r.ms);
        assert!(r.low > 55.0 && r.low < 65.0, "{}", r.low);
    }

    #[test]
    fn a_paused_game_shows_nothing() {
        let list: VecDeque<i64> = (0..60).map(|i| i * 16).collect();
        let r = measure(&list, 10_000, 1000.0, String::new());
        assert_eq!(r.fps, 0.0);
    }
}
