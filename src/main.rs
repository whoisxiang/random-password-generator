// Copyright © 2026 sweet_fish2050
// 本程序依据 GNU GPL v3 协议发布。
// SPDX-License-Identifier: GPL-3.0-or-later

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod clipboard;
mod generator;
mod style;
mod ui;

fn main() -> iced::Result {
    iced::application(app::App::boot, app::App::update, app::App::view)
        .title("随机密码生成器")
        .theme(app::App::theme)
        .subscription(app::App::subscription)
        .window(iced::window::Settings {
            size: iced::Size::new(900.0, 760.0),
            min_size: Some(iced::Size::new(640.0, 560.0)),
            position: iced::window::Position::Centered,
            ..Default::default()
        })
        .run()
}
