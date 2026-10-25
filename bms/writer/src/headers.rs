//! 头部命令的序列化。
//!
//! 从 [`Bms`](bms_parser::Bms) 的域字段重建 tokenizer 的域枚举值，
//! 复用 derive 生成的 `format_header` 完成命令名与值的格式化，
//! 保证与解析路径使用同一套格式模板。

use bms_parser::{Bms, OwnedExWavParams, OwnedSwBgaParams};
use bms_tokenizer::{
    ArgbParams, AtBgaParams, BgaParams, BmsHeaderDisplay, BmsHeaderGameplay, BmsHeaderMetadata,
    BmsHeaderResDefAudio, BmsHeaderResDefVisual, BmsHeaderTiming, ExBmpParams, ExWavParams,
    SwBgaParams, WavCmdParams,
};

/// 序列化全部头部命令行。
///
/// 输出顺序：元数据 → 游玩 → 计时 → 显示 → 音频 → 视觉 → 未识别头
/// （保留原顺序）→ `#STP`（按小节升序）。
pub fn write_headers(bms: &Bms) -> Vec<String> {
    let mut out = Vec::new();
    metadata(&bms.metadata, &mut out);
    gameplay(&bms.gameplay, &mut out);
    timing(&bms.timing, &mut out);
    display(&bms.display, &mut out);
    audio(&bms.audio, &mut out);
    visual(&bms.visual, &mut out);
    for (key, value) in &bms.fallback_headers {
        out.push(format!("#{key} {value}"));
    }
    stp(&bms.messages.stp_events, &mut out);
    out
}

/// 用 derive 生成的格式化模板输出一行头部命令。
///
/// `format_header` 是 derive 生成的固有方法（无 trait），故以宏分发。
macro_rules! push_header {
    ($header:expr, $out:expr) => {{
        let (command, value) = ($header).format_header();
        if value.is_empty() {
            $out.push(command);
        } else {
            $out.push(format!("{command} {value}"));
        }
    }};
}

/// 序列化元数据域。
fn metadata(m: &bms_parser::Metadata, out: &mut Vec<String>) {
    let headers = [
        m.title.clone().map(BmsHeaderMetadata::Title),
        m.subtitle.clone().map(BmsHeaderMetadata::Subtitle),
        m.artist.clone().map(BmsHeaderMetadata::Artist),
        m.sub_artist.clone().map(BmsHeaderMetadata::SubArtist),
        m.genre.clone().map(BmsHeaderMetadata::Genre),
        m.maker.clone().map(BmsHeaderMetadata::Maker),
        m.comment.clone().map(BmsHeaderMetadata::Comment),
        m.charset.clone().map(BmsHeaderMetadata::Charset),
        m.url.clone().map(BmsHeaderMetadata::Url),
        m.email.clone().map(BmsHeaderMetadata::Email),
    ];
    for h in headers.into_iter().flatten() {
        push_header!(h, out);
    }
    for (id, value) in &m.text_defs {
        push_header!(
            BmsHeaderMetadata::Text {
                id: *id,
                value: value.clone(),
            },
            out
        );
    }
}

/// 序列化游玩域。
fn gameplay(g: &bms_parser::Gameplay, out: &mut Vec<String>) {
    let scalars = [
        g.player.map(BmsHeaderGameplay::Player),
        g.rank.map(BmsHeaderGameplay::Rank),
        g.def_ex_rank.map(BmsHeaderGameplay::DefExRank),
        g.total.map(BmsHeaderGameplay::Total),
        g.vol_wav.map(BmsHeaderGameplay::VolWav),
        g.ln_type.map(BmsHeaderGameplay::LnType),
        g.ln_obj.map(BmsHeaderGameplay::LnObj),
        g.ln_mode.map(BmsHeaderGameplay::LnMode),
        g.base.map(BmsHeaderGameplay::Base),
        g.option.clone().map(BmsHeaderGameplay::Option),
    ];
    for h in scalars.into_iter().flatten() {
        push_header!(h, out);
    }
    if g.oct_fp == Some(true) {
        push_header!(BmsHeaderGameplay::OctFp, out);
    }
    for (id, value) in &g.ex_rank_defs {
        push_header!(
            BmsHeaderGameplay::ExRank {
                id: *id,
                value: *value,
            },
            out
        );
    }
    for (id, value) in &g.change_option_defs {
        push_header!(
            BmsHeaderGameplay::ChangeOption {
                id: *id,
                value: value.clone(),
            },
            out
        );
    }
}

/// 序列化计时域。
fn timing(t: &bms_parser::Timing, out: &mut Vec<String>) {
    let scalars = [
        t.bpm.map(BmsHeaderTiming::Bpm),
        t.base_bpm.map(BmsHeaderTiming::BaseBpm),
    ];
    for h in scalars.into_iter().flatten() {
        push_header!(h, out);
    }
    for (id, value) in &t.bpm_defs {
        push_header!(
            BmsHeaderTiming::BpmDef {
                id: *id,
                value: *value,
            },
            out
        );
    }
    for (id, value) in &t.stop_defs {
        push_header!(
            BmsHeaderTiming::StopDef {
                id: *id,
                value: *value,
            },
            out
        );
    }
    for (id, value) in &t.scroll_defs {
        push_header!(
            BmsHeaderTiming::ScrollDef {
                id: *id,
                value: *value,
            },
            out
        );
    }
    for (id, value) in &t.speed_defs {
        push_header!(
            BmsHeaderTiming::SpeedDef {
                id: *id,
                value: *value,
            },
            out
        );
    }
}

/// 序列化显示域。
fn display(d: &bms_parser::Display, out: &mut Vec<String>) {
    let headers = [
        d.stage_file.clone().map(BmsHeaderDisplay::StageFile),
        d.banner.clone().map(BmsHeaderDisplay::Banner),
        d.back_bmp.clone().map(BmsHeaderDisplay::BackBmp),
        d.char_file.clone().map(BmsHeaderDisplay::CharFile),
        d.play_level.map(BmsHeaderDisplay::PlayLevel),
        d.difficulty.map(BmsHeaderDisplay::Difficulty),
        d.preview.clone().map(BmsHeaderDisplay::Preview),
    ];
    for h in headers.into_iter().flatten() {
        push_header!(h, out);
    }
}

/// 序列化音频域。
fn audio(a: &bms_parser::Audio, out: &mut Vec<String>) {
    for (id, filename) in &a.wav_files {
        push_header!(
            BmsHeaderResDefAudio::Wav {
                id: *id,
                filename: filename.clone(),
            },
            out
        );
    }
    for (id, params) in &a.ex_wav_params {
        push_header!(
            BmsHeaderResDefAudio::ExWav {
                id: *id,
                params: ex_wav_params(params),
            },
            out
        );
    }
    if let Some(cmd) = &a.wav_cmd {
        push_header!(
            BmsHeaderResDefAudio::WavCmd {
                params: WavCmdParams {
                    command: cmd.command,
                    wav_index: cmd.wav_index,
                    value: cmd.value,
                },
            },
            out
        );
    }
    let scalars = [
        a.cdda.clone().map(BmsHeaderResDefAudio::Cdda),
        a.midifile.clone().map(BmsHeaderResDefAudio::Midifile),
        a.path_wav.clone().map(BmsHeaderResDefAudio::PathWav),
    ];
    for h in scalars.into_iter().flatten() {
        push_header!(h, out);
    }
}

/// 序列化视觉域。
fn visual(v: &bms_parser::Visual, out: &mut Vec<String>) {
    for (id, filename) in &v.bmp_files {
        push_header!(
            BmsHeaderResDefVisual::Bmp {
                id: *id,
                filename: filename.clone(),
            },
            out
        );
    }
    for (id, params) in &v.ex_bmp_defs {
        push_header!(
            BmsHeaderResDefVisual::ExBmp {
                id: *id,
                params: ExBmpParams {
                    a: params.a,
                    r: params.r,
                    g: params.g,
                    b: params.b,
                    filename: params.filename.clone(),
                },
            },
            out
        );
    }
    for (id, params) in &v.crop_defs {
        push_header!(
            BmsHeaderResDefVisual::Bga {
                id: *id,
                params: BgaParams {
                    bmp_index: params.bmp_index,
                    x1: params.x1,
                    y1: params.y1,
                    x2: params.x2,
                    y2: params.y2,
                    dx: params.dx,
                    dy: params.dy,
                },
            },
            out
        );
    }
    for (id, params) in &v.alt_crop_defs {
        push_header!(
            BmsHeaderResDefVisual::AtBga {
                id: *id,
                params: AtBgaParams {
                    bmp_index: params.bmp_index,
                    sx: params.sx,
                    sy: params.sy,
                    w: params.w,
                    h: params.h,
                    dx: params.dx,
                    dy: params.dy,
                },
            },
            out
        );
    }
    for (id, params) in &v.sw_bga_defs {
        push_header!(
            BmsHeaderResDefVisual::SwBga {
                id: *id,
                params: sw_bga_params(params),
            },
            out
        );
    }
    for (id, params) in &v.argb_defs {
        push_header!(
            BmsHeaderResDefVisual::Argb {
                id: *id,
                params: ArgbParams {
                    a: params.a,
                    r: params.r,
                    g: params.g,
                    b: params.b,
                },
            },
            out
        );
    }
    for (id, value) in &v.seek_defs {
        push_header!(
            BmsHeaderResDefVisual::Seek {
                id: *id,
                value: *value,
            },
            out
        );
    }
    video_scalars(v, out);
}

/// 序列化视频相关标量字段。
fn video_scalars(v: &bms_parser::Visual, out: &mut Vec<String>) {
    let scalars = [
        v.video_file.clone().map(BmsHeaderResDefVisual::VideoFile),
        v.movie.clone().map(BmsHeaderResDefVisual::Movie),
        v.ext_chr.clone().map(BmsHeaderResDefVisual::ExtChr),
        v.video_fps.map(BmsHeaderResDefVisual::VideoFps),
        v.video_colors.map(BmsHeaderResDefVisual::VideoColors),
        v.video_dly.map(BmsHeaderResDefVisual::VideoDly),
        v.poor_bga_mode.map(BmsHeaderResDefVisual::PoorBga),
    ];
    for h in scalars.into_iter().flatten() {
        push_header!(h, out);
    }
}

/// 序列化 `#STP` 事件（按小节升序）。
fn stp(events: &[bms_parser::StpEvent], out: &mut Vec<String>) {
    for event in events {
        let measure = event.position.measure();
        let position = event.position.numer();
        let line = if position == 0 {
            format!("#STP {measure:03} {}", event.duration_ms)
        } else {
            format!("#STP {measure:03}.{position} {}", event.duration_ms)
        };
        out.push(line);
    }
}

/// `#EXWAV` 参数：parser 拥有变体 → tokenizer 复合参数。
fn ex_wav_params(p: &OwnedExWavParams) -> ExWavParams {
    ExWavParams {
        pan: p.pan,
        volume: p.volume,
        frequency: p.frequency,
        filename: p.filename.clone(),
    }
}

/// `#SWBGA` 参数：parser 拥有变体 → tokenizer 复合参数。
fn sw_bga_params(p: &OwnedSwBgaParams) -> SwBgaParams {
    SwBgaParams {
        fr: p.fr,
        time: p.time,
        line: p.line,
        r#loop: p.r#loop,
        a: p.a,
        r: p.r,
        g: p.g,
        b: p.b,
        pattern: p.pattern.clone(),
    }
}
