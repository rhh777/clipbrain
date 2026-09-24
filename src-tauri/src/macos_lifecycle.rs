//! macOS background-process lifecycle guards.
//!
//! ClipBrain is an LSUIElement/accessory app that often lives with its window hidden.
//! Tell AppKit this process has ongoing background work so macOS should not
//! automatically terminate it while the clipboard monitor is expected to stay alive.

#![cfg(target_os = "macos")]
#![allow(deprecated, unexpected_cfgs)]

use cocoa::base::{id, nil};
use cocoa::foundation::NSString;
use objc::{class, msg_send, sel, sel_impl};
use std::sync::atomic::{AtomicPtr, Ordering};

static ACTIVITY_TOKEN: AtomicPtr<std::ffi::c_void> = AtomicPtr::new(std::ptr::null_mut());

const NS_ACTIVITY_SUDDEN_TERMINATION_DISABLED: u64 = 1_u64 << 14;
const NS_ACTIVITY_AUTOMATIC_TERMINATION_DISABLED: u64 = 1_u64 << 15;
const NS_ACTIVITY_BACKGROUND: u64 = 0x0000_00FF;

pub fn prevent_background_termination() {
    if !ACTIVITY_TOKEN.load(Ordering::Acquire).is_null() {
        return;
    }

    #[allow(deprecated, unexpected_cfgs)]
    unsafe {
        let process_info: id = msg_send![class!(NSProcessInfo), processInfo];
        if process_info == nil {
            log::warn!("无法获取 NSProcessInfo，后台保活设置未启用");
            return;
        }

        let reason = NSString::alloc(nil).init_str("ClipBrain clipboard monitor");
        let options = NS_ACTIVITY_BACKGROUND
            | NS_ACTIVITY_SUDDEN_TERMINATION_DISABLED
            | NS_ACTIVITY_AUTOMATIC_TERMINATION_DISABLED;

        let token: id = msg_send![process_info, beginActivityWithOptions: options reason: reason];
        if token == nil {
            log::warn!("beginActivityWithOptions 返回空，后台保活设置未启用");
            return;
        }

        let token: id = msg_send![token, retain];
        ACTIVITY_TOKEN.store(token as *mut std::ffi::c_void, Ordering::Release);
        log::info!("已禁用 macOS 对 ClipBrain 的后台自动终止");
    }
}
