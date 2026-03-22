import { message } from 'ant-design-vue'

export interface ErrorHandlerOptions {
  showMessage?: boolean
  messagePrefix?: string
  rethrow?: boolean
  onError?: (error: any) => void
}

/**
 * 包装异步函数的错误处理工具
 * 自动打印日志并显示 UI 提示
 */
export async function withErrorHandler<T>(
  fn: () => Promise<T>,
  options: ErrorHandlerOptions = {}
): Promise<T | undefined> {
  const {
    showMessage = true,
    messagePrefix = '操作失败',
    rethrow = false,
    onError
  } = options

  try {
    return await fn()
  } catch (error) {
    const errorMessage = error instanceof Error ? error.message : String(error)

    // 控制台记录详细错误
    console.error(`${messagePrefix}:`, error)

    // 显示用户友好的错误消息
    if (showMessage) {
      message.error(`${messagePrefix}: ${errorMessage}`)
    }

    // 执行自定义回调
    if (onError) {
      onError(error)
    }

    // 根据需要重新抛出
    if (rethrow) {
      throw error
    }

    return undefined
  }
}
