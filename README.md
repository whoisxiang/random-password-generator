# 随机密码生成器（Rust + iced）

本程序参考了[linuxdo-app-password-workshop](https://github.com/yfmeii/linuxdo-app-password-workshop) UI 设计
使用原生 Rust + [iced 0.14](https://iced.rs) 实现，
使用 tiny-skia 软件渲染方案，**做到完全离线、单文件运行**。

- 无 WebView2、无安装程序、无外部运行时；静态链接 MSVC CRT，无需 VC++ Redistributable。
- 无网络、无文件写入：密码只存在于内存中，退出或换一组时清零旧缓冲。
- 复制使用 Win32 原生剪贴板 API，并设置以下格式，使密码不进入剪贴板历史 / 云同步 / 监视工具：
  - `ExcludeClipboardContentFromMonitorProcessing`
  - `CanIncludeInClipboardHistory` = 0
  - `CanUploadToCloudClipboard` = 0

## 功能

- 长度 8–64（默认 20），滑杆、数字输入与 +/− 按钮同步。
- 大写 / 小写 / 数字 / 符号四组字符，可排除易混淆字符（O 0 I l 1）。
- CSPRNG（`getrandom`）拒绝采样，消除取模偏差；整串拒绝直到每个所选字符组至少出现一次。
- 熵按「满足所有字符组约束的密码空间」精确计数，强度分级：较弱 / 一般 / 强 / 很强。
- 隐藏模式最多显示 20 个圆点，不泄漏真实长度；复制仍复制真实密码。
- 浅色界面（`style.rs` 内同时提供深色配色，可按需切换 / 恢复跟随系统主题）。
- 快捷键：`Ctrl+C` 复制密码，`Enter` 换一组（数字输入框聚焦时 Enter 用于提交长度）。

## 构建

需要 Rust 1.88+（MSVC 工具链）。

```sh
cargo test
cargo build --release
```

产物：`target/release/random-password-generator.exe`。复制这一个 exe 即可运行。

> 首次构建需要联网下载 crate；若需完全离线构建，可先执行 `cargo vendor`，
> 并按 Cargo 提示配置 `.cargo/config.toml` 的 vendor 源，再在离线环境构建。
> 运行期不需要任何网络。

## 验证

- `cargo test`：字符集、每组必现、排除易混淆、长度边界、随机源错误、
  重试耗尽、拒绝采样、熵值已知用例。
- 手工核对：
  - 快速拖动滑杆后密码与设置一致；取消最后一个字符组被拒绝并提示。
  - 隐藏 / 显示、复制后 `Win+V` 历史中不出现密码。
  - 断网运行，生成与复制功能完整。
  - 最小窗口（640×560）布局正常。

## 已知差异

- iced 的文本输入没有失焦事件：长度输入的非法草稿在回车或操作其他控件时回滚。
- 不内置应用图标（仓库没有 .ico 资源），exe 使用系统默认图标。
- 中文依赖系统字体回退（Windows 11 自带微软雅黑）；极简系统若缺少字体，可后续内嵌 CJK 字体。
- exe 未签名，首次运行可能触发 SmartScreen；发布前应自行代码签名。

## 许可证

Copyright © 2026 sweet_fish2050

本程序依据 GNU GPL v3 协议发布。
