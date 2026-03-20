use std::path::PathBuf;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};
use tracing_appender::rolling::{RollingFileAppender, Rotation};
pub use tracing_appender::non_blocking::WorkerGuard;

/// 初始化日志系统
pub fn init_logger(app_dir: PathBuf) -> WorkerGuard {
    // 1. 设置环境变量过滤，默认级别为 INFO
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,datasmith=debug"));

    // 2. 终端输出层
    let formatting_layer = fmt::layer()
        .with_thread_ids(true)
        .with_target(false)
        .with_line_number(true)
        .pretty();

    // 3. 文件输出层 (按天滚动)
    let log_dir = app_dir.join("logs");
    let file_appender = RollingFileAppender::new(
        Rotation::DAILY,
        &log_dir,
        "datasmith.log",
    );
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);
    
    let file_layer = fmt::layer()
        .with_ansi(false)
        .with_writer(non_blocking);

    // 4. 注册全局订阅者
    tracing_subscriber::registry()
        .with(env_filter)
        .with(formatting_layer)
        .with(file_layer)
        .init();

    tracing::info!("日志系统初始化成功，存储路径: {:?}", log_dir);
    
    guard
}
