use chrono::Local;
use std::{
    backtrace::Backtrace,
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    sync::OnceLock,
};
#[cfg(target_os = "linux")]
use std::os::unix::ffi::OsStrExt;
use tracing_appender::rolling::{RollingFileAppender, Rotation};
pub use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

static LOG_DIR: OnceLock<PathBuf> = OnceLock::new();
static CRASH_DIR: OnceLock<PathBuf> = OnceLock::new();
static PANIC_HOOK_INSTALLED: OnceLock<()> = OnceLock::new();
#[cfg(target_os = "windows")]
static WINDOWS_EXCEPTION_HOOK_INSTALLED: OnceLock<()> = OnceLock::new();
#[cfg(target_os = "linux")]
static LINUX_SIGNAL_HOOK_INSTALLED: OnceLock<()> = OnceLock::new();
#[cfg(target_os = "linux")]
static SIGNAL_CRASH_DIR_PATH: OnceLock<Box<[u8]>> = OnceLock::new();
#[cfg(target_os = "linux")]
static SIGNAL_LATEST_CRASH_PATH: OnceLock<Box<[u8]>> = OnceLock::new();

/// 初始化日志系统
pub fn init_logger(app_dir: PathBuf) -> WorkerGuard {
    let log_dir = app_dir.join("logs");
    let crash_dir = log_dir.join("crashes");
    let _ = fs::create_dir_all(&log_dir);
    let _ = fs::create_dir_all(&crash_dir);
    let _ = LOG_DIR.set(log_dir.clone());
    let _ = CRASH_DIR.set(crash_dir.clone());
    #[cfg(target_os = "linux")]
    cache_signal_log_paths(&log_dir, &crash_dir);

    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,datasmith=debug"));

    let formatting_layer = fmt::layer()
        .with_thread_ids(true)
        .with_thread_names(true)
        .with_target(false)
        .with_line_number(true)
        .pretty();

    let file_appender = RollingFileAppender::new(
        Rotation::DAILY,
        &log_dir,
        "datasmith.log",
    );
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    let file_layer = fmt::layer()
        .with_ansi(false)
        .with_thread_ids(true)
        .with_thread_names(true)
        .with_line_number(true)
        .with_writer(non_blocking);

    let _ = tracing_subscriber::registry()
        .with(env_filter)
        .with(formatting_layer)
        .with(file_layer)
        .try_init();

    install_panic_hook();
    install_platform_crash_hook();

    tracing::info!(
        version = env!("CARGO_PKG_VERSION"),
        os = std::env::consts::OS,
        arch = std::env::consts::ARCH,
        profile = if cfg!(debug_assertions) { "debug" } else { "release" },
        log_dir = %log_dir.display(),
        crash_dir = %crash_dir.display(),
        "日志系统初始化成功"
    );

    guard
}

pub fn write_fatal_report(stage: &str, error: &str) -> Option<PathBuf> {
    let report = format!(
        "DataSmith Fatal Error\n\
         Time: {}\n\
         Stage: {}\n\
         Error: {}\n\n\
         Backtrace:\n{}\n",
        Local::now().format("%Y-%m-%d %H:%M:%S%.3f"),
        stage,
        error,
        Backtrace::force_capture()
    );
    write_crash_report("fatal", &report)
}

fn install_panic_hook() {
    if PANIC_HOOK_INSTALLED.set(()).is_err() {
        return;
    }

    let previous_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let location = panic_info
            .location()
            .map(|loc| format!("{}:{}:{}", loc.file(), loc.line(), loc.column()))
            .unwrap_or_else(|| "unknown".to_string());

        let payload = if let Some(s) = panic_info.payload().downcast_ref::<&str>() {
            (*s).to_string()
        } else if let Some(s) = panic_info.payload().downcast_ref::<String>() {
            s.clone()
        } else {
            "non-string panic payload".to_string()
        };

        let report = format!(
            "DataSmith Panic\n\
             Time: {}\n\
             Thread: {}\n\
             Location: {}\n\
             Payload: {}\n\n\
             Backtrace:\n{}\n",
            Local::now().format("%Y-%m-%d %H:%M:%S%.3f"),
            std::thread::current().name().unwrap_or("unnamed"),
            location,
            payload,
            Backtrace::force_capture()
        );

        let _ = write_crash_report("panic", &report);
        tracing::error!(location = %location, payload = %payload, "应用发生 panic");
        previous_hook(panic_info);
    }));
}

fn write_crash_report(_kind: &str, content: &str) -> Option<PathBuf> {
    let crash_dir = CRASH_DIR.get()?;
    let file_name = format!("crash-{}.log", Local::now().format("%Y%m%d-%H%M%S%.3f"));
    let path = crash_dir.join(file_name);

    write_text_file(&path, content).ok()?;

    if let Some(log_dir) = LOG_DIR.get() {
        let latest_path = log_dir.join("latest-crash.log");
        let _ = write_text_file(&latest_path, content);
    }

    Some(path)
}

fn write_text_file(path: &Path, content: &str) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(path)?;
    file.write_all(content.as_bytes())?;
    file.flush()?;
    Ok(())
}

#[cfg(target_os = "windows")]
fn install_platform_crash_hook() {
    use windows_sys::Win32::System::Diagnostics::Debug::SetUnhandledExceptionFilter;

    if WINDOWS_EXCEPTION_HOOK_INSTALLED.set(()).is_err() {
        return;
    }

    unsafe {
        SetUnhandledExceptionFilter(Some(unhandled_exception_filter));
    }
}

#[cfg(target_os = "linux")]
fn install_platform_crash_hook() {
    if LINUX_SIGNAL_HOOK_INSTALLED.set(()).is_err() {
        return;
    }

    for signal in [
        libc::SIGABRT,
        libc::SIGBUS,
        libc::SIGFPE,
        libc::SIGILL,
        libc::SIGSEGV,
        libc::SIGTRAP,
    ] {
        unsafe {
            let mut action: libc::sigaction = std::mem::zeroed();
            action.sa_flags = libc::SA_SIGINFO | libc::SA_RESETHAND;
            action.sa_sigaction = linux_signal_handler as usize;
            libc::sigemptyset(&mut action.sa_mask);
            libc::sigaction(signal, &action, std::ptr::null_mut());
        }
    }
}

#[cfg(not(any(target_os = "windows", target_os = "linux")))]
fn install_platform_crash_hook() {}

#[cfg(target_os = "windows")]
unsafe extern "system" fn unhandled_exception_filter(
    exception_info: *const windows_sys::Win32::System::Diagnostics::Debug::EXCEPTION_POINTERS,
) -> i32 {
    use windows_sys::Win32::System::Diagnostics::Debug::EXCEPTION_CONTINUE_SEARCH;

    let (code, address) = if exception_info.is_null() {
        (0u32, 0usize)
    } else {
        let record = (*exception_info).ExceptionRecord;
        if record.is_null() {
            (0u32, 0usize)
        } else {
            ((*record).ExceptionCode as u32, (*record).ExceptionAddress as usize)
        }
    };

    let report = format!(
        "DataSmith Windows Crash\n\
         Time: {}\n\
         Thread: {}\n\
         ExceptionCode: 0x{:08X}\n\
         ExceptionAddress: 0x{:X}\n",
        Local::now().format("%Y-%m-%d %H:%M:%S%.3f"),
        std::thread::current().name().unwrap_or("unnamed"),
        code,
        address,
    );

    let _ = write_crash_report("windows-exception", &report);
    EXCEPTION_CONTINUE_SEARCH
}

#[cfg(target_os = "linux")]
fn cache_signal_log_paths(log_dir: &Path, crash_dir: &Path) {
    let _ = SIGNAL_CRASH_DIR_PATH.set(path_to_c_bytes(crash_dir));
    let _ = SIGNAL_LATEST_CRASH_PATH.set(path_to_c_bytes(&log_dir.join("latest-crash.log")));
}

#[cfg(target_os = "linux")]
fn path_to_c_bytes(path: &Path) -> Box<[u8]> {
    let raw = path.as_os_str().as_bytes();
    let mut bytes = Vec::with_capacity(raw.len() + 1);
    bytes.extend_from_slice(raw);
    bytes.push(0);
    bytes.into_boxed_slice()
}

#[cfg(target_os = "linux")]
unsafe extern "C" fn linux_signal_handler(
    signal: libc::c_int,
    info: *mut libc::siginfo_t,
    _context: *mut libc::c_void,
) {
    write_linux_signal_report(signal, info.cast_const());
    libc::signal(signal, libc::SIG_DFL);
    libc::raise(signal);
    libc::_exit(128 + signal);
}

#[cfg(target_os = "linux")]
unsafe fn write_linux_signal_report(signal: libc::c_int, info: *const libc::siginfo_t) {
    let Some(crash_dir) = SIGNAL_CRASH_DIR_PATH.get() else {
        return;
    };

    let mut timespec: libc::timespec = std::mem::zeroed();
    if libc::clock_gettime(libc::CLOCK_REALTIME, &mut timespec) != 0 {
        timespec.tv_sec = 0;
        timespec.tv_nsec = 0;
    }

    let seconds = timespec.tv_sec as i64;
    let millis = (timespec.tv_nsec / 1_000_000) as u32;
    let (year, month, day, hour, minute, second) = unix_to_utc_datetime(seconds);
    let file_name = build_crash_file_name(year, month, day, hour, minute, second, millis);
    let mut crash_path = [0u8; 1024];
    let Some(_path_len) = build_signal_path(crash_dir, &file_name, &mut crash_path) else {
        return;
    };

    let signal_name = linux_signal_name(signal);
    let signal_code = if info.is_null() { 0 } else { (*info).si_code };
    let signal_addr = if info.is_null() { 0usize } else { (*info).si_addr() as usize };
    let pid = libc::getpid();
    let tid = libc::syscall(libc::SYS_gettid) as i64;
    let mut report = [0u8; 1024];
    let mut len = 0usize;
    append_bytes(&mut report, &mut len, b"DataSmith Linux Crash\n");
    append_bytes(&mut report, &mut len, b"Time: ");
    append_datetime(&mut report, &mut len, year, month, day, hour, minute, second, millis);
    append_bytes(&mut report, &mut len, b"\nSignal: ");
    append_bytes(&mut report, &mut len, signal_name);
    append_bytes(&mut report, &mut len, b" (");
    append_i64(&mut report, &mut len, signal as i64);
    append_bytes(&mut report, &mut len, b")\nCode: ");
    append_i64(&mut report, &mut len, signal_code as i64);
    append_bytes(&mut report, &mut len, b"\nPID: ");
    append_i64(&mut report, &mut len, pid as i64);
    append_bytes(&mut report, &mut len, b"\nTID: ");
    append_i64(&mut report, &mut len, tid);
    append_bytes(&mut report, &mut len, b"\nAddress: 0x");
    append_hex_usize(&mut report, &mut len, signal_addr);
    append_bytes(&mut report, &mut len, b"\n");

    write_signal_file(crash_path.as_ptr().cast(), &report[..len]);

    if let Some(latest_path) = SIGNAL_LATEST_CRASH_PATH.get() {
        write_signal_file(latest_path.as_ptr().cast(), &report[..len]);
    }
}

#[cfg(target_os = "linux")]
unsafe fn write_signal_file(path: *const libc::c_char, content: &[u8]) {
    let fd = libc::open(
        path,
        libc::O_CREAT | libc::O_WRONLY | libc::O_TRUNC | libc::O_CLOEXEC,
        0o644,
    );
    if fd < 0 {
        return;
    }

    let _ = libc::write(fd, content.as_ptr().cast(), content.len());
    let _ = libc::fsync(fd);
    let _ = libc::close(fd);
}

#[cfg(target_os = "linux")]
fn unix_to_utc_datetime(seconds: i64) -> (i32, u32, u32, u32, u32, u32) {
    let days = seconds.div_euclid(86_400);
    let secs_of_day = seconds.rem_euclid(86_400);
    let hour = (secs_of_day / 3_600) as u32;
    let minute = ((secs_of_day % 3_600) / 60) as u32;
    let second = (secs_of_day % 60) as u32;
    let (year, month, day) = civil_from_days(days);
    (year, month, day, hour, minute, second)
}

#[cfg(target_os = "linux")]
fn civil_from_days(days: i64) -> (i32, u32, u32) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let month = (mp + if mp < 10 { 3 } else { -9 }) as u32;
    let year = (y + if month <= 2 { 1 } else { 0 }) as i32;
    (year, month, day)
}

#[cfg(target_os = "linux")]
fn build_crash_file_name(
    year: i32,
    month: u32,
    day: u32,
    hour: u32,
    minute: u32,
    second: u32,
    millis: u32,
) -> [u8; 32] {
    let mut name = [0u8; 32];
    let mut pos = 0usize;
    push_ascii(&mut name, &mut pos, b"crash-");
    push_padded_u32(&mut name, &mut pos, year as u32, 4);
    push_padded_u32(&mut name, &mut pos, month, 2);
    push_padded_u32(&mut name, &mut pos, day, 2);
    push_byte(&mut name, &mut pos, b'-');
    push_padded_u32(&mut name, &mut pos, hour, 2);
    push_padded_u32(&mut name, &mut pos, minute, 2);
    push_padded_u32(&mut name, &mut pos, second, 2);
    push_byte(&mut name, &mut pos, b'.');
    push_padded_u32(&mut name, &mut pos, millis, 3);
    push_ascii(&mut name, &mut pos, b".log");
    name
}

#[cfg(target_os = "linux")]
fn build_signal_path(dir: &[u8], file_name: &[u8; 32], out: &mut [u8; 1024]) -> Option<usize> {
    let dir_len = dir.len().checked_sub(1)?;
    let file_len = file_name.iter().position(|b| *b == 0).unwrap_or(file_name.len());
    let total_len = dir_len.checked_add(1)?.checked_add(file_len)?.checked_add(1)?;
    if total_len > out.len() {
        return None;
    }

    out[..dir_len].copy_from_slice(&dir[..dir_len]);
    out[dir_len] = b'/';
    out[dir_len + 1..dir_len + 1 + file_len].copy_from_slice(&file_name[..file_len]);
    out[dir_len + 1 + file_len] = 0;
    Some(dir_len + 1 + file_len)
}

#[cfg(target_os = "linux")]
fn linux_signal_name(signal: libc::c_int) -> &'static [u8] {
    match signal {
        libc::SIGABRT => b"SIGABRT",
        libc::SIGBUS => b"SIGBUS",
        libc::SIGFPE => b"SIGFPE",
        libc::SIGILL => b"SIGILL",
        libc::SIGSEGV => b"SIGSEGV",
        libc::SIGTRAP => b"SIGTRAP",
        _ => b"UNKNOWN",
    }
}

#[cfg(target_os = "linux")]
fn append_datetime(
    out: &mut [u8],
    pos: &mut usize,
    year: i32,
    month: u32,
    day: u32,
    hour: u32,
    minute: u32,
    second: u32,
    millis: u32,
) {
    push_padded_u32(out, pos, year as u32, 4);
    push_byte(out, pos, b'-');
    push_padded_u32(out, pos, month, 2);
    push_byte(out, pos, b'-');
    push_padded_u32(out, pos, day, 2);
    push_byte(out, pos, b' ');
    push_padded_u32(out, pos, hour, 2);
    push_byte(out, pos, b':');
    push_padded_u32(out, pos, minute, 2);
    push_byte(out, pos, b':');
    push_padded_u32(out, pos, second, 2);
    push_byte(out, pos, b'.');
    push_padded_u32(out, pos, millis, 3);
    append_bytes(out, pos, b" UTC");
}

#[cfg(target_os = "linux")]
fn append_bytes(out: &mut [u8], pos: &mut usize, bytes: &[u8]) {
    let available = out.len().saturating_sub(*pos);
    let count = available.min(bytes.len());
    out[*pos..*pos + count].copy_from_slice(&bytes[..count]);
    *pos += count;
}

#[cfg(target_os = "linux")]
fn append_i64(out: &mut [u8], pos: &mut usize, value: i64) {
    if value < 0 {
        push_byte(out, pos, b'-');
    }
    append_u64(out, pos, value.unsigned_abs());
}

#[cfg(target_os = "linux")]
fn append_u64(out: &mut [u8], pos: &mut usize, mut value: u64) {
    let mut digits = [0u8; 20];
    let mut len = 0usize;
    if value == 0 {
        digits[0] = b'0';
        len = 1;
    } else {
        while value > 0 && len < digits.len() {
            digits[len] = b'0' + (value % 10) as u8;
            value /= 10;
            len += 1;
        }
    }
    for idx in (0..len).rev() {
        push_byte(out, pos, digits[idx]);
    }
}

#[cfg(target_os = "linux")]
fn append_hex_usize(out: &mut [u8], pos: &mut usize, value: usize) {
    let width = std::mem::size_of::<usize>() * 2;
    for shift in (0..width).rev() {
        let nibble = ((value >> (shift * 4)) & 0xF) as u8;
        let ch = if nibble < 10 { b'0' + nibble } else { b'A' + nibble - 10 };
        push_byte(out, pos, ch);
    }
}

#[cfg(target_os = "linux")]
fn push_ascii(out: &mut [u8], pos: &mut usize, bytes: &[u8]) {
    append_bytes(out, pos, bytes);
}

#[cfg(target_os = "linux")]
fn push_padded_u32(out: &mut [u8], pos: &mut usize, mut value: u32, width: usize) {
    let mut digits = [b'0'; 10];
    for idx in 0..width {
        let rev = width - 1 - idx;
        digits[rev] = b'0' + (value % 10) as u8;
        value /= 10;
    }
    append_bytes(out, pos, &digits[..width]);
}

#[cfg(target_os = "linux")]
fn push_byte(out: &mut [u8], pos: &mut usize, byte: u8) {
    if *pos < out.len() {
        out[*pos] = byte;
        *pos += 1;
    }
}
