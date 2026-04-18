//! macOS 原生 NSPanel 全屏叠加方案
//!
//! 策略：将主窗口（main）的 contentView（包含完整 WKWebView + 前端状态）
//! 移植到一个原生 NSPanel 中。NSPanel 具备 FullScreenAuxiliary +
//! NonActivating 特性，能在任意全屏 Space 内直接叠加显示（类似 Alfred）。
//! 原始 Tauri NSWindow 保持存活以维持 IPC 通信。

#![cfg(target_os = "macos")]

use cocoa::appkit::*;
use cocoa::base::{id, nil, NO, YES};
use cocoa::foundation::{NSPoint, NSRect, NSSize};
use objc::declare::ClassDecl;
use objc::runtime::{Class, Object, Sel};
use objc::{class, msg_send, sel, sel_impl};
use std::sync::atomic::{AtomicPtr, Ordering};
use std::sync::Once;

static PANEL_PTR: AtomicPtr<std::ffi::c_void> = AtomicPtr::new(std::ptr::null_mut());
static PANEL_DELEGATE_PTR: AtomicPtr<std::ffi::c_void> = AtomicPtr::new(std::ptr::null_mut());
static PANEL_DELEGATE_CLASS: Once = Once::new();

const NS_TITLED_WINDOW_MASK: u64 = 1 << 0;
const NS_RESIZABLE_WINDOW_MASK: u64 = 1 << 3;
const NS_NON_ACTIVATING_PANEL_MASK: u64 = 1 << 7;
const NS_FULL_SIZE_CONTENT_VIEW_MASK: u64 = 1 << 15;
const NS_BACKING_STORE_BUFFERED: u64 = 2;

fn get_panel() -> Option<id> {
    let ptr = PANEL_PTR.load(Ordering::Acquire);
    if ptr.is_null() {
        None
    } else {
        Some(ptr as id)
    }
}

fn panel_delegate_class() -> *const Class {
    static mut CLASS_PTR: *const Class = std::ptr::null();

    PANEL_DELEGATE_CLASS.call_once(|| unsafe {
        let superclass = class!(NSObject);
        let mut decl = ClassDecl::new("ClipBrainPanelDelegate", superclass)
            .expect("failed to declare ClipBrainPanelDelegate");
        decl.add_method(
            sel!(windowDidResignKey:),
            panel_did_resign_key as extern "C" fn(&Object, Sel, id),
        );
        decl.add_method(
            sel!(windowDidResignMain:),
            panel_did_resign_main as extern "C" fn(&Object, Sel, id),
        );
        CLASS_PTR = decl.register();
    });

    unsafe { CLASS_PTR }
}

extern "C" fn panel_did_resign_key(_this: &Object, _cmd: Sel, notification: id) {
    unsafe {
        let panel: id = msg_send![notification, object];
        if !panel.is_null() {
            let _: () = msg_send![panel, orderOut: nil];
        }
    }
}

extern "C" fn panel_did_resign_main(_this: &Object, _cmd: Sel, notification: id) {
    panel_did_resign_key(_this, _cmd, notification);
}

/// Panel 是否已完成初始化
pub fn is_initialized() -> bool {
    !PANEL_PTR.load(Ordering::Acquire).is_null()
}

/// 在应用启动时调用。将主窗口的 contentView 移植到原生 NSPanel。
pub fn init_native_panel(app: &tauri::AppHandle) -> Result<(), String> {
    use tauri::Manager;

    let main_window = app.get_webview_window("main").ok_or("找不到 main 窗口")?;

    let _ = main_window.with_webview(move |webview: tauri::webview::PlatformWebview| {
        #[allow(deprecated, unexpected_cfgs)]
        unsafe {
            let ns_window = webview.ns_window() as id;

            // 用主窗口当前的 frame 来创建 panel（保持位置和大小一致）
            let frame: NSRect = msg_send![ns_window, frame];

            // 创建 NSPanel
            let style: u64 = NS_TITLED_WINDOW_MASK
                | NS_RESIZABLE_WINDOW_MASK
                | NS_FULL_SIZE_CONTENT_VIEW_MASK
                | NS_NON_ACTIVATING_PANEL_MASK;

            let panel: id = msg_send![class!(NSPanel), alloc];
            let panel: id = msg_send![panel,
                initWithContentRect:frame
                styleMask:style
                backing:NS_BACKING_STORE_BUFFERED
                defer:NO
            ];

            // Collection behavior：可加入所有 Space + 全屏辅助（Alfred 同款）
            let behavior = NSWindowCollectionBehavior::NSWindowCollectionBehaviorCanJoinAllSpaces
                | NSWindowCollectionBehavior::NSWindowCollectionBehaviorFullScreenAuxiliary
                | NSWindowCollectionBehavior::NSWindowCollectionBehaviorTransient
                | NSWindowCollectionBehavior::NSWindowCollectionBehaviorIgnoresCycle;
            panel.setCollectionBehavior_(behavior);
            panel.setLevel_(NSMainMenuWindowLevel as i64 + 1);
            panel.setHidesOnDeactivate_(NO);

            // NSPanel 专有属性
            let _: () = msg_send![panel, setFloatingPanel: YES];
            let _: () = msg_send![panel, setBecomesKeyOnlyIfNeeded: NO];
            let _: () = msg_send![panel, setWorksWhenModal: YES];

            // 视觉配置（与 main 窗口保持一致）
            panel.setTitlebarAppearsTransparent_(YES);
            panel.setTitleVisibility_(NSWindowTitleVisibility::NSWindowTitleHidden);
            panel.setOpaque_(NO);
            let clear: id = msg_send![class!(NSColor), clearColor];
            panel.setBackgroundColor_(clear);
            panel.setHasShadow_(YES);
            let _: () = msg_send![panel, setMovableByWindowBackground: YES];

            // 隐藏红绿灯按钮
            for i in 0u64..3 {
                let btn: id = msg_send![panel, standardWindowButton: i];
                if !btn.is_null() {
                    let _: () = msg_send![btn, setHidden: YES];
                }
            }

            // 设置最小尺寸（与 tauri.conf.json 一致）
            let _: () = msg_send![panel, setMinSize: NSSize::new(720.0, 480.0)];

            // 失去焦点时自动隐藏，但不影响 overlay 模式的正常唤起
            let delegate_class = panel_delegate_class();
            let delegate: id = msg_send![delegate_class, new];
            let _: () = msg_send![panel, setDelegate: delegate];
            PANEL_DELEGATE_PTR.store(delegate as *mut std::ffi::c_void, Ordering::Release);

            // ★ 核心：将 contentView 从 Tauri NSWindow 移植到 NSPanel
            let content_view: id = ns_window.contentView();
            let _: () = msg_send![content_view, retain];
            let _: () = msg_send![ns_window, setContentView: nil];
            panel.setContentView_(content_view);
            let _: () = msg_send![content_view, release];

            // 圆角裁剪
            content_view.setWantsLayer(YES);
            let layer: id = msg_send![content_view, layer];
            if !layer.is_null() {
                let _: () = msg_send![layer, setCornerRadius: 16.0_f64];
                let _: () = msg_send![layer, setMasksToBounds: YES];
            }

            // 隐藏原始 Tauri NSWindow（保持存活以维持 IPC）
            let _: () = msg_send![ns_window, orderOut: nil];

            // 存储 panel 指针
            PANEL_PTR.store(panel as *mut std::ffi::c_void, Ordering::Release);
            log::info!("原生 NSPanel 创建成功，主窗口 contentView 已移植");
        }
    });

    Ok(())
}

/// 以 overlay 模式显示 panel（不激活应用，适用于快捷键唤起）
pub fn show_panel() -> Result<(), String> {
    let panel = get_panel().ok_or("原生 panel 未初始化")?;
    unsafe {
        let _: () = msg_send![panel, orderFrontRegardless];
        let _: () = msg_send![panel, makeKeyWindow];
    }
    Ok(())
}

/// 以 focused 模式显示 panel（激活应用，适用于托盘/设置）
pub fn show_panel_focused() -> Result<(), String> {
    let panel = get_panel().ok_or("原生 panel 未初始化")?;
    unsafe {
        let _: () = msg_send![panel, makeKeyAndOrderFront: nil];
        let app: id = msg_send![class!(NSApplication), sharedApplication];
        let _: () = msg_send![app, activateIgnoringOtherApps: YES];
    }
    Ok(())
}

/// 隐藏 panel
pub fn hide_panel() -> Result<(), String> {
    let panel = get_panel().ok_or("原生 panel 未初始化")?;
    unsafe {
        let _: () = msg_send![panel, orderOut: nil];
    }
    Ok(())
}

/// 检查 panel 是否可见
pub fn is_panel_visible() -> bool {
    get_panel()
        .map(|panel| unsafe {
            let visible: bool = msg_send![panel, isVisible];
            visible
        })
        .unwrap_or(false)
}

/// 对 NSPanel 发起拖动（用当前事件）
pub fn start_drag() -> Result<(), String> {
    let panel = get_panel().ok_or("原生 panel 未初始化")?;
    unsafe {
        let app: id = msg_send![class!(NSApplication), sharedApplication];
        let event: id = msg_send![app, currentEvent];
        if !event.is_null() {
            let _: () = msg_send![panel, performWindowDragWithEvent: event];
        }
    }
    Ok(())
}
