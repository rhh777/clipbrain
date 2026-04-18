use super::backend::InferenceBackend;
use std::collections::HashMap;
use std::sync::Arc;

/// 推理路由器 — 根据配置选择后端
pub struct InferenceRouter {
    backends: HashMap<String, Arc<dyn InferenceBackend>>,
    default_backend: String,
}

impl InferenceRouter {
    pub fn new(default_backend: String) -> Self {
        Self {
            backends: HashMap::new(),
            default_backend,
        }
    }

    pub fn register_backend(&mut self, name: String, backend: Arc<dyn InferenceBackend>) {
        self.backends.insert(name, backend);
    }

    pub fn remove_backend(&mut self, name: &str) {
        self.backends.remove(name);
    }

    /// 获取默认后端的 Arc 引用（可在释放锁后使用）
    pub fn get_default_backend(&self) -> Option<Arc<dyn InferenceBackend>> {
        self.backends.get(&self.default_backend).cloned()
    }

    /// 获取指定后端的 Arc 引用
    pub fn get_backend(&self, name: &str) -> Option<Arc<dyn InferenceBackend>> {
        self.backends.get(name).cloned()
    }

    pub fn default_backend_name(&self) -> &str {
        &self.default_backend
    }

    pub fn set_default_backend(&mut self, name: String) {
        self.default_backend = name;
    }

    /// 列出所有已注册的后端名称
    pub fn list_backends(&self) -> Vec<String> {
        self.backends.keys().cloned().collect()
    }

    pub fn has_remote_backend(&self) -> bool {
        !self.backends.is_empty()
    }
}
