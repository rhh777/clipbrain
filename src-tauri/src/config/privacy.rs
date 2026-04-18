use super::manager;

/// 检查是否应该跳过当前剪贴板内容（隐私保护）
pub fn should_skip_clipboard() -> bool {
    let config = manager::get();

    // 检查前台应用是否在排除列表中
    if let Some(app_name) = get_frontmost_app() {
        if config
            .privacy
            .excluded_apps
            .iter()
            .any(|excluded| app_name.to_lowercase().contains(&excluded.to_lowercase()))
        {
            println!("[ClipBrain] 跳过: 当前应用 '{}' 在排除列表中", app_name);
            return true;
        }
    }

    false
}

/// 检查内容是否可以发送到远程 API
pub fn can_send_to_remote(content: &str) -> bool {
    let config = manager::get();

    // 如果隐私配置禁止记录敏感信息，检测内容中是否包含常见敏感模式
    if !config.privacy.log_sensitive {
        if contains_sensitive_patterns(content) {
            println!("[ClipBrain] 隐私保护: 检测到敏感内容，阻止远程发送");
            return false;
        }
    }

    true
}

/// 检测内容中是否包含敏感模式
fn contains_sensitive_patterns(content: &str) -> bool {
    use crate::classifier::patterns::*;

    let trimmed = content.trim();

    // 纯身份证号
    if ID_CARD_RE.is_match(trimmed) {
        return true;
    }

    // 包含多个手机号（可能是通讯录泄露）
    let phone_count = PHONE_RE.find_iter(trimmed).count();
    if phone_count >= 3 {
        return true;
    }

    false
}

/// 获取前台应用名称
#[cfg(target_os = "macos")]
fn get_frontmost_app() -> Option<String> {
    use std::process::Command;
    let output = Command::new("osascript")
        .args(["-e", "tell application \"System Events\" to get name of first application process whose frontmost is true"])
        .output()
        .ok()?;
    if output.status.success() {
        let name = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !name.is_empty() {
            return Some(name);
        }
    }
    None
}

#[cfg(target_os = "windows")]
fn get_frontmost_app() -> Option<String> {
    use std::process::Command;
    let output = Command::new("powershell")
        .args(["-NoProfile", "-Command",
            "(Get-Process | Where-Object { $_.MainWindowHandle -eq (Add-Type '[DllImport(\"user32.dll\")] public static extern IntPtr GetForegroundWindow();' -Name W -PassThru)::GetForegroundWindow() }).ProcessName"])
        .output()
        .ok()?;
    if output.status.success() {
        let name = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !name.is_empty() {
            return Some(name);
        }
    }
    None
}

#[cfg(target_os = "linux")]
fn get_frontmost_app() -> Option<String> {
    use std::process::Command;
    let output = Command::new("xdotool")
        .args(["getactivewindow", "getwindowpid"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let pid = String::from_utf8_lossy(&output.stdout).trim().to_string();
    std::fs::read_to_string(format!("/proc/{}/comm", pid))
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

#[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
fn get_frontmost_app() -> Option<String> {
    None
}
