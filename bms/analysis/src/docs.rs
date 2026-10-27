//! 头部命令与通道的中文说明。

/// 头部命令说明。
#[derive(Debug, Clone, Copy)]
pub struct HeaderDoc {
    /// 命令前缀（不含 `#` / `%`）。
    pub command: &'static str,
    /// 显示标题。
    pub title: &'static str,
    /// 简要说明。
    pub summary: &'static str,
    /// 原始资料链接。
    pub link: &'static str,
}

/// 通道说明。
#[derive(Debug, Clone, Copy)]
pub struct ChannelDoc {
    /// 通道号或通道组。
    pub code: &'static str,
    /// 显示标题。
    pub title: &'static str,
    /// 简要说明。
    pub summary: &'static str,
    /// 原始资料链接。
    pub link: &'static str,
}

/// BMS command memo 链接。
const MEMO: &str = "https://hitkey.nekokan.dyndns.info/cmdsJP.htm";

/// 头部命令表。
pub static HEADER_DOCS: &[HeaderDoc] = &[
    HeaderDoc {
        command: "TITLE",
        title: "#TITLE",
        summary: "乐曲标题；显示在选曲和游玩界面。",
        link: MEMO,
    },
    HeaderDoc {
        command: "SUBTITLE",
        title: "#SUBTITLE",
        summary: "乐曲副标题；可与 #TITLE 组合显示。",
        link: MEMO,
    },
    HeaderDoc {
        command: "ARTIST",
        title: "#ARTIST",
        summary: "主要艺术家或作曲者。",
        link: MEMO,
    },
    HeaderDoc {
        command: "SUBARTIST",
        title: "#SUBARTIST",
        summary: "合作创作者或编曲者。",
        link: MEMO,
    },
    HeaderDoc {
        command: "GENRE",
        title: "#GENRE",
        summary: "音乐流派；部分播放器也接受 #GENLE。",
        link: MEMO,
    },
    HeaderDoc {
        command: "GENLE",
        title: "#GENLE",
        summary: "#GENRE 的常见拼写变体。",
        link: MEMO,
    },
    HeaderDoc {
        command: "MAKER",
        title: "#MAKER",
        summary: "谱面作者名。",
        link: MEMO,
    },
    HeaderDoc {
        command: "COMMENT",
        title: "#COMMENT",
        summary: "选曲列表中显示的说明文本。",
        link: MEMO,
    },
    HeaderDoc {
        command: "CHARSET",
        title: "#CHARSET",
        summary: "源文件编码声明，例如 Shift_JIS 或 UTF-8。",
        link: MEMO,
    },
    HeaderDoc {
        command: "URL",
        title: "%URL",
        summary: "作者网站 URL。",
        link: MEMO,
    },
    HeaderDoc {
        command: "EMAIL",
        title: "%EMAIL",
        summary: "作者邮箱地址。",
        link: MEMO,
    },
    HeaderDoc {
        command: "TEXT",
        title: "#TEXTxx",
        summary: "定时屏幕文本定义；由通道 99 引用。",
        link: MEMO,
    },
    HeaderDoc {
        command: "SONG",
        title: "#SONGxx",
        summary: "#TEXTxx 的同义定义。",
        link: MEMO,
    },
    HeaderDoc {
        command: "PLAYER",
        title: "#PLAYER",
        summary: "玩家模式：1 单打、2 双人、3 双打、4 对战。",
        link: MEMO,
    },
    HeaderDoc {
        command: "RANK",
        title: "#RANK",
        summary: "判定宽度；0 最严、4 最宽。",
        link: MEMO,
    },
    HeaderDoc {
        command: "DEFEXRANK",
        title: "#DEFEXRANK",
        summary: "以 NORMAL 为基准的默认判定宽度。",
        link: MEMO,
    },
    HeaderDoc {
        command: "EXRANK",
        title: "#EXRANKxx",
        summary: "按键级判定宽度定义；由 A0 通道引用。",
        link: MEMO,
    },
    HeaderDoc {
        command: "TOTAL",
        title: "#TOTAL",
        summary: "血槽总增量；影响通关难度。",
        link: MEMO,
    },
    HeaderDoc {
        command: "VOLWAV",
        title: "#VOLWAV",
        summary: "整体音量百分比。",
        link: MEMO,
    },
    HeaderDoc {
        command: "PLAYLEVEL",
        title: "#PLAYLEVEL",
        summary: "谱面难度数值。",
        link: MEMO,
    },
    HeaderDoc {
        command: "DIFFICULTY",
        title: "#DIFFICULTY",
        summary: "难度类别：1 BEGINNER 到 5 INSANE。",
        link: MEMO,
    },
    HeaderDoc {
        command: "OPTION",
        title: "#OPTION",
        summary: "谱面级选项定义。",
        link: MEMO,
    },
    HeaderDoc {
        command: "CHANGEOPTION",
        title: "#CHANGEOPTIONxx",
        summary: "按位置切换选项；由 A6 通道引用。",
        link: MEMO,
    },
    HeaderDoc {
        command: "LNTYPE",
        title: "#LNTYPE",
        summary: "长音符类型：1 RDM 记法、2 MGQ 记法。",
        link: MEMO,
    },
    HeaderDoc {
        command: "LNOBJ",
        title: "#LNOBJ",
        summary: "把指定 WAV ID 视为长音符终点。",
        link: MEMO,
    },
    HeaderDoc {
        command: "LNMODE",
        title: "#LNMODE",
        summary: "beatoraja 长音符模式：LN / CN / HCN。",
        link: MEMO,
    },
    HeaderDoc {
        command: "OCT/FP",
        title: "#OCT/FP",
        summary: "切换到 OCT/FP 模式。",
        link: MEMO,
    },
    HeaderDoc {
        command: "OCT",
        title: "#OCT",
        summary: "OCT/FP 模式的通道映射。",
        link: MEMO,
    },
    HeaderDoc {
        command: "FP",
        title: "#FP",
        summary: "OCT/FP 模式的通道映射。",
        link: MEMO,
    },
    HeaderDoc {
        command: "PREVIEW",
        title: "#PREVIEW",
        summary: "预览音频文件。",
        link: MEMO,
    },
    HeaderDoc {
        command: "PATH_WAV",
        title: "#PATH_WAV",
        summary: "WAV 文件搜索路径。",
        link: MEMO,
    },
    HeaderDoc {
        command: "STAGEFILE",
        title: "#STAGEFILE",
        summary: "游玩背景图。",
        link: MEMO,
    },
    HeaderDoc {
        command: "BANNER",
        title: "#BANNER",
        summary: "选曲列表横幅图。",
        link: MEMO,
    },
    HeaderDoc {
        command: "BACKBMP",
        title: "#BACKBMP",
        summary: "标题画面背景图。",
        link: MEMO,
    },
    HeaderDoc {
        command: "CHARFILE",
        title: "#CHARFILE",
        summary: "角色图片定义。",
        link: MEMO,
    },
    HeaderDoc {
        command: "END",
        title: "#END",
        summary: "谱面结束标记。",
        link: MEMO,
    },
    HeaderDoc {
        command: "BASE",
        title: "#BASE",
        summary: "索引进制：36 或 62。",
        link: MEMO,
    },
    HeaderDoc {
        command: "BPM",
        title: "#BPM",
        summary: "初始 BPM；带 ID 时定义扩展 BPM。",
        link: MEMO,
    },
    HeaderDoc {
        command: "EXBPM",
        title: "#EXBPMxx",
        summary: "扩展 BPM 定义；由 08 通道引用。",
        link: MEMO,
    },
    HeaderDoc {
        command: "BASEBPM",
        title: "#BASEBPM",
        summary: "自动 HI-SPEED 的参考 BPM。",
        link: MEMO,
    },
    HeaderDoc {
        command: "STOP",
        title: "#STOPxx",
        summary: "停止序列定义；由 09 通道引用。",
        link: MEMO,
    },
    HeaderDoc {
        command: "STP",
        title: "#STP",
        summary: "绝对时间停止：小节.位置 毫秒。",
        link: MEMO,
    },
    HeaderDoc {
        command: "SCROLL",
        title: "#SCROLLxx",
        summary: "滚动速度倍率定义；由 SC 通道引用。",
        link: MEMO,
    },
    HeaderDoc {
        command: "SPEED",
        title: "#SPEEDxx",
        summary: "视觉音符间距定义；由 SP 通道引用。",
        link: MEMO,
    },
    HeaderDoc {
        command: "WAV",
        title: "#WAVxx",
        summary: "音频资源定义；由 BGM、音符与 LN 通道引用。",
        link: MEMO,
    },
    HeaderDoc {
        command: "EXWAV",
        title: "#EXWAVxx",
        summary: "带声像、音量与频率参数的音频定义。",
        link: MEMO,
    },
    HeaderDoc {
        command: "WAVCMD",
        title: "#WAVCMD",
        summary: "对 WAV 应用音高、音量或时长命令。",
        link: MEMO,
    },
    HeaderDoc {
        command: "CDDA",
        title: "#CDDA",
        summary: "CD-DA 音轨定义。",
        link: MEMO,
    },
    HeaderDoc {
        command: "MIDIFILE",
        title: "#MIDIFILE",
        summary: "MIDI 文件定义。",
        link: MEMO,
    },
    HeaderDoc {
        command: "BMP",
        title: "#BMPxx",
        summary: "图片资源定义；由 BGA 通道引用。",
        link: MEMO,
    },
    HeaderDoc {
        command: "EXBMP",
        title: "#EXBMPxx",
        summary: "带透明色的图片定义。",
        link: MEMO,
    },
    HeaderDoc {
        command: "BGA",
        title: "#BGAxx",
        summary: "BGA 对象定义；由 04/06/07/0A 通道引用。",
        link: MEMO,
    },
    HeaderDoc {
        command: "@BGA",
        title: "#@BGAxx",
        summary: "以宽高形式定义 BGA 裁剪。",
        link: MEMO,
    },
    HeaderDoc {
        command: "POORBGA",
        title: "#POORBGA",
        summary: "MISS 时显示的 BGA 模式。",
        link: MEMO,
    },
    HeaderDoc {
        command: "SWBGA",
        title: "#SWBGAxx",
        summary: "逐帧动画 BGA 定义。",
        link: MEMO,
    },
    HeaderDoc {
        command: "ARGB",
        title: "#ARGBxx",
        summary: "带 ARGB 颜色的 BGA 定义。",
        link: MEMO,
    },
    HeaderDoc {
        command: "VIDEOFILE",
        title: "#VIDEOFILE",
        summary: "视频背景文件。",
        link: MEMO,
    },
    HeaderDoc {
        command: "VIDEOf/s",
        title: "#VIDEOf/s",
        summary: "视频播放帧率。",
        link: MEMO,
    },
    HeaderDoc {
        command: "VIDEOCOLORS",
        title: "#VIDEOCOLORS",
        summary: "视频颜色数。",
        link: MEMO,
    },
    HeaderDoc {
        command: "VIDEODLY",
        title: "#VIDEODLY",
        summary: "视频延迟。",
        link: MEMO,
    },
    HeaderDoc {
        command: "MOVIE",
        title: "#MOVIE",
        summary: "旧式视频定义。",
        link: MEMO,
    },
    HeaderDoc {
        command: "SEEK",
        title: "#SEEKxx",
        summary: "SEEK 定义；由 05 通道引用。",
        link: MEMO,
    },
    HeaderDoc {
        command: "ExtChr",
        title: "#ExtChr",
        summary: "扩展字符定义。",
        link: MEMO,
    },
    HeaderDoc {
        command: "RANDOM",
        title: "#RANDOM",
        summary: "随机块：在 [1, N] 中取值，供 #IF 选择。",
        link: MEMO,
    },
    HeaderDoc {
        command: "SETRANDOM",
        title: "#SETRANDOM",
        summary: "固定随机值，不调用 RNG。",
        link: MEMO,
    },
    HeaderDoc {
        command: "ENDRANDOM",
        title: "#ENDRANDOM",
        summary: "结束 #RANDOM 块。",
        link: MEMO,
    },
    HeaderDoc {
        command: "IF",
        title: "#IF",
        summary: "随机块内的条件分支。",
        link: MEMO,
    },
    HeaderDoc {
        command: "ELSEIF",
        title: "#ELSEIF",
        summary: "前序分支未命中时的条件分支。",
        link: MEMO,
    },
    HeaderDoc {
        command: "ELSE",
        title: "#ELSE",
        summary: "所有条件都未命中时的分支。",
        link: MEMO,
    },
    HeaderDoc {
        command: "ENDIF",
        title: "#ENDIF",
        summary: "结束 #IF 链。",
        link: MEMO,
    },
    HeaderDoc {
        command: "IFEND",
        title: "#IFEND",
        summary: "#ENDIF 的别名。",
        link: MEMO,
    },
    HeaderDoc {
        command: "END IF",
        title: "#END IF",
        summary: "#ENDIF 的别名。",
        link: MEMO,
    },
    HeaderDoc {
        command: "RONDAM",
        title: "#RONDAM",
        summary: "#RANDOM 的常见拼写变体。",
        link: MEMO,
    },
    HeaderDoc {
        command: "SWITCH",
        title: "#SWITCH",
        summary: "多 case 分支块：在 [1, N] 中取值。",
        link: MEMO,
    },
    HeaderDoc {
        command: "SETSWITCH",
        title: "#SETSWITCH",
        summary: "固定 switch 值，不调用 RNG。",
        link: MEMO,
    },
    HeaderDoc {
        command: "CASE",
        title: "#CASE",
        summary: "switch 的条件分支。",
        link: MEMO,
    },
    HeaderDoc {
        command: "SKIP",
        title: "#SKIP",
        summary: "结束 fall-through，跳到 #ENDSW。",
        link: MEMO,
    },
    HeaderDoc {
        command: "DEF",
        title: "#DEF",
        summary: "switch 的默认分支。",
        link: MEMO,
    },
    HeaderDoc {
        command: "ENDSW",
        title: "#ENDSW",
        summary: "结束 #SWITCH 块。",
        link: MEMO,
    },
    HeaderDoc {
        command: "ENDSWITCH",
        title: "#ENDSWITCH",
        summary: "#ENDSW 的别名。",
        link: MEMO,
    },
];

/// 通道表。
pub static CHANNEL_DOCS: &[ChannelDoc] = &[
    ChannelDoc {
        code: "01",
        title: "01 BGM",
        summary: "背景音乐通道；值引用 #WAVxx。",
        link: MEMO,
    },
    ChannelDoc {
        code: "02",
        title: "02 小节长度",
        summary: "小节长度倍率；1.0 为 4/4。",
        link: MEMO,
    },
    ChannelDoc {
        code: "03",
        title: "03 BPM",
        summary: "BPM 变更；十六进制整数。",
        link: MEMO,
    },
    ChannelDoc {
        code: "04",
        title: "04 BGA BASE",
        summary: "BGA 基础层；值引用 #BGAxx 或 #BMPxx。",
        link: MEMO,
    },
    ChannelDoc {
        code: "05",
        title: "05 SEEK",
        summary: "SEEK 对象；值引用 #SEEKxx。",
        link: MEMO,
    },
    ChannelDoc {
        code: "06",
        title: "06 BGA POOR",
        summary: "MISS 时显示的 BGA 层。",
        link: MEMO,
    },
    ChannelDoc {
        code: "07",
        title: "07 BGA LAYER",
        summary: "BGA 叠加层。",
        link: MEMO,
    },
    ChannelDoc {
        code: "08",
        title: "08 扩展 BPM",
        summary: "值引用 #BPMxx / #EXBPMxx。",
        link: MEMO,
    },
    ChannelDoc {
        code: "09",
        title: "09 STOP",
        summary: "值引用 #STOPxx。",
        link: MEMO,
    },
    ChannelDoc {
        code: "0A",
        title: "0A BGA LAYER2",
        summary: "第二层 BGA。",
        link: MEMO,
    },
    ChannelDoc {
        code: "0B",
        title: "0B BGA BASE 不透明度",
        summary: "BGA 基础层不透明度。",
        link: MEMO,
    },
    ChannelDoc {
        code: "0C",
        title: "0C BGA LAYER 不透明度",
        summary: "BGA 叠加层不透明度。",
        link: MEMO,
    },
    ChannelDoc {
        code: "0D",
        title: "0D BGA LAYER2 不透明度",
        summary: "第二层 BGA 不透明度。",
        link: MEMO,
    },
    ChannelDoc {
        code: "0E",
        title: "0E BGA POOR 不透明度",
        summary: "MISS 层不透明度。",
        link: MEMO,
    },
    ChannelDoc {
        code: "97",
        title: "97 BGM 音量",
        summary: "BGM 通道音量。",
        link: MEMO,
    },
    ChannelDoc {
        code: "98",
        title: "98 KEY 音量",
        summary: "按键音音量。",
        link: MEMO,
    },
    ChannelDoc {
        code: "99",
        title: "99 TEXT",
        summary: "值引用 #TEXTxx / #SONGxx。",
        link: MEMO,
    },
    ChannelDoc {
        code: "A0",
        title: "A0 JUDGE",
        summary: "判定覆盖；值为 base36 判定等级。",
        link: MEMO,
    },
    ChannelDoc {
        code: "A1",
        title: "A1 BGA ARGB BASE",
        summary: "BGA 基础层 ARGB。",
        link: MEMO,
    },
    ChannelDoc {
        code: "A2",
        title: "A2 BGA ARGB LAYER",
        summary: "BGA 叠加层 ARGB。",
        link: MEMO,
    },
    ChannelDoc {
        code: "A3",
        title: "A3 BGA ARGB LAYER2",
        summary: "第二层 BGA ARGB。",
        link: MEMO,
    },
    ChannelDoc {
        code: "A4",
        title: "A4 BGA ARGB POOR",
        summary: "MISS 层 ARGB。",
        link: MEMO,
    },
    ChannelDoc {
        code: "A5",
        title: "A5 BGA KEYBOUND",
        summary: "BGA 按键绑定。",
        link: MEMO,
    },
    ChannelDoc {
        code: "A6",
        title: "A6 OPTION",
        summary: "值引用 #CHANGEOPTIONxx。",
        link: MEMO,
    },
    ChannelDoc {
        code: "SC",
        title: "SC SCROLL",
        summary: "滚动速度倍率；值引用 #SCROLLxx。",
        link: MEMO,
    },
    ChannelDoc {
        code: "SP",
        title: "SP SPEED",
        summary: "视觉音符间距；值引用 #SPEEDxx。",
        link: MEMO,
    },
];

/// 查找头部命令说明。
#[must_use]
pub fn header_doc(command: &str) -> Option<&'static HeaderDoc> {
    let raw_command = command.trim_start_matches(['#', '%']);
    HEADER_DOCS
        .iter()
        .filter(|doc| {
            raw_command
                .get(..doc.command.len())
                .is_some_and(|prefix| prefix.eq_ignore_ascii_case(doc.command))
                && raw_command.get(doc.command.len()..).is_some_and(|suffix| {
                    suffix.is_empty()
                        || (suffix.len() <= 2
                            && suffix.chars().all(|ch| ch.is_ascii_alphanumeric()))
                })
        })
        .max_by_key(|doc| doc.command.len())
}

/// 返回全部头部命令说明。
#[must_use]
pub const fn header_completions() -> &'static [HeaderDoc] {
    HEADER_DOCS
}

/// 查找通道说明。
#[must_use]
pub fn channel_doc(code: &str) -> Option<ChannelDoc> {
    let normalized = code.trim().to_ascii_uppercase();
    if let Some(doc) = CHANNEL_DOCS
        .iter()
        .find(|doc| doc.code.eq_ignore_ascii_case(&normalized))
    {
        return Some(*doc);
    }
    note_channel_doc(&normalized)
}

/// 返回全部通道说明。
#[must_use]
pub const fn channel_completions() -> &'static [ChannelDoc] {
    CHANNEL_DOCS
}

/// 音符通道组说明。
const fn note_channel_doc(code: &str) -> Option<ChannelDoc> {
    let bytes = code.as_bytes();
    let &[first, second] = bytes else {
        return None;
    };
    if !second.is_ascii_alphanumeric() {
        return None;
    }
    let (title, summary) = match first {
        b'1' => ("1P 可见音符", "1P 可见音符；值引用 #WAVxx。"),
        b'2' => ("2P 可见音符", "2P 可见音符；值引用 #WAVxx。"),
        b'3' => ("1P 不可见音符", "1P 不可见音符；值引用 #WAVxx。"),
        b'4' => ("2P 不可见音符", "2P 不可见音符；值引用 #WAVxx。"),
        b'5' => ("1P 长音符", "1P 长音符；值引用 #WAVxx。"),
        b'6' => ("2P 长音符", "2P 长音符；值引用 #WAVxx。"),
        b'D' => ("1P 地雷", "1P 地雷；值引用 #WAVxx。"),
        b'E' => ("2P 地雷", "2P 地雷；值引用 #WAVxx。"),
        _ => return None,
    };
    Some(ChannelDoc {
        code: "1Z",
        title,
        summary,
        link: MEMO,
    })
}
