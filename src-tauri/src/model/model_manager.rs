use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// 推荐模型清单条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelRegistryEntry {
    pub id: String,
    pub name: String,
    pub size_mb: u64,
    pub description: String,
    pub format: String, // "gguf" | "mlx"
    pub download_url: String,
    pub filename: String,
}

/// 已下载的本地模型信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalModelInfo {
    pub id: String,
    pub name: String,
    pub filename: String,
    pub size_mb: u64,
    pub format: String,
    pub path: String,
}

/// 模型下载进度
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadProgress {
    pub model_id: String,
    pub downloaded_bytes: u64,
    pub total_bytes: u64,
    pub percent: f32,
}

/// 获取模型目录
pub fn models_dir() -> PathBuf {
    let base = dirs::home_dir().expect("无法获取 HOME 目录");
    let dir = base.join(".clipbrain").join("models");
    let _ = std::fs::create_dir_all(&dir);
    dir
}

/// 内置推荐模型列表
pub fn recommended_models() -> Vec<ModelRegistryEntry> {
    vec![
        ModelRegistryEntry {
            id: "qwen2.5-1.5b-instruct".to_string(),
            name: "Qwen2.5 1.5B Instruct (GGUF Q4)".to_string(),
            size_mb: 1100,
            description: "轻量级中英文模型，适合翻译、摘要等基础任务".to_string(),
            format: "gguf".to_string(),
            download_url: "https://huggingface.co/Qwen/Qwen2.5-1.5B-Instruct-GGUF/resolve/main/qwen2.5-1.5b-instruct-q4_k_m.gguf".to_string(),
            filename: "qwen2.5-1.5b-instruct-q4_k_m.gguf".to_string(),
        },
        ModelRegistryEntry {
            id: "qwen2.5-3b-instruct".to_string(),
            name: "Qwen2.5 3B Instruct (GGUF Q4)".to_string(),
            size_mb: 2100,
            description: "中等大小中英文模型，平衡质量与速度".to_string(),
            format: "gguf".to_string(),
            download_url: "https://huggingface.co/Qwen/Qwen2.5-3B-Instruct-GGUF/resolve/main/qwen2.5-3b-instruct-q4_k_m.gguf".to_string(),
            filename: "qwen2.5-3b-instruct-q4_k_m.gguf".to_string(),
        },
        ModelRegistryEntry {
            id: "qwen2.5-7b-instruct".to_string(),
            name: "Qwen2.5 7B Instruct (GGUF Q4)".to_string(),
            size_mb: 4700,
            description: "高质量中英文模型，适合复杂推理任务".to_string(),
            format: "gguf".to_string(),
            download_url: "https://huggingface.co/Qwen/Qwen2.5-7B-Instruct-GGUF/resolve/main/qwen2.5-7b-instruct-q4_k_m.gguf".to_string(),
            filename: "qwen2.5-7b-instruct-q4_k_m.gguf".to_string(),
        },
    ]
}

/// 列出已下载的模型
pub fn list_downloaded_models() -> Vec<LocalModelInfo> {
    let dir = models_dir();
    let mut models = Vec::new();
    let registry = recommended_models();

    if let Ok(entries) = std::fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }

            let filename = path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            let size_bytes = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
            let size_mb = size_bytes / (1024 * 1024);

            // 尝试从推荐列表匹配
            let (id, name, format) = registry
                .iter()
                .find(|r| r.filename == filename)
                .map(|r| (r.id.clone(), r.name.clone(), r.format.clone()))
                .unwrap_or_else(|| {
                    let fmt = if filename.ends_with(".gguf") {
                        "gguf"
                    } else {
                        "unknown"
                    };
                    (filename.clone(), filename.clone(), fmt.to_string())
                });

            models.push(LocalModelInfo {
                id,
                name,
                filename,
                size_mb,
                format,
                path: path.to_string_lossy().to_string(),
            });
        }
    }

    models
}

/// 删除已下载的模型
pub fn delete_model(filename: &str) -> Result<(), String> {
    let path = models_dir().join(filename);
    if !path.exists() {
        return Err(format!("模型文件不存在: {}", filename));
    }
    std::fs::remove_file(&path).map_err(|e| format!("删除模型失败: {}", e))?;
    log::info!("已删除模型: {}", filename);
    Ok(())
}

/// 下载模型（同步，用于 IPC 命令中 spawn_blocking 调用）
pub async fn download_model(
    url: &str,
    filename: &str,
    on_progress: impl Fn(DownloadProgress) + Send + 'static,
    model_id: &str,
) -> Result<String, String> {
    let path = models_dir().join(filename);

    let client = reqwest::Client::new();
    let resp = client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("下载请求失败: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("下载失败，HTTP 状态: {}", resp.status()));
    }

    let total = resp.content_length().unwrap_or(0);
    let mut downloaded: u64 = 0;
    let model_id_owned = model_id.to_string();

    let mut file = tokio::fs::File::create(&path)
        .await
        .map_err(|e| format!("创建文件失败: {}", e))?;

    use tokio::io::AsyncWriteExt;
    let mut stream = resp.bytes_stream();
    use futures_util::StreamExt;

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| format!("下载中断: {}", e))?;
        file.write_all(&chunk)
            .await
            .map_err(|e| format!("写入失败: {}", e))?;

        downloaded += chunk.len() as u64;
        let percent = if total > 0 {
            (downloaded as f32 / total as f32) * 100.0
        } else {
            0.0
        };

        on_progress(DownloadProgress {
            model_id: model_id_owned.clone(),
            downloaded_bytes: downloaded,
            total_bytes: total,
            percent,
        });
    }

    file.flush()
        .await
        .map_err(|e| format!("文件刷新失败: {}", e))?;
    log::info!("模型下载完成: {} -> {}", filename, path.display());

    Ok(path.to_string_lossy().to_string())
}
