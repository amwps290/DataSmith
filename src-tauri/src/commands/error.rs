/// 将任何实现了 Display 的错误转为 Result<T, String> 的 trait
/// 用于 Tauri 命令层统一错误处理
pub trait ToCommandResult<T> {
    fn to_cmd_result(self) -> Result<T, String>;
}

impl<T, E: std::fmt::Display> ToCommandResult<T> for Result<T, E> {
    fn to_cmd_result(self) -> Result<T, String> {
        self.map_err(|e| e.to_string())
    }
}
