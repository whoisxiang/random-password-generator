// Copyright © 2026 sweet_fish2050
// 本程序依据 GNU GPL v3 协议发布。
// SPDX-License-Identifier: GPL-3.0-or-later

//! 带机密标记的剪贴板写入。
//!
//! Windows 上同时设置三个剪贴板格式，避免密码进入剪贴板历史、
//! 云同步或被剪贴板监视工具记录：
//!
//! - `ExcludeClipboardContentFromMonitorProcessing`
//! - `CanIncludeInClipboardHistory` = 0
//! - `CanUploadToCloudClipboard` = 0
//!
//! 不提供明文回退路径：复制失败只报告错误，宁可失败也不把密码写入
//! 未受保护的剪贴板。

use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClipboardError {
    /// 仅非 Windows 平台构造；Windows 构建中保留以便统一错误展示。
    #[allow(dead_code)]
    Unsupported,
    Win32(String),
}

impl fmt::Display for ClipboardError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unsupported => write!(formatter, "当前系统不支持机密剪贴板写入"),
            Self::Win32(reason) => write!(formatter, "{reason}"),
        }
    }
}

impl std::error::Error for ClipboardError {}

/// 将文本写入剪贴板，并标记为不进入历史/云同步/监视。
pub fn copy_concealed(text: &str) -> Result<(), ClipboardError> {
    #[cfg(windows)]
    {
        windows_impl::copy(text)
    }

    #[cfg(not(windows))]
    {
        let _ = text;
        Err(ClipboardError::Unsupported)
    }
}

#[cfg(windows)]
mod windows_impl {
    use super::ClipboardError;

    use std::ptr;

    use windows_sys::Win32::Foundation::{GlobalFree, HGLOBAL};
    use windows_sys::Win32::System::DataExchange::{
        CloseClipboard, EmptyClipboard, OpenClipboard, RegisterClipboardFormatW, SetClipboardData,
    };
    use windows_sys::Win32::System::Memory::{
        GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE,
    };
    use zeroize::Zeroize;

    /// `CF_UNICODETEXT`；`SetClipboardData` 的格式参数是 `u32`，
    /// 本地定义常量可避免引入额外的 windows-sys feature。
    const CF_UNICODETEXT: u32 = 13;

    const EXCLUSION_FORMATS: [&str; 3] = [
        "ExcludeClipboardContentFromMonitorProcessing",
        "CanIncludeInClipboardHistory",
        "CanUploadToCloudClipboard",
    ];

    pub fn copy(text: &str) -> Result<(), ClipboardError> {
        let _session = ClipboardSession::open()?;

        // SAFETY: 剪贴板已由 ClipboardSession 打开。
        if unsafe { EmptyClipboard() } == 0 {
            return Err(last_error("清空剪贴板失败"));
        }

        let outcome = (|| -> Result<(), ClipboardError> {
            // 先写排除格式，失败时密码根本不会进入剪贴板。
            for name in EXCLUSION_FORMATS {
                let format = register_format(name)?;
                set_format(format, &[0u8; 4])?;
            }

            let mut wide = to_utf16(text);
            let mut bytes = Vec::with_capacity(wide.len() * 2);

            for unit in &wide {
                bytes.extend_from_slice(&unit.to_le_bytes());
            }

            let written = set_format(CF_UNICODETEXT, &bytes);

            wide.zeroize();
            bytes.zeroize();

            written
        })();

        // 任何一步失败都清空剪贴板，避免留下不完整或未受保护的内容。
        if outcome.is_err() {
            // SAFETY: 剪贴板仍处于打开状态。
            unsafe {
                EmptyClipboard();
            }
        }

        outcome
    }

    struct ClipboardSession;

    impl ClipboardSession {
        fn open() -> Result<Self, ClipboardError> {
            for _ in 0..5 {
                // SAFETY: 传入 NULL 表示把剪贴板关联到当前任务。
                if unsafe { OpenClipboard(ptr::null_mut()) } != 0 {
                    return Ok(Self);
                }

                std::thread::sleep(std::time::Duration::from_millis(10));
            }

            Err(last_error("打开剪贴板失败"))
        }
    }

    impl Drop for ClipboardSession {
        fn drop(&mut self) {
            // SAFETY: 只在 Copy 期间打开，Drop 对应一次关闭。
            unsafe {
                CloseClipboard();
            }
        }
    }

    fn set_format(format: u32, bytes: &[u8]) -> Result<(), ClipboardError> {
        // SAFETY: GlobalAlloc 返回的句柄由我们独占，失败时负责释放。
        let handle: HGLOBAL = unsafe { GlobalAlloc(GMEM_MOVEABLE, bytes.len().max(1)) };

        if handle.is_null() {
            return Err(last_error("分配剪贴板内存失败"));
        }

        // SAFETY: 句柄有效且已锁定；SetClipboardData 成功后所有权转移给系统。
        unsafe {
            let destination = GlobalLock(handle);

            if destination.is_null() {
                let _ = GlobalFree(handle);
                return Err(last_error("锁定剪贴板内存失败"));
            }

            ptr::copy_nonoverlapping(bytes.as_ptr(), destination.cast::<u8>(), bytes.len());
            GlobalUnlock(handle);

            if SetClipboardData(format, handle).is_null() {
                let _ = GlobalFree(handle);
                return Err(last_error("写入剪贴板失败"));
            }
        }

        Ok(())
    }

    fn register_format(name: &str) -> Result<u32, ClipboardError> {
        let wide = to_utf16(name);

        // SAFETY: wide 以 NUL 结尾，生命周期覆盖调用。
        let format = unsafe { RegisterClipboardFormatW(wide.as_ptr()) };

        if format == 0 {
            Err(last_error("注册剪贴板格式失败"))
        } else {
            Ok(format)
        }
    }

    fn to_utf16(text: &str) -> Vec<u16> {
        text.encode_utf16().chain(std::iter::once(0)).collect()
    }

    fn last_error(context: &str) -> ClipboardError {
        ClipboardError::Win32(format!("{context}: {}", std::io::Error::last_os_error()))
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn utf16_encoding_is_nul_terminated() {
            assert_eq!(to_utf16("ab"), vec![0x0061, 0x0062, 0x0000]);
            assert_eq!(to_utf16("密")[0], 0x5BC6);
            assert_eq!(*to_utf16("密").last().unwrap(), 0x0000);
        }

        #[test]
        fn exclusion_formats_cover_history_and_cloud() {
            assert!(EXCLUSION_FORMATS.contains(&"CanIncludeInClipboardHistory"));
            assert!(EXCLUSION_FORMATS.contains(&"CanUploadToCloudClipboard"));
            assert!(EXCLUSION_FORMATS.contains(&"ExcludeClipboardContentFromMonitorProcessing"));
        }

        #[test]
        fn empty_text_still_produces_terminator() {
            assert_eq!(to_utf16(""), vec![0x0000]);
        }

        fn clipboard_has_format(name: &str) -> bool {
            use windows_sys::Win32::System::DataExchange::EnumClipboardFormats;

            let target = {
                let wide = to_utf16(name);
                // SAFETY: wide 以 NUL 结尾，生命周期覆盖调用。
                unsafe { RegisterClipboardFormatW(wide.as_ptr()) }
            };

            if target == 0 {
                return false;
            }

            // SAFETY: 打开后按格式链遍历，最后关闭剪贴板。
            unsafe {
                if OpenClipboard(ptr::null_mut()) == 0 {
                    return false;
                }

                let mut format = EnumClipboardFormats(0);

                while format != 0 {
                    if format == target {
                        CloseClipboard();
                        return true;
                    }

                    format = EnumClipboardFormats(format);
                }

                CloseClipboard();
            }

            false
        }

        /// 手工验证用：`cargo test -- --ignored`。
        /// 会覆盖系统剪贴板内容。
        #[test]
        #[ignore = "会修改系统剪贴板"]
        fn live_copy_sets_concealment_formats() {
            copy("random-password-generator-clipboard-test").expect("clipboard write should succeed");

            for name in EXCLUSION_FORMATS {
                assert!(clipboard_has_format(name), "missing format {name}");
            }
        }
    }
}
