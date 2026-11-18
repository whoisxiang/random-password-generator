// Copyright © 2026 sweet_fish2050
// 本程序依据 GNU GPL v3 协议发布。
// SPDX-License-Identifier: GPL-3.0-or-later

//! 应用状态与更新逻辑。

use std::time::Duration;

use iced::{Element, Subscription, Task, Theme};
use zeroize::Zeroizing;

use crate::clipboard;
use crate::generator::{self, Generated, Options, OsRandom};
use crate::ui;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToastKind {
    Success,
    Error,
}

#[derive(Debug, Clone)]
pub struct Toast {
    pub kind: ToastKind,
    pub text: String,
    pub born: iced::time::Instant,
}

#[derive(Debug, Clone)]
pub enum Message {
    LengthInput(String),
    LengthSubmit,
    LengthSlider(u32),
    LengthStep(i32),
    ToggleUpper(bool),
    ToggleLower(bool),
    ToggleDigits(bool),
    ToggleSymbols(bool),
    ToggleExcludeSimilar(bool),
    Regenerate,
    ToggleHidden,
    Copy,
    Tick(iced::time::Instant),
}

#[derive(Debug, Clone, Copy)]
enum Group {
    Upper,
    Lower,
    Digits,
    Symbols,
}

pub struct App {
    pub options: Options,
    pub length_draft: String,
    pub password: Zeroizing<String>,
    pub entropy: f64,
    pub hidden: bool,
    pub copied_at: Option<iced::time::Instant>,
    pub toast: Option<Toast>,
}

impl App {
    pub fn boot() -> (Self, Task<Message>) {
        let options = Options::default();

        let mut app = Self {
            options,
            length_draft: options.length.to_string(),
            password: Zeroizing::new(String::new()),
            entropy: 0.0,
            hidden: false,
            copied_at: None,
            toast: None,
        };

        app.regenerate();

        (app, Task::none())
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::LengthInput(value) => {
                // 保留草稿；合法值立即生效。
                self.length_draft = value.chars().filter(char::is_ascii_digit).take(3).collect();

                if let Some(length) = self.valid_draft_length() {
                    if length != self.options.length {
                        self.apply_length(length);
                    }
                }
            }
            Message::LengthSubmit => self.commit_length_draft(),
            Message::LengthSlider(value) => self.apply_length(value as usize),
            Message::LengthStep(delta) => {
                let next = (self.options.length as i32 + delta)
                    .clamp(generator::LENGTH_MIN as i32, generator::LENGTH_MAX as i32)
                    as usize;

                if next != self.options.length {
                    self.apply_length(next);
                }
            }
            Message::ToggleUpper(enabled) => {
                self.toggle_group(Group::Upper, enabled);
            }
            Message::ToggleLower(enabled) => {
                self.toggle_group(Group::Lower, enabled);
            }
            Message::ToggleDigits(enabled) => {
                self.toggle_group(Group::Digits, enabled);
            }
            Message::ToggleSymbols(enabled) => {
                self.toggle_group(Group::Symbols, enabled);
            }
            Message::ToggleExcludeSimilar(enabled) => {
                self.commit_length_draft();
                self.options.exclude_similar = enabled;
                self.regenerate();
            }
            Message::Regenerate => {
                self.commit_length_draft();
                self.regenerate();
            }
            Message::ToggleHidden => {
                self.hidden = !self.hidden;
            }
            Message::Copy => self.copy(),
            Message::Tick(now) => self.tick(now),
        }

        Task::none()
    }

    pub fn view(&self) -> Element<'_, Message> {
        ui::view(self)
    }

    pub fn subscription(&self) -> Subscription<Message> {
        let mut subscriptions = vec![iced::keyboard::listen().filter_map(|event| match event {
            iced::keyboard::Event::KeyPressed { key, modifiers, .. } => match key.as_ref() {
                iced::keyboard::Key::Character("c") if modifiers.command() => Some(Message::Copy),
                iced::keyboard::Key::Named(iced::keyboard::key::Named::Enter) => {
                    Some(Message::Regenerate)
                }
                _ => None,
            },
            _ => None,
        })];

        // 只有存在 Toast 或「已复制」提示时才需要计时器。
        if self.toast.is_some() || self.copied_at.is_some() {
            subscriptions.push(iced::time::every(Duration::from_millis(100)).map(Message::Tick));
        }

        Subscription::batch(subscriptions)
    }

    pub fn theme(&self) -> Theme {
        Theme::Light
    }

    fn regenerate(&mut self) {
        let mut rng = OsRandom;

        match generator::generate(self.options, &mut rng) {
            Ok(Generated {
                value,
                entropy_bits,
            }) => {
                self.password = Zeroizing::new(value);
                self.entropy = entropy_bits;
                self.copied_at = None;
            }
            Err(error) => {
                self.password = Zeroizing::new(String::new());
                self.entropy = 0.0;
                self.copied_at = None;
                self.show_toast(ToastKind::Error, error.to_string());
            }
        }
    }

    fn apply_length(&mut self, length: usize) {
        let length = length.clamp(generator::LENGTH_MIN, generator::LENGTH_MAX);
        self.length_draft = length.to_string();

        if length != self.options.length {
            self.options.length = length;
            self.regenerate();
        }
    }

    fn valid_draft_length(&self) -> Option<usize> {
        self.length_draft
            .parse::<usize>()
            .ok()
            .filter(|length| (generator::LENGTH_MIN..=generator::LENGTH_MAX).contains(length))
    }

    /// 提交长度草稿：合法则应用，非法则回滚并提示。
    /// iced 的 text_input 没有失焦事件，因此在回车或操作其他控件时提交。
    fn commit_length_draft(&mut self) {
        match self.valid_draft_length() {
            Some(length) => {
                if length != self.options.length {
                    self.apply_length(length);
                } else {
                    self.length_draft = length.to_string();
                }
            }
            None => {
                self.length_draft = self.options.length.to_string();
                self.show_toast(ToastKind::Error, "密码长度需在 8–64 之间".to_owned());
            }
        }
    }

    fn toggle_group(&mut self, group: Group, enabled: bool) {
        self.commit_length_draft();

        let mut next = self.options;
        match group {
            Group::Upper => next.upper = enabled,
            Group::Lower => next.lower = enabled,
            Group::Digits => next.digits = enabled,
            Group::Symbols => next.symbols = enabled,
        }

        if !(next.upper || next.lower || next.digits || next.symbols) {
            self.show_toast(ToastKind::Error, "请至少保留一种字符。".to_owned());
            return;
        }

        self.options = next;
        self.regenerate();
    }

    fn copy(&mut self) {
        if self.password.is_empty() {
            self.show_toast(ToastKind::Error, "没有可复制的密码".to_owned());
            return;
        }

        match clipboard::copy_concealed(self.password.as_str()) {
            Ok(()) => {
                self.copied_at = Some(iced::time::Instant::now());
                self.show_toast(ToastKind::Success, "已复制到剪贴板".to_owned());
            }
            Err(error) => {
                self.show_toast(ToastKind::Error, format!("复制到剪贴板失败：{error}"));
            }
        }
    }

    fn tick(&mut self, now: iced::time::Instant) {
        if let Some(toast) = &self.toast {
            let lifetime = match toast.kind {
                ToastKind::Success => Duration::from_secs(3),
                ToastKind::Error => Duration::from_secs(7),
            };

            if now.saturating_duration_since(toast.born) >= lifetime {
                self.toast = None;
            }
        }

        if let Some(copied_at) = self.copied_at {
            if now.saturating_duration_since(copied_at) >= Duration::from_millis(2_200) {
                self.copied_at = None;
            }
        }
    }

    fn show_toast(&mut self, kind: ToastKind, text: String) {
        self.toast = Some(Toast {
            kind,
            text,
            born: iced::time::Instant::now(),
        });
    }
}
