use serde::{Deserialize, Serialize};

/// 统一错误码
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ErrorCode {
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

/// 统一应用错误 — 所有 IPC 命令返回此类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppError {
    pub code: ErrorCode,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

impl AppError {
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            detail: None,
        }
    }

    pub fn with_detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{:?}] {}", self.code, self.message)
    }
}

impl std::error::Error for AppError {}

// --- 便捷构造函数 ---

impl AppError {
    pub fn clipboard_access() -> Self {
        Self::new(ErrorCode::ClipboardAccessDenied, "无法访问剪贴板")
    }

    pub fn clipboard_empty() -> Self {
        Self::new(ErrorCode::ClipboardEmpty, "剪贴板为空")
    }

    pub fn action_not_found(id: &str) -> Self {
        Self::new(ErrorCode::ActionNotFound, format!("操作 '{}' 未找到", id))
    }

    pub fn action_failed(msg: impl Into<String>) -> Self {
        Self::new(ErrorCode::ActionExecutionFailed, msg)
    }

    pub fn model_not_configured() -> Self {
        Self::new(ErrorCode::ModelNotConfigured, "未配置推理后端")
    }

    pub fn api_key_missing(backend: &str) -> Self {
        Self::new(
            ErrorCode::ApiKeyMissing,
            format!("后端 '{}' 缺少 API Key", backend),
        )
    }

    pub fn timeout(msg: impl Into<String>) -> Self {
        Self::new(ErrorCode::Timeout, msg)
    }

    pub fn config_load(err: impl std::fmt::Display) -> Self {
        Self::new(
            ErrorCode::ConfigLoadFailed,
            format!("配置加载失败: {}", err),
        )
    }

    pub fn config_save(err: impl std::fmt::Display) -> Self {
        Self::new(
            ErrorCode::ConfigSaveFailed,
            format!("配置保存失败: {}", err),
        )
    }
}

// --- 从常见错误类型自动转换 ---

impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self {
        Self::new(ErrorCode::Unknown, e.to_string())
    }
}

impl From<serde_json::Error> for AppError {
    fn from(e: serde_json::Error) -> Self {
        Self::new(ErrorCode::InvalidInput, format!("JSON 解析失败: {}", e))
    }
}

/// IPC 命令统一返回类型
pub type AppResult<T> = Result<T, AppError>;
