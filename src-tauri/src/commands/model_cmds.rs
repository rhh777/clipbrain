use crate::config::manager as config_manager;
use crate::config::schema::RemoteBackendConfig;
use crate::model::remote::openai_compat::RemoteConfig;
use crate::model::state;
use serde::{Deserialize, Serialize};

/// 前端传入的模型配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfigInput {
    pub name: String,
    pub base_url: String,
    pub api_key: String,
    pub model: String,
    pub timeout_secs: Option<u64>,
    pub max_tokens: Option<u32>,
}

/// 保存模型配置并注册后端（同时持久化到 config.toml）
#[tauri::command]
pub fn save_model_config(config: ModelConfigInput) -> Result<String, String> {
    // 尝试用 keyring 安全存储 API Key
    let key_stored = store_api_key(&config.name, &config.api_key);

    let timeout = config.timeout_secs.unwrap_or(30);
    let max_tokens = config.max_tokens.unwrap_or(2048);

    let remote_config = RemoteConfig {
        name: config.name.clone(),
        base_url: config.base_url.clone(),
        api_key: Some(config.api_key.clone()),
        model: config.model.clone(),
        timeout_secs: timeout,
        max_tokens,
    };

    state::configure_remote_backend(remote_config)?;

    // 持久化到 config.toml
    let backend_cfg = RemoteBackendConfig {
        name: config.name.clone(),
        base_url: config.base_url,
        api_key: if key_stored {
            None
        } else {
            Some(config.api_key)
        },
        model: config.model,
        timeout,
        max_tokens,
    };
    config_manager::update_with(|cfg| {
        if let Some(existing) = cfg
            .model
            .remote_backends
            .iter_mut()
            .find(|b| b.name == backend_cfg.name)
        {
            *existing = backend_cfg.clone();
        } else {
            cfg.model.remote_backends.push(backend_cfg.clone());
        }
        // 首个后端自动设为默认
        if cfg.model.default_backend == "rules" {
            cfg.model.default_backend = backend_cfg.name.clone();
        }
    })?;

    if key_stored {
        Ok(format!(
            "后端 '{}' 已配置（API Key 已安全存储）",
            config.name
        ))
    } else {
        Ok(format!(
            "后端 '{}' 已配置（API Key 仅在内存中）",
            config.name
        ))
    }
}

/// 测试模型连接
#[tauri::command]
pub async fn test_model_connection(name: String) -> Result<String, String> {
    let ok = state::test_connection(&name).await?;
    if ok {
        Ok(format!("后端 '{}' 连接成功", name))
    } else {
        Err(format!("后端 '{}' 连接失败", name))
    }
}

/// 列出所有已配置的后端
#[tauri::command]
pub fn list_model_backends() -> Vec<String> {
    state::list_backends()
}

/// 检查是否有可用的远程后端
#[tauri::command]
pub fn has_model_backend() -> bool {
    state::has_remote_backend()
}

/// 列出已保存的模型配置（从 config.toml）
#[tauri::command]
pub fn list_model_configs() -> Vec<ModelConfigOutput> {
    let cfg = config_manager::get();
    let active = cfg.model.default_backend.clone();
    cfg.model
        .remote_backends
        .iter()
        .map(|b| {
            let api_key = load_api_key(&b.name)
                .or_else(|| b.api_key.clone())
                .unwrap_or_default();
            ModelConfigOutput {
                name: b.name.clone(),
                base_url: b.base_url.clone(),
                api_key,
                model: b.model.clone(),
                timeout_secs: b.timeout,
                max_tokens: b.max_tokens,
                is_active: b.name == active,
            }
        })
        .collect()
}

/// 删除模型配置
#[tauri::command]
pub fn delete_model_config(name: String) -> Result<String, String> {
    // 从内存路由器移除
    let _ = state::remove_remote_backend(&name);
    // 从 keyring 移除
    let _ = remove_api_key(&name);
    // 从 config.toml 移除
    config_manager::update_with(|cfg| {
        cfg.model.remote_backends.retain(|b| b.name != name);
        if cfg.model.default_backend == name {
            cfg.model.default_backend = cfg
                .model
                .remote_backends
                .first()
                .map(|b| b.name.clone())
                .unwrap_or_else(|| "rules".to_string());
        }
    })?;
    Ok(format!("后端 '{}' 已删除", name))
}

/// 切换活跃模型配置
#[tauri::command]
pub fn set_active_model(name: String) -> Result<String, String> {
    // 验证配置存在
    let cfg = config_manager::get();
    if !cfg.model.remote_backends.iter().any(|b| b.name == name) {
        return Err(format!("配置 '{}' 不存在", name));
    }
    // 更新内存路由器默认后端
    state::set_default_backend(&name)?;
    // 持久化
    config_manager::update_with(|cfg| {
        cfg.model.default_backend = name.clone();
    })?;
    Ok(format!("已切换到 '{}'", name))
}

/// 快速配置：一步完成保存+测试
#[tauri::command]
pub async fn setup_and_test_model(config: ModelConfigInput) -> Result<String, String> {
    save_model_config(config.clone())?;

    match state::test_connection(&config.name).await {
        Ok(true) => Ok(format!("后端 '{}' 配置成功且连接正常 ✓", config.name)),
        Ok(false) => Err(format!("后端 '{}' 已保存但连接测试失败", config.name)),
        Err(e) => Err(format!(
            "后端 '{}' 已保存但连接测试出错: {}",
            config.name, e
        )),
    }
}

// --- 输出结构 ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfigOutput {
    pub name: String,
    pub base_url: String,
    pub api_key: String,
    pub model: String,
    pub timeout_secs: u64,
    pub max_tokens: u32,
    pub is_active: bool,
}

// --- keyring 辅助 ---

fn store_api_key(name: &str, api_key: &str) -> bool {
    let service = format!("clipbrain-{}", name);
    match keyring::Entry::new(&service, "api_key") {
        Ok(entry) => entry.set_password(api_key).is_ok(),
        Err(_) => false,
    }
}

fn load_api_key(name: &str) -> Option<String> {
    let service = format!("clipbrain-{}", name);
    keyring::Entry::new(&service, "api_key")
        .ok()
        .and_then(|entry| entry.get_password().ok())
}

fn remove_api_key(name: &str) -> bool {
    let service = format!("clipbrain-{}", name);
    keyring::Entry::new(&service, "api_key")
        .ok()
        .and_then(|entry| entry.delete_credential().ok())
        .is_some()
}

/// 启动时从 config.toml 加载已保存的远程后端并注册到内存路由器
pub fn restore_backends_from_config() {
    let cfg = config_manager::get();
    for backend in &cfg.model.remote_backends {
        let api_key = load_api_key(&backend.name).or_else(|| backend.api_key.clone());
        let remote = RemoteConfig {
            name: backend.name.clone(),
            base_url: backend.base_url.clone(),
            api_key,
            model: backend.model.clone(),
            timeout_secs: backend.timeout,
            max_tokens: backend.max_tokens,
        };
        match state::configure_remote_backend(remote) {
            Ok(_) => log::info!("已恢复模型后端: {}", backend.name),
            Err(e) => log::warn!("恢复模型后端 '{}' 失败: {}", backend.name, e),
        }
    }
    // 恢复默认后端设置
    if cfg.model.default_backend != "rules" {
        let _ = state::set_default_backend(&cfg.model.default_backend);
    }
}
