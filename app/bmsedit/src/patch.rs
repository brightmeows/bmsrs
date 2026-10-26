//! BMS 行级 patch 引擎。
//!
//! 以 `bms-document` 的字节保真 `Document` 为工作面：查询与后验
//! 验证走完整管线（token → 控制流展开 → `Bms`），写操作按
//! 三档策略回写——行内替换、行插入/删除、兜底全量重写。
//! 每次编辑提交后重新解析并运行后验检查，失败即报错且不落盘。

use std::path::{Path, PathBuf};

use bmsrs::bms::control_flow::FlowDoc;
use bmsrs::bms::document::{Document, DocumentOptions, ErrorStrategy};
use bmsrs::bms::parser::Bms;
use bmsrs::bms::tokenizer::{BmsHeader, BmsMessage, BmsToken};

/// 提取头部命令名与值（总枚举分发到子枚举的 derive 格式化）。
#[must_use]
pub fn header_name_value(header: &BmsHeader) -> (String, String) {
    match header {
        BmsHeader::ResDefAudio(h) => h.format_header(),
        BmsHeader::Timing(h) => h.format_header(),
        BmsHeader::ResDefVisual(h) => h.format_header(),
        BmsHeader::ControlFlow(h) => h.format_header(),
        BmsHeader::Gameplay(h) => h.format_header(),
        BmsHeader::Display(h) => h.format_header(),
        BmsHeader::Metadata(h) => h.format_header(),
        BmsHeader::Fallback(h) => (h.command.clone(), h.value.clone()),
    }
}
use rand::SeedableRng as _;
use rand::rngs::StdRng;

use crate::error::BmsEditError;

/// 分支选择用固定种子默认值——保证同一文件多次编辑落在同一分支。
pub const DEFAULT_SEED: u64 = 1;

/// 一个已加载的 BMS 文档：保真行 + 语义模型。
pub struct BmsDoc {
    /// 字节保真文档（行级工作面）。
    pub document: Document,
    /// 语义模型（查询与验证）。
    pub bms: Bms,
    /// 读取警告（含分支结构警告）。
    pub warnings: Vec<String>,
    /// 分支选择种子。
    seed: u64,
}

/// 由行集合组装语义模型的公共路径。
fn build_bms(document: &Document, seed: u64) -> Result<Bms, BmsEditError> {
    let pairs: Vec<_> = document
        .token_results_with(ErrorStrategy::CollectAll)
        .filter_map(|(line, result)| result.ok().map(|token| (line, token)))
        .collect();
    let flow = FlowDoc::from_tokens(pairs)
        .map_err(|err| BmsEditError::Parse(format!("control flow: {err}")))?;
    let mut rng = StdRng::seed_from_u64(seed);
    let flat = flow.select_branches(&mut rng).0;
    Ok(Bms::from_flat_tokens(flat))
}

impl BmsDoc {
    /// 从字节加载。
    ///
    /// # Errors
    ///
    /// 控制流结构不合法时返回 [`BmsEditError::Parse`]。
    pub fn from_bytes(bytes: Vec<u8>, seed: u64) -> Result<Self, BmsEditError> {
        let document = Document::from_bytes_with(bytes, &DocumentOptions::default());
        let mut warnings = Vec::new();
        for (line, result) in document.token_results_with(ErrorStrategy::CollectAll) {
            if let Err(err) = result {
                warnings.push(format!("line {line}: skipped ({err})"));
            }
        }
        let bms = build_bms(&document, seed)?;
        for warn in flow_warnings(&document) {
            warnings.push(warn);
        }
        Ok(Self {
            document,
            bms,
            warnings,
            seed,
        })
    }

    /// 从文件加载。
    ///
    /// # Errors
    ///
    /// 文件读取失败或控制流结构不合法时返回 [`BmsEditError`]。
    pub fn from_path(path: &Path, seed: u64) -> Result<Self, BmsEditError> {
        let bytes = std::fs::read(path).map_err(|source| BmsEditError::Io {
            path: path.to_owned(),
            source,
        })?;
        Self::from_bytes(bytes, seed)
    }

    /// 提交一次编辑：重新解析、运行后验检查、原子写回。
    ///
    /// 验证失败时文件保持原样。
    ///
    /// # Errors
    ///
    /// 后验检查失败返回 [`BmsEditError::Verify`]；写盘失败返回
    /// [`BmsEditError::Io`]。
    pub fn commit(
        &mut self,
        path: &Path,
        verify: impl Fn(&Bms) -> Result<(), String>,
    ) -> Result<(), BmsEditError> {
        let bytes = self.document.render();
        let rebuilt = Self::from_bytes(bytes.clone(), self.seed)?;
        verify(&rebuilt.bms).map_err(BmsEditError::Verify)?;
        self.document = rebuilt.document;
        self.bms = rebuilt.bms;
        self.warnings = rebuilt.warnings;
        atomic_write(path, &bytes)
    }

    /// 定位一个通道组的全部行。
    ///
    /// 通道匹配大小写不敏感（规范化为大写比较）。
    #[must_use]
    pub fn message_lines(&self, measure: u16, channel: &str) -> Vec<(usize, BmsMessage)> {
        let target = channel.to_ascii_uppercase();
        self.document
            .lines()
            .iter()
            .enumerate()
            .filter_map(|(index, line)| match line.kind() {
                bmsrs::bms::document::LineKind::Message(msg) => Some((index, msg)),
                _ => None,
            })
            .filter(|(_, msg)| msg.track() == measure && msg.channel().as_str() == target)
            .map(|(index, msg)| (index, msg.clone()))
            .collect()
    }

    /// 将一个通道组的逻辑位置映射回写为单行（三档策略）。
    ///
    /// `slots` 为按槽位升序的 `(slot, value)` 对；分辨率
    /// `resolution` 为槽位总数。
    ///
    /// # Errors
    ///
    /// 行编辑越界或文档层拒绝变更时返回 [`BmsEditError`]。
    pub fn write_channel_group(
        &mut self,
        measure: u16,
        channel: &str,
        resolution: usize,
        slots: &[(usize, String)],
    ) -> Result<(), BmsEditError> {
        let body = bmsrs::bms::tokenizer::BmsMessage::encode_body((0..resolution).map(|i| {
            slots
                .iter()
                .find(|(slot, _)| *slot == i)
                .map_or("00", |(_, v)| v.as_str())
        }));
        if body.chars().all(|c| c == '0') {
            // 全空组：删除既有行，不落新行。
            let lines = self.message_lines(measure, channel);
            for (index, _) in lines.into_iter().rev() {
                self.document
                    .remove_line(index)
                    .map_err(|err| BmsEditError::Usage(format!("remove line: {err}")))?;
            }
            return Ok(());
        }
        let lines = self.message_lines(measure, channel);
        match lines.as_slice() {
            [] => {
                let text = line_text(measure, channel, &body);
                let at = self.insert_position(measure, channel);
                self.document
                    .insert_line(at, &text)
                    .map_err(|err| BmsEditError::Usage(format!("insert line: {err}")))?;
            }
            [(first, _)] => {
                let text = line_text(measure, channel, &body);
                self.document
                    .replace_content(*first, &text)
                    .map_err(|err| BmsEditError::Usage(format!("replace line: {err}")))?;
            }
            [(first, _), rest @ ..] => {
                // 多行组退化为局部重写：完整映射写入首行，其余行删除。
                let text = line_text(measure, channel, &body);
                self.document
                    .replace_content(*first, &text)
                    .map_err(|err| BmsEditError::Usage(format!("replace line: {err}")))?;
                // 行索引删除后位移：先收集原索引再降序删除。
                let mut doomed: Vec<usize> = rest.iter().map(|(index, _)| *index).collect();
                doomed.sort_unstable();
                for index in doomed.into_iter().rev() {
                    self.document
                        .remove_line(index)
                        .map_err(|err| BmsEditError::Usage(format!("remove line: {err}")))?;
                }
            }
        }
        Ok(())
    }

    /// 替换或插入一行头部命令。
    ///
    /// `command` 为命令名（如 `TITLE`、`WAV01`）；存在同名头部行
    /// 时原位替换（保留行级格式），否则插入到首个头部行之前，
    /// 无头部行时插入到文件开头。
    ///
    /// # Errors
    ///
    /// 行编辑越界或文档层拒绝变更时返回 [`BmsEditError`]。
    pub fn write_header_line(&mut self, command: &str, value: &str) -> Result<(), BmsEditError> {
        let target = command.to_ascii_uppercase();
        let mut header_line = None;
        for (index, line) in self.document.lines().iter().enumerate() {
            if let bmsrs::bms::document::LineKind::Header(header) = line.kind() {
                let (header_command, _) = header_name_value(header);
                let normalized = header_command
                    .trim_start_matches(['#', '%'])
                    .to_ascii_uppercase();
                if normalized == target {
                    header_line = Some(index);
                    break;
                }
                if header_line.is_none() {
                    header_line = Some(index);
                }
            }
        }
        let text = format!("#{command} {value}");
        match header_line {
            Some(at) => {
                self.document
                    .replace_content(at, &text)
                    .map_err(|err| BmsEditError::Usage(format!("replace header: {err}")))?;
            }
            None => {
                self.document
                    .insert_line(0, &text)
                    .map_err(|err| BmsEditError::Usage(format!("insert header: {err}")))?;
            }
        }
        Ok(())
    }

    /// 删除一行头部命令（命令名大小写不敏感）；不存在时静默返回。
    ///
    /// # Errors
    ///
    /// 行编辑越界或文档层拒绝变更时返回 [`BmsEditError`]。
    pub fn remove_header_line(&mut self, command: &str) -> Result<(), BmsEditError> {
        let target = command.to_ascii_uppercase();
        let mut found = None;
        for (index, line) in self.document.lines().iter().enumerate() {
            if let bmsrs::bms::document::LineKind::Header(header) = line.kind() {
                let (header_command, _) = header_name_value(header);
                let normalized = header_command
                    .trim_start_matches(['#', '%'])
                    .to_ascii_uppercase();
                if normalized == target {
                    found = Some(index);
                    break;
                }
            }
        }
        if let Some(index) = found {
            self.document
                .remove_line(index)
                .map_err(|err| BmsEditError::Usage(format!("remove header: {err}")))?;
        }
        Ok(())
    }

    /// 计算新通道行的插入位置。
    ///
    /// 保持（小节，通道）升序：插到最后一个 `(track, channel)` 字典序
    /// 更小的消息行之后；无更小者时插到首个消息行之前；无消息行时
    /// 追加到文档末尾。
    /// 计算新通道行的插入位置（保持（小节，通道）升序）。
    #[must_use]
    pub fn insert_position(&self, measure: u16, channel: &str) -> usize {
        let target = channel.to_ascii_uppercase();
        let mut position = self.document.lines().len();
        let mut anchored = false;
        for (index, line) in self.document.lines().iter().enumerate() {
            let bmsrs::bms::document::LineKind::Message(msg) = line.kind() else {
                continue;
            };
            let msg_channel = msg.channel().as_str().to_string();
            let before = (msg.track(), msg_channel.as_str()) < (measure, target.as_str());
            if !before && !anchored {
                position = index;
                anchored = true;
            }
        }
        position
    }
}

/// 通道组逻辑位置映射：约分后的（分子, 分母）→ 2 字符值。
pub type SlotMap = std::collections::BTreeMap<(u32, u32), String>;

/// 读取一个通道组的合并逻辑映射。
///
/// 多行按解析合并语义折叠（非 `00` 覆盖，`00` 保留），位置以
/// 约分分数表示，分辨率差异在分数域消解。
#[must_use]
pub fn group_slots(doc: &BmsDoc, measure: u16, channel: &str) -> SlotMap {
    let mut map = SlotMap::new();
    for (_, msg) in doc.message_lines(measure, channel) {
        let denom = msg.body.chars().count() / 2;
        if denom == 0 {
            continue;
        }
        #[expect(
            clippy::cast_possible_truncation,
            reason = "per-measure slot counts fit u32 in practice"
        )]
        let denom_u32 = denom as u32;
        for (slot, value) in chunk_values(&msg.body).enumerate() {
            let (numer, reduced_denom) = reduce(u32::try_from(slot).unwrap_or(0), denom_u32);
            let existing_is_empty = map
                .get(&(numer, reduced_denom))
                .is_none_or(String::is_empty);
            if value == "00" {
                continue;
            }
            if existing_is_empty || value != "00" {
                map.insert((numer, reduced_denom), value.to_owned());
            }
        }
    }
    map.retain(|_, value| value != "00");
    map
}

/// 对通道组的逻辑映射应用变更并回写（三档策略）。
///
/// `edit` 在合并后的逻辑映射上就地修改；回写分辨率取能精确
/// 表示全部位置的最小 4 倍数（上限 1024，超出报 [`BmsEditError::Usage`]）。
///
/// # Errors
///
/// 行编辑越界、分辨率超限或文档层拒绝变更时返回 [`BmsEditError`]。
pub fn edit_group(
    doc: &mut BmsDoc,
    measure: u16,
    channel: &str,
    edit: impl FnOnce(&mut SlotMap),
) -> Result<(), BmsEditError> {
    let mut map = group_slots(doc, measure, channel);
    edit(&mut map);
    map.retain(|_, value| value != "00");
    let denoms: Vec<u32> = map.keys().map(|&(_, denom)| denom).collect();
    let resolution = min_resolution(&denoms)
        .ok_or_else(|| BmsEditError::Usage("required resolution exceeds 1024 slots".to_owned()))?;
    let slots: Vec<(usize, String)> = map
        .iter()
        .map(|(&(numer, denom), value)| {
            (
                usize::try_from(numer * u32::try_from(resolution).unwrap_or(u32::MAX) / denom)
                    .unwrap_or(usize::MAX),
                value.clone(),
            )
        })
        .collect();
    doc.write_channel_group(measure, channel, resolution, &slots)
}

/// 能被全部分母整除的最小 4 倍数槽位数（上限 1024）。
#[must_use]
pub fn min_resolution(denoms: &[u32]) -> Option<usize> {
    let mut lcm: u64 = 1;
    for &denom in denoms {
        lcm = lcm / gcd(lcm, u64::from(denom)) * u64::from(denom);
    }
    let multiple = lcm.div_ceil(4) * 4;
    usize::try_from(multiple).ok().filter(|size| *size <= 1024)
}

/// 最大公约数。
fn gcd(a: u64, b: u64) -> u64 {
    if b == 0 { a } else { gcd(b, a % b) }
}

/// 约分分数。
fn reduce(numer: u32, denom: u32) -> (u32, u32) {
    let g = gcd(u64::from(numer), u64::from(denom)).max(1);
    #[expect(
        clippy::cast_possible_truncation,
        reason = "gcd divides the u32 inputs, so quotients are within u32"
    )]
    let to_u32 = |value: u64| value as u32;
    (to_u32(u64::from(numer) / g), to_u32(u64::from(denom) / g))
}

/// 将正文按 2 字符切块迭代。
pub fn chunk_values(body: &str) -> impl Iterator<Item = &str> {
    body.as_bytes()
        .chunks(2)
        .map(|chunk| std::str::from_utf8(chunk).unwrap_or("00"))
}

/// 原子写：临时文件 + rename。
fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), BmsEditError> {
    let tmp = temp_sibling(path);
    std::fs::write(&tmp, bytes).map_err(|source| BmsEditError::Io {
        path: tmp.clone(),
        source,
    })?;
    std::fs::rename(&tmp, path).map_err(|source| BmsEditError::Io {
        path: path.to_owned(),
        source,
    })
}

/// 同目录同名的临时文件路径。
fn temp_sibling(path: &Path) -> PathBuf {
    let mut name = path.file_name().map_or_else(
        || std::ffi::OsString::from("bmsedit.tmp"),
        std::ffi::OsStr::to_os_string,
    );
    name.push(".bmsedit-tmp");
    path.with_file_name(name)
}

/// 组装通道行文本。
fn line_text(measure: u16, channel: &str, body: &str) -> String {
    format!("#{measure:03}{}:{body}", channel.to_ascii_uppercase())
}

/// 扫描文档中的控制流命令，产出警告。
fn flow_warnings(document: &Document) -> Vec<String> {
    let mut warnings = Vec::new();
    for (line, token) in document.tokens() {
        if let BmsToken::Header(bmsrs::bms::tokenizer::BmsHeader::ControlFlow(_)) = token {
            warnings.push(format!(
                "line {line}: branch structure (#RANDOM/#SWITCH) present; \
                 editing operates on the selected branch only"
            ));
        }
    }
    warnings.dedup();
    warnings
}
