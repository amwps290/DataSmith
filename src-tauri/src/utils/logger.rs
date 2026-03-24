use chrono::Local;
use std::{
    backtrace::Backtrace,
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    sync::OnceLock,
};
use tracing_appender::rolling::{RollingFileAppender, Rotation};
pub use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

static LOG_DIR: OnceLock<PathBuf> = OnceLock::new();
static CRASH_DIR: OnceLock<PathBuf> = OnceLock::new();
static PANIC_HOOK_INSTALLED: OnceLock<()> = OnceLock::new();
#[cfg(target_os = "windows")]
static WINDOWS_EXCEPTION_HOOK_INSTALLED: OnceLock<()> = OnceLock::new();

/// 初始化日志系统
pub fn init_logger(app_dir: PathBuf) -> WorkerGuard {
    let log_dir = app_dir.join("logs");
    let crash_dir = log_dir.join("crashes");
    let _ = fs::create_dir_all(&log_dir);
    let _ = fs::create_dir_all(&crash_dir);
    let _ = LOG_DIR.set(log_dir.clone());
    let _ = CRASH_DIR.set(crash_dir.clone());

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

fn write_crash_report(kind: &str, content: &str) -> Option<PathBuf> {
    let crash_dir = CRASH_DIR.get()?;
    let file_name = format!(
        "{}-{}.log",
        Local::now().format("%Y%m%d-%H%M%S%.3f"),
        kind
    );
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

#[cfg(not(target_os = "windows"))]
fn install_platform_crash_hook() {}

#[cfg(target_os = "windows")]
unsafe extern "system" fn unhandled_exception_filter(
    exception_info: *mut windows_sys::Win32::System::Diagnostics::Debug::EXCEPTION_POINTERS,
) -> i32 {
    use windows_sys::Win32::Foundation::EXCEPTION_CONTINUE_SEARCH;

    let (code, address) = if exception_info.is_null() {
        (0u32, 0usize)
    } else {
        let record = (*exception_info).ExceptionRecord;
        if record.is_null() {
            (0u32, 0usize)
        } else {
            ((*record).ExceptionCode, (*record).ExceptionAddress as usize)
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
