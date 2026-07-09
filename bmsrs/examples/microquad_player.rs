//! Microquad BMS/BMSON 谱面播放器
//!
//! 使用 macroquad 渲染、kira 播放音频的 BMS/BMSON 谱面预览器。
//! 支持 7+1k 键位布局。可通过命令行参数指定谱面文件路径。
//!
//! # 用法
//!
//! ```sh
//! cargo run --example microquad_player -- path/to/chart.bms
//! cargo run --example microquad_player -- path/to/chart.bmson
//! ```

// 示例的职责就是向 stdout/stderr 输出；工作区内禁用
// `println!` / `eprintln!` 的规则在此不适用。
#![expect(
    clippy::print_stdout,
    reason = "example prints status to stdout by design"
)]
#![expect(
    clippy::print_stderr,
    reason = "example prints diagnostics to stderr by design"
)]
// 示例中大量存在 f64→f32、usize→f32 等数值转换，预期范围内安全。
#![expect(
    clippy::cast_precision_loss,
    reason = "u64/f64/usize→f32 casts are within safe range for display values"
)]
// 窗口尺寸等 f32→i32 截断在设计上是可接受的（像素取整）。
#![expect(
    clippy::cast_possible_truncation,
    reason = "f32→i32 truncation for pixel values is acceptable"
)]
// macroquad::main 宏会消耗其下方函数的文档注释，导致
// missing_docs_in_private_items 误报。此处对整例统一放行。
#![expect(
    clippy::missing_docs_in_private_items,
    reason = "macroquad::main strips doc comments; other items are documented"
)]

use std::collections::HashMap;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use bmsrs::bms::parser::Bms;
use bmsrs::bms::processor::BmsProcessor;
use bmsrs::bms::processor::custom_event::BmsCustomEvent;
use bmsrs::bms::tokenizer::BmsTokenizer;
use bmsrs::bmson::de::BmsonParser;
use bmsrs::bmson::processor::{BmsonNoteExt, BmsonProcessor};
use bmsrs::chart::{AudioAsset, Chart, Event, EventKind, Lane, NoCustomEvent, NoteKind, NoteSide};
use bmsrs::player::Player;
use clap::Parser;
use kira::sound::static_sound::StaticSoundData;
use kira::{AudioManager, AudioManagerSettings, Capacities, DefaultBackend};
use macroquad::prelude::*;

// 常量

/// 屏幕宽度。
const SCREEN_WIDTH: f32 = 800.0;

/// 屏幕高度。
const SCREEN_HEIGHT: f32 = 600.0;

/// 轨道数量（7 个常规按键 + 1 个搓碟转盘）。
const TRACK_COUNT: usize = 8;

/// 每条轨道的渲染宽度（像素）。
const TRACK_WIDTH: f32 = 55.0;

/// 轨道间距（像素）。
const TRACK_SPACING: f32 = 4.0;

/// 全部轨道占用的总宽度。
const TOTAL_TRACKS_WIDTH: f32 = TRACK_COUNT as f32 * (TRACK_WIDTH + TRACK_SPACING);

/// 判定线纵坐标。
const JUDGMENT_LINE_Y: f32 = 500.0;

/// 反应时间（毫秒）—— 判定线下方可见的历史时长。
const REACTION_TIME_MS: u64 = 550;

/// 预读时间（毫秒）—— 判定线上方可预见的未来时长。
const LOOKAHEAD_TIME_MS: u64 = 1650;

/// 每拍像素数 —— 控制音符滚动速度。
const PIXELS_PER_BEAT: f32 = 120.0;

// 配色

/// 背景色。
const COLOR_BG: Color = Color::new(0.12, 0.12, 0.12, 1.0);

/// 轨道边线色。
const COLOR_TRACK_LINE: Color = Color::new(0.4, 0.4, 0.4, 1.0);

/// 轨道背景色。
const COLOR_TRACK_BG: Color = Color::new(0.18, 0.18, 0.18, 1.0);

/// 判定线色。
const COLOR_JUDGMENT_LINE: Color = Color::new(1.0, 1.0, 1.0, 1.0);

/// 小节线色。
const COLOR_BAR_LINE: Color = Color::new(0.3, 0.3, 0.3, 0.7);

/// 白色音符色（奇数轨道）。
const COLOR_NOTE_WHITE: Color = Color::new(1.0, 1.0, 1.0, 1.0);

/// 蓝色音符色（偶数轨道）。
const COLOR_NOTE_BLUE: Color = Color::new(0.4, 0.6, 0.9, 1.0);

/// 搓碟转盘音符色（轨道 0）。
const COLOR_NOTE_SCRATCH: Color = Color::new(1.0, 0.2, 0.2, 1.0);

/// 地雷音符色。
const COLOR_NOTE_MINE: Color = Color::new(1.0, 1.0, 0.0, 1.0);

/// 不可见音符色。
const COLOR_NOTE_INVISIBLE: Color = Color::new(0.6, 0.6, 0.6, 0.5);

/// 信息文字色。
const COLOR_INFO_TEXT: Color = Color::new(1.0, 1.0, 1.0, 1.0);

/// 提示文字色。
const COLOR_HINT_TEXT: Color = Color::new(0.5, 0.5, 0.5, 1.0);

// 窗口配置

/// 返回 macroquad 窗口配置。
fn window_conf() -> Conf {
    Conf {
        window_title: "bmsrs — Microquad Player".to_owned(),
        window_width: SCREEN_WIDTH as i32,
        window_height: SCREEN_HEIGHT as i32,
        platform: miniquad::conf::Platform {
            linux_backend: miniquad::conf::LinuxBackend::WaylandWithX11Fallback,
            ..Default::default()
        },
        ..Default::default()
    }
}

// 命令行参数

/// BMS/BMSON 谱面播放器。
#[derive(Parser, Debug)]
#[command(name = "microquad_player")]
#[command(about = "A simple BMS/BMSON chart player", long_about = None)]
struct Config {
    /// 谱面文件路径（.bms / .bme / .bml / .pms / .bmson）。
    #[arg(value_name = "FILE")]
    chart_path: PathBuf,
}

// 谱面加载

/// 加载并解析谱面文件，返回 `Chart<(), NoCustomEvent>` 与谱面所在目录。
///
/// 自动根据文件扩展名选择解析路径（BMS 或 BMSON）。
fn load_chart(path: &Path) -> Result<(Chart<(), NoCustomEvent>, PathBuf), String> {
    let extension = path
        .extension()
        .and_then(|e| e.to_str())
        .ok_or_else(|| format!("无法识别文件扩展名: {}", path.display()))?;

    let base_path = path
        .parent()
        .map_or_else(|| PathBuf::from("."), Path::to_path_buf);

    match extension.to_lowercase().as_str() {
        "bms" | "bme" | "bml" | "pms" => {
            let chart = normalize_bms_chart(load_bms(path)?);
            Ok((chart, base_path))
        }
        "bmson" => {
            let chart = load_bmson(path)?;
            Ok((chart, base_path))
        }
        ext => Err(format!("不支持的谱面格式: {ext}")),
    }
}

/// 加载 BMS 格式谱面（保留引擎特定自定义事件）。
fn load_bms(path: &Path) -> Result<Chart<(), BmsCustomEvent>, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("无法读取文件 {}: {e}", path.display()))?;

    // BMS 文件通常使用 Shift-JIS 编码。
    let content = encoding_rs::SHIFT_JIS.decode(&bytes).0.into_owned();

    // 分词后直接送入解析器，避免不必要的中间收集。
    let bms = Bms::from_flat_tokens(
        BmsTokenizer::new()
            .tokenize_owned(&content)
            .into_iter()
            .filter_map(|(_, res)| match res {
                Ok(tok) => Some(tok),
                Err(e) => {
                    eprintln!("警告: 跳过无效 token: {e}");
                    None
                }
            }),
    );

    // 转换为格式无关的 Chart。
    BmsProcessor::process_default(&bms).map_err(|e| format!("BMS 处理失败: {e}"))
}

/// 加载 BMSON 格式谱面。
fn load_bmson(path: &Path) -> Result<Chart<(), NoCustomEvent>, String> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("无法读取文件 {}: {e}", path.display()))?;

    // 使用 bmson-de-chumsky 解析（支持 v0/v1/v2）。
    let bmson = BmsonParser::parse(&content).map_err(|e| format!("BMSON 解析失败: {e}"))?;

    // 转换为 Chart<BmsonNoteExt>。
    let chart =
        BmsonProcessor::process_default(&bmson).map_err(|e| format!("BMSON 处理失败: {e}"))?;

    // 剥离扩展数据，统一为 Chart<()>。
    Ok(normalize_chart(chart))
}

/// 将 `Chart<(), BmsCustomEvent>` 转换为 `Chart<(), NoCustomEvent>`，
/// 丢弃 BMS 引擎特定的自定义事件（渲染器无需处理它们）。
fn normalize_bms_chart(chart: Chart<(), BmsCustomEvent>) -> Chart<(), NoCustomEvent> {
    chart.filter_map_events(|e| {
        let tick = e.tick();
        match e.kind {
            EventKind::Custom(_) => None,
            kind => Some(Event::new(tick, kind.map_custom(|_| NoCustomEvent))),
        }
    })
}

/// 将 `Chart<BmsonNoteExt>` 转换为 `Chart<(), NoCustomEvent>`。
fn normalize_chart(chart: Chart<BmsonNoteExt, NoCustomEvent>) -> Chart<(), NoCustomEvent> {
    chart.map_events(|e| Event::new(e.tick(), e.kind.map_ext(|_| ())))
}

// 音频加载

/// 尝试以多种扩展名查找音频文件。
fn find_audio_with_extensions(path: &Path, extensions: &[&str]) -> Option<PathBuf> {
    let stem = path.with_extension("");
    for ext in extensions {
        let candidate = stem.with_extension(ext);
        if candidate.exists() {
            return Some(candidate);
        }
    }
    None
}

/// 预加载全部音频素材。
fn preload_audio(audio_assets: &[AudioAsset], base_path: &Path) -> HashMap<u32, StaticSoundData> {
    let mut map = HashMap::new();
    for (i, asset) in audio_assets.iter().enumerate() {
        let full_path = base_path.join(asset.path.as_ref());
        let found = find_audio_with_extensions(&full_path, &["ogg", "flac", "wav", "mp3"]);
        let Some(path) = found else {
            eprintln!("警告: 未找到音频文件: {}", full_path.display());
            continue;
        };
        match StaticSoundData::from_file(&path) {
            Ok(data) => {
                #[expect(clippy::cast_possible_truncation, reason = "asset count fits in u32")]
                map.insert(i as u32, data);
            }
            Err(e) => {
                eprintln!("警告: 无法加载音频 {}: {e}", path.display());
            }
        }
    }
    map
}

// 轨道映射

/// 将 `Lane` 映射为轨道索引（0–7）。
///
/// 0 = Scratch, 1–7 = Key1–Key7
#[must_use]
const fn lane_to_index(lane: Lane) -> Option<usize> {
    match lane {
        Lane::Scratch(n) if n.get() == 1 => Some(0),
        Lane::Key(n) => {
            let i = n.get() as usize;
            if i >= 1 && i <= 7 { Some(i) } else { None }
        }
        _ => None,
    }
}

/// 计算轨道的屏幕 X 坐标。
#[must_use]
fn track_x(index: usize) -> f32 {
    let start_x = (SCREEN_WIDTH - TOTAL_TRACKS_WIDTH) / 2.0;
    (index as f32).mul_add(TRACK_WIDTH + TRACK_SPACING, start_x)
}

// 音符颜色

/// 根据轨道索引返回音符颜色。
#[must_use]
const fn note_color_for_track(index: usize) -> Color {
    match index {
        0 => COLOR_NOTE_SCRATCH,
        i if i % 2 == 1 => COLOR_NOTE_WHITE,
        _ => COLOR_NOTE_BLUE,
    }
}

// 渲染

/// 绘制轨道背景与判定线。
fn render_tracks() {
    for i in 0..TRACK_COUNT {
        let x = track_x(i);
        // 轨道背景。
        draw_rectangle(x, 0.0, TRACK_WIDTH, SCREEN_HEIGHT, COLOR_TRACK_BG);
        // 轨道边框。
        draw_line(x, 0.0, x, SCREEN_HEIGHT, 1.0, COLOR_TRACK_LINE);
        draw_line(
            x + TRACK_WIDTH,
            0.0,
            x + TRACK_WIDTH,
            SCREEN_HEIGHT,
            1.0,
            COLOR_TRACK_LINE,
        );
    }

    // 判定线。
    let start_x = (SCREEN_WIDTH - TOTAL_TRACKS_WIDTH) / 2.0;
    draw_line(
        start_x - 10.0,
        JUDGMENT_LINE_Y,
        start_x + TOTAL_TRACKS_WIDTH + 10.0,
        JUDGMENT_LINE_Y,
        3.0,
        COLOR_JUDGMENT_LINE,
    );
}

/// 计算音符在屏幕上的纵坐标。
///
/// 基于滚动位置（由 SCROLL 事件影响）计算音符相对于判定线的视觉偏移。
#[must_use]
fn note_screen_y(scroll_diff_beats: f64, spacing: f64) -> f32 {
    JUDGMENT_LINE_Y - f32::mul_add(scroll_diff_beats as f32, PIXELS_PER_BEAT, -(spacing as f32))
}

/// 渲染单个小节线事件。
fn render_bar_line(y: f32) {
    if !(-10.0..=SCREEN_HEIGHT + 10.0).contains(&y) {
        return;
    }
    let start_x = (SCREEN_WIDTH - TOTAL_TRACKS_WIDTH) / 2.0;
    draw_line(
        start_x,
        y,
        start_x + TOTAL_TRACKS_WIDTH,
        y,
        1.0,
        COLOR_BAR_LINE,
    );
}

/// 渲染单个音符事件。
fn render_note(
    event: &Event<(), NoCustomEvent>,
    scroll_diff: f64,
    current_scroll_pos: f64,
    player: &Player<(), NoCustomEvent>,
) {
    let EventKind::Note {
        kind, lane, side, ..
    } = &event.kind
    else {
        return;
    };

    // 仅渲染 1P 侧的音符。
    if *side != NoteSide::P1 {
        return;
    }
    let Some(track_idx) = lane_to_index(*lane) else {
        return;
    };

    let spacing = player.spacing_at(event.tick());
    let y_start = note_screen_y(scroll_diff, spacing);
    if !(-50.0..=SCREEN_HEIGHT + 50.0).contains(&y_start) {
        return;
    }

    let x = track_x(track_idx);

    match kind {
        NoteKind::Normal => {
            draw_rectangle(
                x + 2.0,
                y_start - 8.0,
                TRACK_WIDTH - 4.0,
                8.0,
                note_color_for_track(track_idx),
            );
        }
        NoteKind::Long { duration } if *duration > 0 => {
            let ln_end_tick = event.tick() + duration;
            let end_scroll_pos = player.scroll_position_at(ln_end_tick);
            let end_diff = end_scroll_pos - current_scroll_pos;
            let y_end = note_screen_y(end_diff, spacing);

            // 长条身体（半透明）。
            let body_top = y_end.min(y_start);
            let body_bottom = y_end.max(y_start);
            let base_color = note_color_for_track(track_idx);
            let fade_color =
                Color::new(base_color.r, base_color.g, base_color.b, base_color.a * 0.4);
            draw_rectangle(
                x + 2.0,
                body_top,
                TRACK_WIDTH - 4.0,
                body_bottom - body_top,
                fade_color,
            );

            // 长条头。
            draw_rectangle(x + 2.0, y_start - 8.0, TRACK_WIDTH - 4.0, 8.0, base_color);
        }
        NoteKind::Long { .. } => {
            // 空长音（duration == 0）作为普通音符渲染。
            draw_rectangle(
                x + 2.0,
                y_start - 8.0,
                TRACK_WIDTH - 4.0,
                8.0,
                note_color_for_track(track_idx),
            );
        }
        NoteKind::Mine { .. } => {
            // 地雷以圆形绘制。
            draw_circle(
                x + TRACK_WIDTH / 2.0,
                y_start,
                TRACK_WIDTH / 4.0,
                COLOR_NOTE_MINE,
            );
        }
        NoteKind::Invisible => {
            // 不可见音符以半透明小方块绘制。
            draw_rectangle(
                x + TRACK_WIDTH / 2.0 - 4.0,
                y_start - 4.0,
                8.0,
                8.0,
                COLOR_NOTE_INVISIBLE,
            );
        }
    }
}

/// 渲染可见音符与条形线。
fn render_notes(player: &Player<(), NoCustomEvent>) {
    let current_tick = player.current_tick();

    // 计算可见 tick 窗口。
    let (start_tick, end_tick) = player.visible_tick_range(
        Duration::from_millis(REACTION_TIME_MS),
        Duration::from_millis(LOOKAHEAD_TIME_MS),
    );

    // 获取当前滚动位置（以节拍为单位）。
    let current_scroll_pos = player.scroll_position_at(current_tick);

    // 获取可见范围内的事件。
    let events = player.events_in_range(start_tick..end_tick);

    for event in events {
        let note_tick = event.tick();

        // 计算滚动位置差（节拍）。
        let note_scroll_pos = player.scroll_position_at(note_tick);
        let scroll_diff = note_scroll_pos - current_scroll_pos;

        match &event.kind {
            EventKind::Bar => {
                let y = note_screen_y(scroll_diff, 1.0);
                render_bar_line(y);
            }
            EventKind::Note { .. } => {
                render_note(event, scroll_diff, current_scroll_pos, player);
            }
            _ => {}
        }
    }
}

/// 渲染信息文字。
fn render_info(player: &Player<(), NoCustomEvent>, elapsed: Duration) {
    let bpm = player.current_bpm();
    let tick = player.current_tick();
    let total_ticks = player.chart().data.last_tick();

    draw_text(format!("BPM: {bpm:.1}"), 10.0, 24.0, 20.0, COLOR_INFO_TEXT);

    draw_text(
        format!("Time: {:.1}s", elapsed.as_secs_f64()),
        10.0,
        50.0,
        20.0,
        COLOR_INFO_TEXT,
    );

    let pct = if total_ticks > 0 {
        ((tick as f64) / (total_ticks as f64) * 100.0).min(100.0)
    } else {
        0.0
    };
    draw_text(
        format!("Progress: {pct:.0}%"),
        10.0,
        76.0,
        20.0,
        COLOR_INFO_TEXT,
    );

    let title = &player.chart().song.title;
    if !title.is_empty() {
        let text_w = measure_text(title, None, 24, 1.0).width;
        draw_text(
            title,
            (SCREEN_WIDTH - text_w) / 2.0,
            30.0,
            24.0,
            COLOR_INFO_TEXT,
        );
    }

    draw_text(
        "7+1k Layout: S | 1 | 2 | 3 | 4 | 5 | 6 | 7",
        10.0,
        SCREEN_HEIGHT - 20.0,
        14.0,
        COLOR_HINT_TEXT,
    );

    draw_text(
        "Audio: Note/BGM events trigger sounds",
        10.0,
        SCREEN_HEIGHT - 40.0,
        14.0,
        COLOR_HINT_TEXT,
    );
}

// 音频处理

/// 处理新到达事件（播放音频）。
///
/// 使用每帧局部 [`HashSet`] 去重，同一 `(tick, audio_index)` 只播放一次。
/// 相比持久 `HashMap`，不会随播放进度无限增长内存。
fn process_audio_events(
    new_events: &[Event<(), NoCustomEvent>],
    audio_data: &HashMap<u32, StaticSoundData>,
    audio_manager: &mut AudioManager<DefaultBackend>,
    missed_sounds: &mut u32,
) {
    let mut played = HashSet::new();
    for event in new_events {
        let audio_idx = match &event.kind {
            EventKind::Note { audio_index, .. } => *audio_index,
            EventKind::Bgm { audio_index } => Some(*audio_index),
            _ => continue,
        };
        let Some(idx) = audio_idx else {
            continue;
        };

        // 去重：同一 (tick, audio_index) 只播放一次。
        if !played.insert((event.tick(), idx)) {
            continue;
        }

        // 播放音频。
        if let Some(data) = audio_data.get(&idx)
            && let Err(e) = audio_manager.play(data.clone())
        {
            if matches!(e, kira::PlaySoundError::SoundLimitReached) {
                *missed_sounds += 1;
            } else {
                eprintln!("音频播放失败: {e}");
            }
        }
    }
}

// 主函数

#[macroquad::main(window_conf)]
async fn main() {
    let config = Config::parse();

    if !config.chart_path.exists() {
        eprintln!("错误: 文件不存在: {}", config.chart_path.display());
        return;
    }

    println!("加载谱面: {}", config.chart_path.display());

    let (chart, base_path) = match load_chart(&config.chart_path) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("错误: 谱面加载失败: {e}");
            return;
        }
    };
    println!("谱面已加载: {} — {}", chart.song.title, chart.song.artist);

    let mut player = Player::new(chart);

    // 预加载音频。
    let audio_assets = player.audio_assets().to_vec();
    println!("加载音频文件... (共 {} 个素材)", audio_assets.len());
    let audio_data = preload_audio(&audio_assets, &base_path);
    println!("音频加载完成: {} 个文件已加载", audio_data.len());

    // 初始化音频系统。
    let mut audio_manager = match AudioManager::<DefaultBackend>::new(AudioManagerSettings {
        capacities: Capacities {
            sub_track_capacity: 512,
            send_track_capacity: 16,
            clock_capacity: 8,
            modulator_capacity: 16,
            listener_capacity: 8,
        },
        internal_buffer_size: 256,
        ..Default::default()
    }) {
        Ok(am) => am,
        Err(e) => {
            eprintln!("错误: 音频系统初始化失败: {e}");
            return;
        }
    };
    println!("音频系统已初始化");

    // 播放状态跟踪。
    let start_time = Instant::now();
    let mut last_tick: u64 = 0;
    let mut missed_sounds: u32 = 0;
    let mut next_status_time = Duration::ZERO;

    println!("开始播放...");

    loop {
        let elapsed = start_time.elapsed();

        // 更新播放器位置。
        player.seek(elapsed);
        let current_tick = player.current_tick();

        // 处理新到达的事件（音频触发）。
        if current_tick > last_tick {
            let new_events = player.events_in_range(last_tick..current_tick);
            process_audio_events(
                new_events,
                &audio_data,
                &mut audio_manager,
                &mut missed_sounds,
            );
        }
        last_tick = current_tick;

        // 每秒打印一次状态。
        if elapsed >= next_status_time {
            println!(
                "[播放] 时间: {:.1}s | 脉冲: {} | BPM: {:.1} | 遗漏音频: {}",
                elapsed.as_secs_f64(),
                current_tick,
                player.current_bpm(),
                missed_sounds,
            );
            next_status_time = elapsed + Duration::from_secs(1);
        }

        // 渲染。
        clear_background(COLOR_BG);
        render_tracks();
        render_notes(&player);
        render_info(&player, elapsed);

        // 检查播放是否结束。
        if current_tick >= player.chart().data.last_tick() {
            draw_text(
                "♪ 播放完毕 ♪",
                SCREEN_WIDTH / 2.0 - 80.0,
                SCREEN_HEIGHT / 2.0,
                36.0,
                Color::new(1.0, 0.8, 0.2, 1.0),
            );
        }

        next_frame().await;
    }
}
