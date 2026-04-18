use crate::model::model_manager::{self, LocalModelInfo, ModelRegistryEntry};
use tauri::{AppHandle, Emitter};

/// 获取推荐模型列表
#[tauri::command]
pub fn list_recommended_models() -> Vec<ModelRegistryEntry> {
    model_manager::recommended_models()
}

/// 获取已下载模型列表
#[tauri::command]
pub fn list_downloaded_models() -> Vec<LocalModelInfo> {
    model_manager::list_downloaded_models()
}

/// 删除已下载模型
#[tauri::command]
pub fn delete_model(filename: String) -> Result<(), String> {
    model_manager::delete_model(&filename)
}

/// 下载模型（异步，通过事件推送进度）
#[tauri::command]
pub async fn download_model(
    app: AppHandle,
    model_id: String,
    url: String,
    filename: String,
) -> Result<String, String> {
    let mid = model_id.clone();
    model_manager::download_model(
        &url,
        &filename,
        move |progress| {
            let _ = app.emit("model-download-progress", &progress);
        },
        &mid,
    )
    .await
}

/// 获取模型目录路径
#[tauri::command]
pub fn get_models_dir() -> String {
    model_manager::models_dir().to_string_lossy().to_string()
}
