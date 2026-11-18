// Copyright © 2026 sweet_fish2050
// 本程序依据 GNU GPL v3 协议发布。
// SPDX-License-Identifier: GPL-3.0-or-later

//! 明暗配色与控件样式。
//!
//! 颜色按 `Palette` 集中定义，控件的 style 闭包根据 `theme.mode()`
//! 选择对应配色，避免在界面代码里散落魔法数字。

use iced::border::Border;
use iced::theme::{Base as _, Mode};
use iced::widget::{button, container, progress_bar, text_input};
use iced::{Background, Color, Shadow, Theme};

pub struct Palette {
    pub bg: Color,
    pub card: Color,
    pub fg: Color,
    pub muted: Color,
    pub border: Color,
    pub primary: Color,
    pub primary_hover: Color,
    pub digit: Color,
    pub symbol: Color,
    pub success: Color,
    pub success_bg: Color,
    pub warning: Color,
    pub danger: Color,
    pub danger_bg: Color,
}

pub const LIGHT: Palette = Palette {
    bg: Color::from_rgb8(0xF3, 0xF5, 0xF9),
    card: Color::from_rgb8(0xFF, 0xFF, 0xFF),
    fg: Color::from_rgb8(0x1B, 0x23, 0x33),
    muted: Color::from_rgb8(0x6B, 0x74, 0x89),
    border: Color::from_rgb8(0xE2, 0xE6, 0xEF),
    primary: Color::from_rgb8(0x4F, 0x63, 0xE8),
    primary_hover: Color::from_rgb8(0x40, 0x54, 0xD6),
    digit: Color::from_rgb8(0x2F, 0x7C, 0xF6),
    symbol: Color::from_rgb8(0xC2, 0x4B, 0x8B),
    success: Color::from_rgb8(0x14, 0x7A, 0x4D),
    success_bg: Color::from_rgb8(0xE5, 0xF6, 0xEE),
    warning: Color::from_rgb8(0xA8, 0x6A, 0x14),
    danger: Color::from_rgb8(0xC0, 0x39, 0x2B),
    danger_bg: Color::from_rgb8(0xFD, 0xEC, 0xEA),
};

pub const DARK: Palette = Palette {
    bg: Color::from_rgb8(0x11, 0x15, 0x1C),
    card: Color::from_rgb8(0x19, 0x1F, 0x29),
    fg: Color::from_rgb8(0xE8, 0xEC, 0xF4),
    muted: Color::from_rgb8(0x9A, 0xA4, 0xB8),
    border: Color::from_rgb8(0x2A, 0x32, 0x40),
    primary: Color::from_rgb8(0x71, 0x83, 0xFF),
    primary_hover: Color::from_rgb8(0x85, 0x95, 0xFF),
    digit: Color::from_rgb8(0x6C, 0xB6, 0xFF),
    symbol: Color::from_rgb8(0xF0, 0x8B, 0xC0),
    success: Color::from_rgb8(0x5B, 0xD6, 0xA5),
    success_bg: Color::from_rgb8(0x15, 0x33, 0x27),
    warning: Color::from_rgb8(0xF2, 0xC4, 0x6B),
    danger: Color::from_rgb8(0xFF, 0x8A, 0x80),
    danger_bg: Color::from_rgb8(0x3A, 0x1D, 0x1D),
};

pub fn palette(theme: &Theme) -> &'static Palette {
    match theme.mode() {
        Mode::Dark => &DARK,
        _ => &LIGHT,
    }
}

pub fn background(theme: &Theme) -> container::Style {
    let palette = palette(theme);

    container::Style {
        background: Some(Background::Color(palette.bg)),
        text_color: Some(palette.fg),
        ..Default::default()
    }
}

pub fn card(theme: &Theme) -> container::Style {
    let palette = palette(theme);

    container::Style {
        background: Some(Background::Color(palette.card)),
        text_color: Some(palette.fg),
        border: Border {
            color: palette.border,
            width: 1.0,
            radius: 16.0.into(),
        },
        // iced_tiny_skia 的阴影在高 DPI 下会错误覆盖控件背景，故不使用阴影。
        shadow: Shadow::default(),
        ..Default::default()
    }
}

pub fn primary_button(theme: &Theme, status: button::Status) -> button::Style {
    let palette = palette(theme);

    let (background, text_color) = match status {
        button::Status::Active => (palette.primary, Color::WHITE),
        button::Status::Hovered | button::Status::Pressed => (palette.primary_hover, Color::WHITE),
        button::Status::Disabled => (palette.border, palette.muted),
    };

    button::Style {
        background: Some(Background::Color(background)),
        text_color,
        border: Border {
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: 10.0.into(),
        },
        shadow: Shadow::default(),
        snap: true,
    }
}

pub fn ghost_button(theme: &Theme, status: button::Status) -> button::Style {
    let palette = palette(theme);

    let (background, text_color, border_color) = match status {
        button::Status::Active => (palette.card, palette.fg, palette.border),
        button::Status::Hovered => (palette.bg, palette.fg, palette.primary),
        button::Status::Pressed => (palette.border, palette.fg, palette.primary),
        button::Status::Disabled => (palette.card, palette.muted, palette.border),
    };

    button::Style {
        background: Some(Background::Color(background)),
        text_color,
        border: Border {
            color: border_color,
            width: 1.0,
            radius: 10.0.into(),
        },
        shadow: Shadow::default(),
        snap: true,
    }
}

pub fn input(theme: &Theme, status: text_input::Status) -> text_input::Style {
    let palette = palette(theme);

    let border_color = match status {
        text_input::Status::Focused { .. } => palette.primary,
        _ => palette.border,
    };

    text_input::Style {
        background: Background::Color(palette.card),
        border: Border {
            color: border_color,
            width: 1.0,
            radius: 10.0.into(),
        },
        icon: palette.muted,
        placeholder: palette.muted,
        value: palette.fg,
        selection: palette.primary,
    }
}

pub struct Strength {
    pub label: &'static str,
    pub color: Color,
}

/// 强度标签阈值与 all.md 一致：<40 较弱，<60 一般，<80 强，否则很强。
pub fn strength(theme: &Theme, bits: f64) -> Strength {
    let palette = palette(theme);

    if bits < 40.0 {
        Strength {
            label: "较弱",
            color: palette.danger,
        }
    } else if bits < 60.0 {
        Strength {
            label: "一般",
            color: palette.warning,
        }
    } else if bits < 80.0 {
        Strength {
            label: "强",
            color: palette.success,
        }
    } else {
        Strength {
            label: "很强",
            color: palette.success,
        }
    }
}

pub fn strength_bar(theme: &Theme, bits: f64) -> progress_bar::Style {
    let palette = palette(theme);
    let bar = strength(theme, bits).color;

    progress_bar::Style {
        background: Background::Color(palette.border),
        bar: Background::Color(bar),
        border: Border {
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: 4.0.into(),
        },
    }
}
