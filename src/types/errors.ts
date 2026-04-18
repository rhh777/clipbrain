/** 统一错误码 — 与 Rust 后端 ErrorCode 对应 */
export enum ErrorCode {
  // 通用错误 1xxx
  Unknown = 1000,
  InvalidInput = 1001,
  NotFound = 1002,
  PermissionDenied = 1003,
  Timeout = 1004,

  // 剪贴板错误 2xxx
  ClipboardAccessDenied = 2001,
  ClipboardEmpty = 2002,
  ClipboardWriteFailed = 2003,

  // 分类器错误 3xxx
  ClassifyFailed = 3001,

  // Action 错误 4xxx
  ActionNotFound = 4001,
  ActionExecutionFailed = 4002,
  ActionTimeout = 4003,

  // 模型/推理错误 5xxx
  ModelNotConfigured = 5001,
  ModelConnectionFailed = 5002,
  ModelRequestFailed = 5003,
  ModelResponseInvalid = 5004,
  ApiKeyMissing = 5005,

  // 配置错误 6xxx
  ConfigLoadFailed = 6001,
  ConfigSaveFailed = 6002,
  ConfigInvalid = 6003,
}

/** 统一应用错误 — 与 Rust 后端 AppError 对应 */
export interface AppError {
  code: ErrorCode;
  message: string;
  detail?: string;
}

/** 判断是否为 AppError */
export function isAppError(err: unknown): err is AppError {
  return (
    typeof err === "object" &&
    err !== null &&
    "code" in err &&
    "message" in err
  );
}

/** 从 IPC 调用异常中提取 AppError */
export function parseIpcError(err: unknown): AppError {
  if (isAppError(err)) return err;

  if (typeof err === "string") {
    try {
      const parsed = JSON.parse(err);
      if (isAppError(parsed)) return parsed;
    } catch {
      // not JSON
    }
    return { code: ErrorCode.Unknown, message: err };
  }

  return {
    code: ErrorCode.Unknown,
    message: err instanceof Error ? err.message : "未知错误",
  };
}

/** 错误码 → 用户友好描述 */
export function errorMessage(code: ErrorCode): string {
  const map: Record<number, string> = {
    [ErrorCode.ClipboardAccessDenied]: "无法访问剪贴板，请检查系统权限",
    [ErrorCode.ClipboardEmpty]: "剪贴板为空",
    [ErrorCode.ActionNotFound]: "找不到该操作",
    [ErrorCode.ActionExecutionFailed]: "操作执行失败",
    [ErrorCode.ActionTimeout]: "操作超时",
    [ErrorCode.ModelNotConfigured]: "未配置 AI 模型后端",
    [ErrorCode.ApiKeyMissing]: "缺少 API Key",
    [ErrorCode.ModelConnectionFailed]: "无法连接到模型服务",
    [ErrorCode.Timeout]: "请求超时",
  };
  return map[code] ?? "未知错误";
}
