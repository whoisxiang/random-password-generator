// Copyright © 2026 sweet_fish2050
// 本程序依据 GNU GPL v3 协议发布。
// SPDX-License-Identifier: GPL-3.0-or-later

//! 随机密码生成与熵计算。
//!
//! 纯逻辑模块，不依赖 GUI，随机源通过 [`RandomSource`] 注入以便单元测试。

use std::fmt;

pub const LENGTH_MIN: usize = 8;
pub const LENGTH_MAX: usize = 64;

const UPPER: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZ";
const LOWER: &str = "abcdefghijklmnopqrstuvwxyz";
const DIGITS: &str = "0123456789";
const SYMBOLS: &str = "!@#$%^&*()-_=+[]{};:,.?";
const SIMILAR: &[char] = &['O', '0', 'I', 'l', '1'];

/// 整串拒绝采样的最大尝试次数，避免异常配置下死循环。
const MAX_ATTEMPTS: u32 = 100_000;

/// 生成选项。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Options {
    pub length: usize,
    pub upper: bool,
    pub lower: bool,
    pub digits: bool,
    pub symbols: bool,
    pub exclude_similar: bool,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            length: 20,
            upper: true,
            lower: true,
            digits: true,
            symbols: true,
            exclude_similar: true,
        }
    }
}

impl Options {
    /// 返回每个已选字符组的字符集合（已按 `exclude_similar` 过滤）。
    fn groups(&self) -> Vec<Vec<u8>> {
        let mut groups = Vec::with_capacity(4);

        for (enabled, chars) in [
            (self.upper, UPPER),
            (self.lower, LOWER),
            (self.digits, DIGITS),
            (self.symbols, SYMBOLS),
        ] {
            if !enabled {
                continue;
            }

            let group: Vec<u8> = chars
                .chars()
                .filter(|character| !(self.exclude_similar && SIMILAR.contains(character)))
                .map(|character| character as u8)
                .collect();

            if !group.is_empty() {
                groups.push(group);
            }
        }

        groups
    }
}

/// 生成结果。
#[derive(Debug, Clone, PartialEq)]
pub struct Generated {
    pub value: String,
    pub entropy_bits: f64,
}

/// 生成失败原因，`Display` 输出界面直接展示的中文文案。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GenerateError {
    InvalidLength,
    NoGroups,
    Random(String),
    Exhausted,
}

impl fmt::Display for GenerateError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidLength => write!(formatter, "密码长度需在 8–64 之间"),
            Self::NoGroups => write!(formatter, "请至少选择一种字符"),
            Self::Random(reason) => write!(formatter, "无法使用安全随机数：{reason}"),
            Self::Exhausted => write!(formatter, "无法生成满足条件的密码，请重试"),
        }
    }
}

impl std::error::Error for GenerateError {}

/// 可注入的随机字节源。
pub trait RandomSource {
    fn fill(&mut self, buffer: &mut [u8]) -> Result<(), String>;
}

/// 操作系统 CSPRNG（Windows 上为 `ProcessPrng`）。
#[derive(Debug, Default, Clone, Copy)]
pub struct OsRandom;

impl RandomSource for OsRandom {
    fn fill(&mut self, buffer: &mut [u8]) -> Result<(), String> {
        getrandom::fill(buffer).map_err(|error| error.to_string())
    }
}

/// 按选项生成随机密码与对应熵值。
pub fn generate(options: Options, rng: &mut impl RandomSource) -> Result<Generated, GenerateError> {
    generate_with_limit(options, rng, MAX_ATTEMPTS)
}

/// 与 [`generate`] 相同，但允许测试注入更小的尝试次数上限。
pub(crate) fn generate_with_limit(
    options: Options,
    rng: &mut impl RandomSource,
    max_attempts: u32,
) -> Result<Generated, GenerateError> {
    if !(LENGTH_MIN..=LENGTH_MAX).contains(&options.length) {
        return Err(GenerateError::InvalidLength);
    }

    let groups = options.groups();
    if groups.is_empty() {
        return Err(GenerateError::NoGroups);
    }

    let pool: Vec<u8> = groups.iter().flatten().copied().collect();
    let limit = pool.len() as u32;

    for _ in 0..max_attempts {
        let mut candidate = Vec::with_capacity(options.length);

        for _ in 0..options.length {
            let index = random_below(rng, limit)? as usize;
            candidate.push(pool[index]);
        }

        // 整串拒绝：仅当每个所选字符组都至少出现一次时才接受，
        // 从而在所有合法密码上保持均匀分布。
        let satisfies_groups = groups
            .iter()
            .all(|group| candidate.iter().any(|byte| group.contains(byte)));

        if satisfies_groups {
            let value = String::from_utf8(candidate).expect("character sets are ASCII");
            return Ok(Generated {
                value,
                entropy_bits: entropy_bits(options),
            });
        }
    }

    Err(GenerateError::Exhausted)
}

/// 无偏随机整数：拒绝采样消除取模偏差。
fn random_below(rng: &mut impl RandomSource, limit: u32) -> Result<u32, GenerateError> {
    debug_assert!(limit > 0);

    let limit = u64::from(limit);
    let space = 1u64 << 32;
    let zone = space - (space % limit);

    loop {
        let mut bytes = [0u8; 4];
        rng.fill(&mut bytes).map_err(GenerateError::Random)?;
        let value = u64::from(u32::from_le_bytes(bytes));

        if value < zone {
            return Ok((value % limit) as u32);
        }
    }
}

/// 精确统计「长度固定且包含所有所选字符组」的密码数量后取 log2。
///
/// 使用容斥原理：`Σ (-1)^|S| × (total - excluded(S))^length`。
/// 最大数量为 `85^64 ≈ 3e123`，远小于 f64 上限，故无需大整数。
pub fn entropy_bits(options: Options) -> f64 {
    let groups = options.groups();
    if groups.is_empty() || !(LENGTH_MIN..=LENGTH_MAX).contains(&options.length) {
        return 0.0;
    }

    let sizes: Vec<usize> = groups.iter().map(Vec::len).collect();
    let total: usize = sizes.iter().sum();
    let count_subsets = 1usize << sizes.len();

    let mut count = 0.0f64;

    for mask in 0..count_subsets {
        let mut excluded = 0usize;
        let mut bits = 0u32;

        for (index, size) in sizes.iter().enumerate() {
            if mask & (1 << index) != 0 {
                excluded += size;
                bits += 1;
            }
        }

        let remaining = total.saturating_sub(excluded) as f64;
        let term = remaining.powi(options.length as i32);

        if bits % 2 == 0 {
            count += term;
        } else {
            count -= term;
        }
    }

    if count <= 0.0 {
        0.0
    } else {
        count.log2()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 按脚本依次输出字节并循环。
    struct ScriptedSource {
        bytes: Vec<u8>,
        position: usize,
    }

    impl ScriptedSource {
        fn new(bytes: Vec<u8>) -> Self {
            Self { bytes, position: 0 }
        }
    }

    impl RandomSource for ScriptedSource {
        fn fill(&mut self, buffer: &mut [u8]) -> Result<(), String> {
            for byte in buffer.iter_mut() {
                *byte = self.bytes[self.position % self.bytes.len()];
                self.position += 1;
            }
            Ok(())
        }
    }

    struct FailingSource;

    impl RandomSource for FailingSource {
        fn fill(&mut self, _buffer: &mut [u8]) -> Result<(), String> {
            Err("模拟随机源失败".to_owned())
        }
    }

    fn generate_ok(options: Options) -> Generated {
        let mut rng = OsRandom;
        generate(options, &mut rng).expect("generation should succeed")
    }

    fn all_groups(length: usize) -> Options {
        Options {
            length,
            upper: true,
            lower: true,
            digits: true,
            symbols: true,
            exclude_similar: false,
        }
    }

    #[test]
    fn default_options_match_specification() {
        let options = Options::default();
        assert_eq!(options.length, 20);
        assert!(options.upper && options.lower && options.digits);
        assert!(options.symbols && options.exclude_similar);
    }

    #[test]
    fn length_boundaries_are_valid() {
        for length in [LENGTH_MIN, LENGTH_MAX] {
            let generated = generate_ok(all_groups(length));
            assert_eq!(generated.value.len(), length);
        }
    }

    #[test]
    fn out_of_range_length_is_rejected() {
        let mut rng = OsRandom;

        for length in [0, 7, 65, 200] {
            let options = Options {
                length,
                ..Options::default()
            };

            assert_eq!(
                generate(options, &mut rng),
                Err(GenerateError::InvalidLength)
            );
        }
    }

    #[test]
    fn no_selected_groups_is_rejected() {
        let options = Options {
            upper: false,
            lower: false,
            digits: false,
            symbols: false,
            ..Options::default()
        };
        let mut rng = OsRandom;

        assert_eq!(generate(options, &mut rng), Err(GenerateError::NoGroups));
    }

    #[test]
    fn every_selected_group_appears() {
        let options = all_groups(8);

        for _ in 0..50 {
            let generated = generate_ok(options);
            let value = generated.value.as_bytes();

            assert!(value.iter().any(u8::is_ascii_uppercase));
            assert!(value.iter().any(u8::is_ascii_lowercase));
            assert!(value.iter().any(u8::is_ascii_digit));
            assert!(value.iter().any(|byte| !byte.is_ascii_alphanumeric()));
        }
    }

    #[test]
    fn only_selected_charsets_are_used() {
        let options = Options {
            length: 32,
            upper: false,
            lower: true,
            digits: true,
            symbols: false,
            exclude_similar: true,
        };

        for _ in 0..20 {
            let generated = generate_ok(options);

            for character in generated.value.chars() {
                assert!(character.is_ascii_lowercase() || character.is_ascii_digit());
            }
        }
    }

    #[test]
    fn similar_characters_are_excluded() {
        let options = Options {
            exclude_similar: true,
            ..all_groups(64)
        };

        for _ in 0..20 {
            let generated = generate_ok(options);

            for character in generated.value.chars() {
                assert!(
                    !SIMILAR.contains(&character),
                    "found similar char {character}"
                );
            }
        }
    }

    #[test]
    fn failed_random_source_is_reported() {
        let mut rng = FailingSource;

        assert_eq!(
            generate(Options::default(), &mut rng),
            Err(GenerateError::Random("模拟随机源失败".to_owned()))
        );
    }

    #[test]
    fn impossible_options_exhaust_retries() {
        // 全零随机源让每个位置都取候选池第一个字符 'A'，
        // 小写/数字/符号组永远不可能出现。
        let mut rng = ScriptedSource::new(vec![0, 0, 0, 0]);

        assert_eq!(
            generate_with_limit(all_groups(8), &mut rng, 10),
            Err(GenerateError::Exhausted)
        );
    }

    #[test]
    fn random_below_rejects_biased_values() {
        // 前 4 字节 0xFFFF_FFFF 落在拒绝区，随后取到 5。
        let mut rng = ScriptedSource::new(vec![0xFF, 0xFF, 0xFF, 0xFF, 5, 0, 0, 0]);

        assert_eq!(random_below(&mut rng, 10), Ok(5));
    }

    #[test]
    fn entropy_for_single_group_matches_cartesian_product() {
        let options = Options {
            length: 8,
            upper: false,
            lower: true,
            digits: false,
            symbols: false,
            exclude_similar: false,
        };

        let expected = 8.0 * 26f64.log2();
        assert!((entropy_bits(options) - expected).abs() < 1e-9);
    }

    #[test]
    fn entropy_for_two_groups_uses_inclusion_exclusion() {
        let options = Options {
            length: 8,
            upper: true,
            lower: true,
            digits: false,
            symbols: false,
            exclude_similar: false,
        };

        // 全部 52^8 减去缺大写 26^8、缺小写 26^8，再加上两者都缺的 0。
        let expected = (52f64.powi(8) - 2.0 * 26f64.powi(8)).log2();
        assert!((entropy_bits(options) - expected).abs() < 1e-9);
    }

    #[test]
    fn entropy_for_all_groups_is_close_to_pool_size() {
        let options = all_groups(64);
        let upper_bound = 64.0 * 85f64.log2();
        let value = entropy_bits(options);

        // 长度 64 时「缺某一组」的概率极小（最大的一项约为 3e-4），
        // 因此熵非常接近无约束上限，但不会相等。
        assert!(value.is_finite());
        assert!(value < upper_bound);
        assert!(upper_bound - value < 0.01);
    }

    #[test]
    fn excluding_similar_characters_lowers_entropy() {
        let with = all_groups(20);
        let without = Options {
            exclude_similar: true,
            ..with
        };

        assert!(entropy_bits(without) < entropy_bits(with));
        assert!(entropy_bits(without) > 0.0);
    }

    #[test]
    fn error_messages_match_specification() {
        assert_eq!(
            GenerateError::InvalidLength.to_string(),
            "密码长度需在 8–64 之间"
        );
        assert_eq!(GenerateError::NoGroups.to_string(), "请至少选择一种字符");
        assert_eq!(
            GenerateError::Random("原因".to_owned()).to_string(),
            "无法使用安全随机数：原因"
        );
        assert_eq!(
            GenerateError::Exhausted.to_string(),
            "无法生成满足条件的密码，请重试"
        );
    }
}
