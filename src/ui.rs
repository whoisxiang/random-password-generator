// Copyright © 2026 sweet_fish2050
// 本程序依据 GNU GPL v3 协议发布。
// SPDX-License-Identifier: GPL-3.0-or-later

//! 界面视图。

use iced::widget::text::{Rich, Span, Wrapping};
use iced::widget::{
    button, checkbox, column, container, progress_bar, row, scrollable, slider, span, text,
    text_input, toggler, Stack,
};
use iced::{Alignment, Background, Border, Element, Font, Length, Theme};

use crate::app::{App, Message, Toast, ToastKind};
use crate::style::{self, Palette};

pub fn view(app: &App) -> Element<'_, Message> {
    let palette = style::palette(&app.theme());

    let header = column![
        text("随机密码生成器")
            .size(28)
            .font(Font {
                weight: iced::font::Weight::Bold,
                ..Font::DEFAULT
            })
            .color(palette.fg),
        text("完全离线 · 只在本机内存中生成，不联网、不写磁盘")
            .size(13)
            .color(palette.muted)
            .width(Length::Fill)
            .wrapping(Wrapping::WordOrGlyph),
    ]
    .spacing(6);

    let password_card = container(
        column![
            text("随机密码").size(12).color(palette.muted),
            password_line(app, palette),
            action_row(app),
        ]
        .spacing(16),
    )
    .style(style::card)
    .padding(22)
    .width(Length::Fill);

    let strength = style::strength(&app.theme(), app.entropy);
    let strength_row = row![
        progress_bar(0.0..=128.0, app.entropy.clamp(0.0, 128.0) as f32)
            .style(move |theme| style::strength_bar(theme, app.entropy))
            .girth(Length::Fixed(8.0))
            .length(Length::Fill),
        text(format!(
            "约 {:.1} bits · 强度：{}",
            app.entropy, strength.label
        ))
        .size(12)
        .color(strength.color),
    ]
    .spacing(14)
    .align_y(Alignment::Center);

    let options_card = container(options(app, palette))
        .style(style::card)
        .padding(22)
        .width(Length::Fill);

    let footer = text(
        "提示：密码只保存在内存中。复制后剪贴板内容会保留到被替换；\
         本程序已把密码排除在 Windows 剪贴板历史与云同步之外。",
    )
    .size(11)
    .color(palette.muted)
    .width(Length::Fill)
    .wrapping(Wrapping::WordOrGlyph);

    // 版权信息固定在窗体底部，并水平居中。
    let copyright = container(
        text("Copyright © 2026 sweet_fish2050 · 本程序依据 GNU GPL v3 协议发布")
            .size(11)
            .color(palette.muted)
    )
    .width(Length::Fill)
    .padding([12.0, 0.0])
    .align_x(iced::alignment::Horizontal::Center);

    let content =
        container(column![header, password_card, strength_row, options_card, footer].spacing(20))
            .max_width(820)
            .padding(28)
            .center_x(Length::Fill);

    let page: Element<'_, Message> = container(
        column![
            container(scrollable(content).height(Length::Fill).width(Length::Fill))
                .width(Length::Fill)
                .height(Length::Fill),
            copyright,
        ]
    )
    .style(style::background)
    .width(Length::Fill)
    .height(Length::Fill)
    .into();

    match &app.toast {
        Some(toast) => Stack::with_children(vec![page, toast_layer(toast, palette)])
            .width(Length::Fill)
            .height(Length::Fill)
            .into(),
        None => page,
    }
}

fn password_line<'a>(app: &'a App, palette: &'static Palette) -> Element<'a, Message> {
    if app.password.is_empty() {
        return text("生成失败，请重试")
            .size(22)
            .font(Font::MONOSPACE)
            .color(palette.danger)
            .into();
    }

    if app.hidden {
        // 最多显示 20 个圆点，不暴露真实长度。
        let dots = "•".repeat(app.password.chars().count().min(20));

        return text(dots)
            .size(24)
            .font(Font::MONOSPACE)
            .color(palette.fg)
            .into();
    }

    let spans: Vec<Span<'a, (), Font>> = app
        .password
        .chars()
        .map(|character| {
            let color = if character.is_ascii_digit() {
                palette.digit
            } else if character.is_ascii_alphabetic() {
                palette.fg
            } else {
                palette.symbol
            };

            span::<(), Font>(character.to_string())
                .color(color)
                .font(Font::MONOSPACE)
        })
        .collect();

    let rich: Rich<'a, (), Message, Theme, iced::Renderer> = Rich::with_spans(spans);

    rich.size(24)
        .width(Length::Fill)
        .wrapping(Wrapping::Glyph)
        .into()
}

fn action_row(app: &App) -> Element<'_, Message> {
    row![
        button("换一组")
            .on_press(Message::Regenerate)
            .style(style::ghost_button)
            .padding([10, 18]),
        button(if app.hidden { "显示" } else { "隐藏" })
            .on_press(Message::ToggleHidden)
            .style(style::ghost_button)
            .padding([10, 18]),
        button(if app.copied_at.is_some() {
            "已复制"
        } else {
            "复制密码"
        })
        .on_press(Message::Copy)
        .style(style::primary_button)
        .padding([10, 20]),
    ]
    .spacing(10)
    .into()
}

fn options<'a>(app: &'a App, palette: &'static Palette) -> Element<'a, Message> {
    let length_row = row![
        text("长度")
            .size(13)
            .color(palette.muted)
            .width(Length::Fixed(48.0)),
        slider(
            8u32..=64u32,
            app.options.length as u32,
            Message::LengthSlider
        )
        .step(1u32)
        .width(Length::Fill),
        button("−")
            .on_press(Message::LengthStep(-1))
            .style(style::ghost_button)
            .padding([6, 10]),
        text_input("20", &app.length_draft)
            .on_input(Message::LengthInput)
            .on_submit(Message::LengthSubmit)
            .style(style::input)
            .padding([6, 8])
            .width(Length::Fixed(64.0)),
        button("+")
            .on_press(Message::LengthStep(1))
            .style(style::ghost_button)
            .padding([6, 10]),
    ]
    .spacing(10)
    .align_y(Alignment::Center);

    let groups = column![
        row![
            checkbox(app.options.upper)
                .label("大写 A-Z")
                .on_toggle(Message::ToggleUpper),
            checkbox(app.options.lower)
                .label("小写 a-z")
                .on_toggle(Message::ToggleLower),
        ]
        .spacing(28),
        row![
            checkbox(app.options.digits)
                .label("数字 0-9")
                .on_toggle(Message::ToggleDigits),
            checkbox(app.options.symbols)
                .label("符号 !@#$")
                .on_toggle(Message::ToggleSymbols),
        ]
        .spacing(28),
    ]
    .spacing(10);

    let similar = toggler(app.options.exclude_similar)
        .label("排除易混淆字符（O 0 I l 1）")
        .on_toggle(Message::ToggleExcludeSimilar);

    column![
        text("选项").size(12).color(palette.muted),
        length_row,
        groups,
        similar,
    ]
    .spacing(16)
    .into()
}

fn toast_layer<'a>(toast: &'a Toast, palette: &'static Palette) -> Element<'a, Message> {
    let (background, text_color, border_color) = match toast.kind {
        ToastKind::Success => (palette.success_bg, palette.success, palette.success),
        ToastKind::Error => (palette.danger_bg, palette.danger, palette.danger),
    };

    let card = container(text(toast.text.as_str()).size(13).color(text_color))
        .style(move |_theme| iced::widget::container::Style {
            background: Some(Background::Color(background)),
            text_color: Some(text_color),
            border: Border {
                color: border_color,
                width: 1.0,
                radius: 10.0.into(),
            },
            ..Default::default()
        })
        .padding([10, 16]);

    container(card)
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(iced::alignment::Horizontal::Center)
        .align_y(iced::alignment::Vertical::Bottom)
        .padding(24)
        .into()
}
