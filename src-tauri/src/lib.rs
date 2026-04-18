use std::{
    panic::{catch_unwind, AssertUnwindSafe},
    sync::{Mutex, OnceLock},
};
use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    Emitter, Manager, RunEvent, WindowEvent,
};
use tauri_plugin_autostart::{MacosLauncher, ManagerExt as AutostartManagerExt};
use tauri_plugin_global_shortcut::GlobalShortcutExt;

mod actions;
mod classifier;
mod clipboard;
mod commands;
mod config;
mod errors;
#[cfg(target_os = "macos")]
mod macos_panel;
mod model;
mod storage;
mod utils;

#[derive(Clone, Copy)]
pub(crate) enum MainWindowShowMode {
    Overlay,
    Focused,
}

#[cfg(target_os = "macos")]
fn previous_overlay_app_pid() -> &'static Mutex<Option<i32>> {
    static PREVIOUS_APP_PID: OnceLock<Mutex<Option<i32>>> = OnceLock::new();
    PREVIOUS_APP_PID.get_or_init(|| Mutex::new(None))
}

#[cfg(target_os = "macos")]
fn remember_overlay_target_app() {
    #[allow(deprecated, unexpected_cfgs)]
    unsafe {
        use cocoa::base::{id, nil};
        use objc::{class, msg_send, sel, sel_impl};

        let workspace: id = msg_send![class!(NSWorkspace), sharedWorkspace];
        if workspace == nil {
            return;
        }
        let frontmost_app: id = msg_send![workspace, frontmostApplication];
        if frontmost_app == nil {
            return;
        }
        let pid: i32 = msg_send![frontmost_app, processIdentifier];
        let current_pid = std::process::id() as i32;
        if pid != current_pid {
            if let Ok(mut slot) = previous_overlay_app_pid().lock() {
                *slot = Some(pid);
            }
        }
    }
}

#[cfg(target_os = "macos")]
pub(crate) fn reactivate_previous_overlay_app() -> Result<bool, String> {
    #[allow(deprecated, unexpected_cfgs)]
    unsafe {
        use cocoa::appkit::{NSApplicationActivateIgnoringOtherApps, NSRunningApplication};
        use cocoa::base::{id, nil, NO};
        use objc::{class, msg_send, sel, sel_impl};

        let pid = match previous_overlay_app_pid()
            .lock()
            .map_err(|_| "读取目标应用状态失败".to_string())?
            .as_ref()
            .copied()
        {
            Some(pid) => pid,
            None => return Ok(false),
        };

        let running_app: id =
            msg_send![class!(NSRunningApplication), runningApplicationWithProcessIdentifier: pid];
        if running_app == nil {
            return Err("目标应用已不可用，无法恢复焦点".to_string());
        }

        let activated = running_app.activateWithOptions_(NSApplicationActivateIgnoringOtherApps);
        if activated == NO {
            return Err("恢复刚才的应用失败".to_string());
        }

        if let Ok(mut slot) = previous_overlay_app_pid().lock() {
            if slot.as_ref().copied() == Some(pid) {
                *slot = None;
            }
        }

        Ok(true)
    }
}

pub(crate) fn show_main_window(app_handle: &tauri::AppHandle, mode: MainWindowShowMode) {
    #[cfg(target_os = "macos")]
    if matches!(mode, MainWindowShowMode::Overlay) {
        remember_overlay_target_app();
    }

    // macOS: 如果 NSPanel 已就绪，通过 panel 显示
    #[cfg(target_os = "macos")]
    if macos_panel::is_initialized() {
        let res = match mode {
            MainWindowShowMode::Overlay => macos_panel::show_panel(),
            MainWindowShowMode::Focused => macos_panel::show_panel_focused(),
        };
        if let Err(e) = res {
            log::error!("show_panel 失败: {}", e);
        }
        // IPC 事件仍通过 Tauri webview window 发送（WKWebView 不变）
        if let Some(window) = app_handle.get_webview_window("main") {
            let _ = window.emit(
                "main-window-shown",
                if matches!(mode, MainWindowShowMode::Overlay) {
                    "overlay"
                } else {
                    "focused"
                },
            );
        }
        return;
    }

    // 回退路径（panel 未就绪或非 macOS）
    if let Some(window) = app_handle.get_webview_window("main") {
        let _ = window.show();
        #[cfg(target_os = "macos")]
        configure_macos_fallback_window(&window, mode);
        #[cfg(target_os = "macos")]
        if matches!(mode, MainWindowShowMode::Focused) {
            let _ = window.set_focus();
        }
        #[cfg(not(target_os = "macos"))]
        let _ = window.set_focus();
        let _ = window.emit(
            "main-window-shown",
            if matches!(mode, MainWindowShowMode::Overlay) {
                "overlay"
            } else {
                "focused"
            },
        );
    }
}

#[cfg(target_os = "macos")]
fn configure_macos_fallback_window(window: &tauri::WebviewWindow, mode: MainWindowShowMode) {
    let _ = window.with_webview(move |webview| {
        #[allow(deprecated, unexpected_cfgs)]
        unsafe {
            use cocoa::appkit::{NSMainMenuWindowLevel, NSWindow, NSWindowCollectionBehavior};
            use cocoa::base::{id, nil, NO, YES};
            use cocoa::foundation::NSInteger;
            use objc::{msg_send, sel, sel_impl};

            let ns_window = webview.ns_window() as id;
            let behavior = NSWindowCollectionBehavior::NSWindowCollectionBehaviorCanJoinAllSpaces
                | NSWindowCollectionBehavior::NSWindowCollectionBehaviorFullScreenAuxiliary
                | NSWindowCollectionBehavior::NSWindowCollectionBehaviorTransient
                | NSWindowCollectionBehavior::NSWindowCollectionBehaviorIgnoresCycle;
            ns_window.setCollectionBehavior_(ns_window.collectionBehavior() | behavior);
            ns_window.setLevel_(NSMainMenuWindowLevel as NSInteger);
            ns_window.setHidesOnDeactivate_(NO);
            let _: () = msg_send![ns_window, setMovableByWindowBackground: YES];
            match mode {
                MainWindowShowMode::Overlay => ns_window.orderFrontRegardless(),
                MainWindowShowMode::Focused => {
                    let _: () = msg_send![ns_window, makeKeyAndOrderFront: nil];
                }
            }
        }
    });
}

fn main_window_is_visible(app_handle: &tauri::AppHandle) -> bool {
    #[cfg(target_os = "macos")]
    if macos_panel::is_initialized() {
        return macos_panel::is_panel_visible();
    }

    app_handle
        .get_webview_window("main")
        .and_then(|window| window.is_visible().ok())
        .unwrap_or(false)
}

fn hide_main_window(app_handle: &tauri::AppHandle) {
    #[cfg(target_os = "macos")]
    if macos_panel::is_initialized() {
        let _ = macos_panel::hide_panel();
        return;
    }

    if let Some(window) = app_handle.get_webview_window("main") {
        let _ = window.hide();
    }
}

fn run_hotkey_action(action: &str, callback: impl FnOnce()) {
    if let Err(payload) = catch_unwind(AssertUnwindSafe(callback)) {
        let reason = if let Some(message) = payload.downcast_ref::<&str>() {
            *message
        } else if let Some(message) = payload.downcast_ref::<String>() {
            message.as_str()
        } else {
            "unknown panic payload"
        };
        log::error!("hotkey action '{}' panicked: {}", action, reason);
    }
}

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! Welcome to ClipBrain.", name)
}

/// 隐藏面板（前端调用，自动路由到 NSPanel 或 Tauri 窗口）
#[tauri::command]
fn hide_overlay_panel(app_handle: tauri::AppHandle) {
    hide_main_window(&app_handle);
}

/// 显示面板（前端调用，用于错误恢复等场景）
#[tauri::command]
fn show_overlay_panel(app_handle: tauri::AppHandle) {
    show_main_window(&app_handle, MainWindowShowMode::Focused);
}

/// 拖动面板窗口（macOS NSPanel 专用）
#[tauri::command]
fn start_panel_drag() {
    #[cfg(target_os = "macos")]
    if macos_panel::is_initialized() {
        let _ = macos_panel::start_drag();
    }
}

/// 更新全局快捷键（先注销旧的，再注册新的）
#[tauri::command]
fn update_shortcut(
    app_handle: tauri::AppHandle,
    old_shortcut: String,
    new_shortcut: String,
) -> Result<(), String> {
    use tauri_plugin_global_shortcut::ShortcutState;

    let gs = app_handle.global_shortcut();

    // 注销旧快捷键
    if let Ok(old) = old_shortcut.parse::<tauri_plugin_global_shortcut::Shortcut>() {
        let _ = gs.unregister(old);
    }

    // 注册新快捷键
    let new_sc: tauri_plugin_global_shortcut::Shortcut = new_shortcut
        .parse()
        .map_err(|e| format!("快捷键格式无效: {}", e))?;

    gs.on_shortcut(new_sc, move |handle, _shortcut, event| {
        if event.state == ShortcutState::Pressed {
            run_hotkey_action("update_shortcut_toggle_panel", || {
                if main_window_is_visible(handle) {
                    hide_main_window(handle);
                } else {
                    show_main_window(handle, MainWindowShowMode::Overlay);
                }
            });
        }
    })
    .map_err(|e| format!("注册快捷键失败: {}", e))?;

    // 持久化到配置
    config::manager::update_with(|cfg| {
        cfg.hotkey.open_panel = new_shortcut;
    })?;

    Ok(())
}

pub fn run() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            None::<Vec<&str>>,
        ));

    builder
        .setup(|app| {
            // macOS: 设为 Accessory 激活策略 —— 不在 Dock 显示图标，仅保留菜单栏托盘
            #[cfg(target_os = "macos")]
            {
                #[allow(deprecated, unexpected_cfgs)]
                unsafe {
                    use cocoa::appkit::{NSApp, NSApplication, NSApplicationActivationPolicy};
                    let ns_app = NSApp();
                    ns_app.setActivationPolicy_(
                        NSApplicationActivationPolicy::NSApplicationActivationPolicyAccessory,
                    );
                }
            }

            let config = config::manager::get();
            let autostart = app.autolaunch();
            let current_enabled = autostart.is_enabled().unwrap_or(false);
            if config.general.auto_start && !current_enabled {
                autostart
                    .enable()
                    .map_err(|e| format!("启用开机自启动失败: {}", e))?;
            } else if !config.general.auto_start && current_enabled {
                autostart
                    .disable()
                    .map_err(|e| format!("关闭开机自启动失败: {}", e))?;
            }

            // 创建托盘菜单
            let show_i = MenuItem::with_id(app, "show", "显示面板", true, None::<&str>)?;
            let settings_i = MenuItem::with_id(app, "settings", "设置", true, None::<&str>)?;
            let quit_i = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show_i, &settings_i, &quit_i])?;

            // 构建托盘图标（使用单色模板图标，适配 macOS 菜单栏）
            let tray_icon_bytes = include_bytes!("../icons/tray-icon@2x.png");
            let tray_img = image::load_from_memory(tray_icon_bytes)
                .expect("failed to load tray icon")
                .into_rgba8();
            let (w, h) = tray_img.dimensions();
            let icon = tauri::image::Image::new_owned(tray_img.into_raw(), w, h);
            TrayIconBuilder::new()
                .icon(icon)
                .icon_as_template(true)
                .menu(&menu)
                .tooltip("ClipBrain - 智能剪贴板助手")
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "show" => {
                        show_main_window(app, MainWindowShowMode::Focused);
                    }
                    "settings" => {
                        if let Some(window) = app.get_webview_window("main") {
                            show_main_window(app, MainWindowShowMode::Focused);
                            let _ = window.emit("navigate", "settings");
                        }
                    }
                    "quit" => {
                        app.exit(0);
                    }
                    _ => {}
                })
                .build(app)?;

            // 注册全局快捷键（从配置读取）
            #[cfg(desktop)]
            {
                use tauri_plugin_global_shortcut::ShortcutState;

                let hotkey = config::manager::get().hotkey.open_panel;
                log::info!("注册全局快捷键: {}", hotkey);

                app.global_shortcut().on_shortcut(
                    hotkey.as_str(),
                    move |app_handle: &tauri::AppHandle, _shortcut, event| {
                        if event.state == ShortcutState::Pressed {
                            run_hotkey_action("global_shortcut_toggle_panel", || {
                                if main_window_is_visible(app_handle) {
                                    hide_main_window(app_handle);
                                } else {
                                    show_main_window(app_handle, MainWindowShowMode::Overlay);
                                }
                            });
                        }
                    },
                )?;
            }

            // “更多操作”已隐藏，历史快捷操作绑定保留在配置中但不再注册执行。
            {
                let quick_actions = config::manager::get().hotkey.quick_actions;
                if !quick_actions.is_empty() {
                    log::info!("已跳过 {} 个历史快捷操作绑定", quick_actions.len());
                }
            }

            // 初始化数据库
            let _db = storage::database::get_db();
            log::info!("数据库已初始化");

            // macOS: 原生圆角窗口（模拟系统应用效果）
            #[cfg(target_os = "macos")]
            {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.with_webview(move |webview| {
                        #[allow(deprecated, unexpected_cfgs)]
                        unsafe {
                            use cocoa::appkit::{
                                NSView, NSWindow, NSWindowStyleMask, NSWindowTitleVisibility,
                            };
                            use cocoa::base::{id, NO, YES};
                            use objc::runtime::Class;
                            use objc::{msg_send, sel, sel_impl};

                            let ns_window = webview.ns_window() as id;

                            // 添加 Titled + FullSizeContentView 以获得 macOS 原生圆角
                            let mut style = ns_window.styleMask();
                            style |= NSWindowStyleMask::NSTitledWindowMask;
                            style |= NSWindowStyleMask::NSFullSizeContentViewWindowMask;
                            ns_window.setStyleMask_(style);

                            // 隐藏标题栏（保持无边框外观）
                            ns_window.setTitlebarAppearsTransparent_(YES);
                            ns_window
                                .setTitleVisibility_(NSWindowTitleVisibility::NSWindowTitleHidden);

                            // 窗口背景透明，消除白色方角
                            ns_window.setOpaque_(NO);
                            let cls = Class::get("NSColor").unwrap();
                            let clear: id = msg_send![cls, clearColor];
                            ns_window.setBackgroundColor_(clear);

                            // 原生阴影
                            ns_window.setHasShadow_(YES);

                            // 隐藏红绿灯按钮
                            for i in 0u64..3 {
                                let btn: id = msg_send![ns_window, standardWindowButton: i];
                                if !btn.is_null() {
                                    let _: () = msg_send![btn, setHidden: YES];
                                }
                            }

                            // Layer 圆角裁剪
                            let content_view: id = ns_window.contentView();
                            content_view.setWantsLayer(YES);
                            let layer: id = msg_send![content_view, layer];
                            if !layer.is_null() {
                                let _: () = msg_send![layer, setCornerRadius: 16.0_f64];
                                let _: () = msg_send![layer, setMasksToBounds: YES];
                            }
                        }
                    });
                }
            }

            // 从 config.toml 恢复已保存的模型后端
            commands::model_cmds::restore_backends_from_config();

            // macOS: 初始化原生 NSPanel（全屏叠加面板）
            #[cfg(target_os = "macos")]
            {
                if let Err(e) = macos_panel::init_native_panel(app.handle()) {
                    log::error!("初始化原生 NSPanel 失败: {}", e);
                }
            }

            // 首次安装时直接展示窗口，避免 panel 初始化后保持隐藏，必须手动点 Dock 图标才出现。
            if commands::config_cmds::is_first_launch() {
                show_main_window(app.handle(), MainWindowShowMode::Focused);
            }

            // 启动剪贴板后台监听（轮询间隔 500ms，去重节流 300ms）
            let monitor = clipboard::monitor::ClipboardMonitor::new(500, 300);
            monitor.start(app.handle().clone());

            log::info!("ClipBrain started successfully");
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            greet,
            update_shortcut,
            hide_overlay_panel,
            show_overlay_panel,
            start_panel_drag,
            commands::clipboard_cmds::get_clipboard_content,
            commands::clipboard_cmds::write_to_clipboard,
            commands::clipboard_cmds::write_image_to_clipboard,
            commands::clipboard_cmds::write_files_to_clipboard,
            commands::clipboard_cmds::paste_clipboard,
            commands::clipboard_cmds::restore_previous_app_and_paste,
            commands::action_cmds::list_actions,
            commands::action_cmds::execute_action,
            commands::action_cmds::execute_action_stream,
            commands::config_cmds::get_config,
            commands::config_cmds::save_config,
            commands::config_cmds::reload_config,
            commands::config_cmds::is_first_launch,
            commands::config_cmds::complete_onboarding,
            commands::model_cmds::save_model_config,
            commands::model_cmds::test_model_connection,
            commands::model_cmds::list_model_backends,
            commands::model_cmds::has_model_backend,
            commands::model_cmds::setup_and_test_model,
            commands::model_cmds::list_model_configs,
            commands::model_cmds::delete_model_config,
            commands::model_cmds::set_active_model,
            commands::history_cmds::list_history,
            commands::history_cmds::search_history,
            commands::history_cmds::delete_history,
            commands::history_cmds::toggle_pin,
            commands::history_cmds::clear_history,
            commands::history_cmds::clear_history_with_retention,
            commands::history_cmds::history_count,
            commands::history_cmds::count_history_over_size,
            commands::history_cmds::clear_history_over_size,
            commands::history_cmds::search_history_advanced,
            commands::plugin_cmds::list_plugins,
            commands::plugin_cmds::get_plugins_dir,
            commands::plugin_cmds::reload_plugins,
            commands::plugin_cmds::fetch_store_index,
            commands::plugin_cmds::install_store_plugin,
            commands::plugin_cmds::uninstall_plugin,
            commands::plugin_cmds::installed_plugin_ids,
            commands::model_mgmt_cmds::list_recommended_models,
            commands::model_mgmt_cmds::list_downloaded_models,
            commands::model_mgmt_cmds::delete_model,
            commands::model_mgmt_cmds::download_model,
            commands::model_mgmt_cmds::get_models_dir,
            commands::tag_cmds::add_tag,
            commands::tag_cmds::remove_tag,
            commands::tag_cmds::get_tags,
            commands::tag_cmds::list_all_tags,
            commands::tag_cmds::search_by_tag,
            commands::history_cmds::get_app_icon,
            commands::history_cmds::get_file_preview,
            commands::action_cmds::execute_quick_action,
            commands::action_cmds::execute_custom_stream,
            commands::stats_cmds::get_stats,
            commands::clipboard_cmds::read_image_base64,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app_handle, event| match event {
            RunEvent::WindowEvent {
                ref label,
                event: WindowEvent::CloseRequested { api, .. },
                ..
            } => {
                api.prevent_close();
                if label == "main" {
                    hide_main_window(app_handle);
                }
            }
            RunEvent::Reopen { .. } => {
                show_main_window(app_handle, MainWindowShowMode::Focused);
            }
            _ => {}
        });
}
