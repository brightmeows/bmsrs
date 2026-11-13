//! Visual-resource conversion steps: BMP/BGA, SWBGA, and background video.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::time::Duration;

use bmsrs_bms_model::{BgaEvent, BmsBgaLayer, Position};
use bmsrs_bms_values::{BmpIndex, BmsBase, BmsChannel as RawChannel, BmsIndex};
use bmsrs_chart::{
    BgaFrame, BgaResourceContent, BgaResourceId, CropRect, Event, EventKind, SwBgaResource,
    SwBgaResourceId, VideoAsset,
};

use super::{BmsConverter, ProcessWarning, ProcessWarningKind};

impl BmsConverter<'_> {
    /// Build BGA events from BGA events and BMP file definitions.
    ///
    /// The poor layer carries the miss-animation channel (`#xxx06`,
    /// jbms-parser semantics): a measure's line with more than one distinct
    /// object defines a 500 ms miss animation referenced once at the measure
    /// line, while single-object lines stay timed switches.
    pub fn collect_bga(&mut self, bmp_map: &BTreeMap<BmpIndex, BmpEntry>) {
        let base = self.bms.gameplay.base.unwrap_or_default();
        let animated = self.build_poor_animations(bmp_map, base);
        let mut emitted_measures = std::collections::BTreeSet::new();
        for be in &self.bms.messages.bga_events {
            if be.layer == BmsBgaLayer::Poor
                && let Some(&resource_id) = animated.get(&be.position.measure())
            {
                let measure = be.position.measure();
                if emitted_measures.insert(measure) {
                    let tick = self.table.position_to_tick(Position::new(measure, 0, 1));
                    self.events.push(Event::new(
                        tick,
                        EventKind::Bga {
                            layer: map_bga_layer(BmsBgaLayer::Poor),
                            resource_id: Some(resource_id),
                        },
                    ));
                }
                continue;
            }
            let tick = self.table.position_to_tick(be.position);
            let resource_id = if let Some(entry) = bmp_map.get(&be.bmp_id) {
                Some(entry.resource_id)
            } else {
                // Referencing an undefined BMP index clears the layer
                // (mainstream engines render it as nothing).
                let channel = match be.layer {
                    BmsBgaLayer::Base => RawChannel::BgaBase,
                    BmsBgaLayer::Poor => RawChannel::BgaPoor,
                    BmsBgaLayer::Layer => RawChannel::BgaLayer,
                    BmsBgaLayer::Layer2 => RawChannel::BgaLayer2,
                };
                let span = self.message_span(be.position.measure(), &[channel]);
                if let Some(key) = be.bmp_id.to_index_in(base) {
                    self.warnings.push(ProcessWarning::new(
                        span,
                        ProcessWarningKind::MissingBmpDefinition { key, tick },
                    ));
                }
                None
            };
            self.events.push(Event::new(
                tick,
                EventKind::Bga {
                    layer: map_bga_layer(be.layer),
                    resource_id,
                },
            ));
        }
    }

    /// Build miss-animation sequence resources for multi-object poor-layer
    /// lines.
    ///
    /// Returns the animated measures mapped to their sequence resource ids.
    /// Each distinct object member plays `500 ms / object count`, matching
    /// jbms-parser. Objects without a `#BMP` definition become blank frames
    /// and warn.
    fn build_poor_animations(
        &mut self,
        bmp_map: &BTreeMap<BmpIndex, BmpEntry>,
        base: BmsBase,
    ) -> BTreeMap<u16, BgaResourceId> {
        let mut by_measure: BTreeMap<u16, Vec<&BgaEvent>> = BTreeMap::new();
        for event in &self.bms.messages.bga_events {
            if event.layer == BmsBgaLayer::Poor {
                by_measure
                    .entry(event.position.measure())
                    .or_default()
                    .push(event);
            }
        }

        let mut animated = BTreeMap::new();
        for (measure, events) in &by_measure {
            let mut distinct = BTreeSet::new();
            for event in events {
                distinct.insert(event.bmp_id);
            }
            if distinct.len() <= 1 {
                continue;
            }

            #[expect(
                clippy::cast_possible_truncation,
                reason = "a single channel line never carries more objects than fit u32"
            )]
            let frame_duration = Duration::from_millis(500) / events.len() as u32;
            let mut frames = Vec::with_capacity(events.len());
            for event in events {
                let resource = bmp_map.get(&event.bmp_id).map(|entry| entry.resource_id);
                if resource.is_none()
                    && let Some(key) = event.bmp_id.to_index_in(base)
                {
                    let tick = self.table.position_to_tick(event.position);
                    let span = self.message_span(*measure, &[RawChannel::BgaPoor]);
                    self.warnings.push(ProcessWarning::new(
                        span,
                        ProcessWarningKind::MissingBmpDefinition { key, tick },
                    ));
                }
                frames.push(BgaFrame {
                    resource,
                    duration: frame_duration,
                });
            }

            #[expect(
                clippy::cast_possible_truncation,
                reason = "BGA resource counts fit u32"
            )]
            let resource_id =
                BgaResourceId::new((bmp_map.len() + self.extra_bga_contents.len()) as u32);
            self.extra_bga_contents
                .push(BgaResourceContent::Sequence { frames });
            animated.insert(*measure, resource_id);
        }
        animated
    }

    /// Build a mapping from BMP indices to [`BmpEntry`] (dense id, path, and
    /// optional crop).
    ///
    /// Merges keys from the `#BMP`, `#BGA`, and `#@BGA` namespaces:
    ///
    /// - When an id is defined in `#BGA` / `#@BGA`, `#BGA` wins per the BMS
    ///   specification: the path comes from the crop definition's source BMP
    ///   (resolved through the decimal `bmp_index`), with a [`CropRect`]
    ///   attached.
    /// - Otherwise the path comes from `#BMP` itself and the crop is `None`.
    /// - Crop ids whose source path cannot be resolved are skipped (they enter
    ///   neither the resource table nor the event stream).
    ///
    /// `#@BGA` (width/height form) is normalized to `#BGA` (bottom-right-corner
    /// form): `x2 = sx + w` and `y2 = sy + h`.
    #[expect(clippy::cast_possible_truncation, reason = "BMP count fits in u32")]
    pub fn build_bmp_map(&self) -> BTreeMap<BmpIndex, BmpEntry> {
        let visual = &self.bms.visual;
        let mut map: BTreeMap<BmpIndex, BmpEntry> = BTreeMap::new();

        // Collect every involved id (bmp_files, crop_defs, alt_crop_defs).
        let mut ids: BTreeSet<BmpIndex> = visual.bmp_files.keys().copied().collect();
        ids.extend(visual.crop_defs.keys().copied());
        ids.extend(visual.alt_crop_defs.keys().copied());

        for (i, id) in ids.into_iter().enumerate() {
            // Crop definition: #BGA takes priority over #@BGA (the same id is
            // unlikely to appear in both). Both give (CropRect, source
            // bmp_index); #@BGA's w/h is normalized to the bottom-right corner.
            let crop_def = visual
                .crop_defs
                .get(&id)
                .map(|p| {
                    (
                        CropRect {
                            x1: p.x1,
                            y1: p.y1,
                            x2: p.x2,
                            y2: p.y2,
                            dx: p.dx,
                            dy: p.dy,
                        },
                        p.bmp_index,
                    )
                })
                .or_else(|| {
                    visual.alt_crop_defs.get(&id).map(|p| {
                        (
                            CropRect {
                                x1: p.sx,
                                y1: p.sy,
                                x2: p.sx + p.w,
                                y2: p.sy + p.h,
                                dx: p.dx,
                                dy: p.dy,
                            },
                            p.bmp_index,
                        )
                    })
                });

            // Path: with a crop definition, use the source BMP (#BGA first);
            // otherwise use #BMP itself.
            let (crop, path_opt) = match crop_def {
                Some((crop, source_index)) => (
                    Some(crop),
                    resolve_bmp_path(&visual.bmp_files, source_index).cloned(),
                ),
                None => (None, visual.bmp_files.get(&id).cloned()),
            };

            let Some(path) = path_opt else {
                continue;
            };

            map.insert(
                id,
                BmpEntry {
                    resource_id: BgaResourceId::new(i as u32),
                    path,
                    crop,
                },
            );
        }

        map
    }

    /// Map each `#SWBGAxx` definition key to its vector index in
    /// `swbga_resources`. Keys iterate in sorted order, matching the numbering
    /// in [`Self::build_swbga_resources`].
    #[expect(clippy::cast_possible_truncation, reason = "SWBGA count fits in u32")]
    pub fn build_swbga_map(&self) -> BTreeMap<BmpIndex, SwBgaResourceId> {
        self.bms
            .visual
            .sw_bga_defs
            .keys()
            .copied()
            .enumerate()
            .map(|(i, id)| (id, SwBgaResourceId::new(i as u32)))
            .collect()
    }

    /// Build SWBGA animation definitions from `#SWBGAxx`.
    ///
    /// Pattern elements are two-digit indices decoded through the chart's base
    /// and matched against `bmp_map` by numeric value; `00` resolves to the
    /// `#BMP00` image when defined, otherwise the frame is blank.
    pub fn build_swbga_resources(
        &self,
        bmp_map: &BTreeMap<BmpIndex, BmpEntry>,
    ) -> Vec<SwBgaResource> {
        let base = self.bms.gameplay.base.unwrap_or_default();
        self.bms
            .visual
            .sw_bga_defs
            .values()
            .map(|def| {
                let pattern = def
                    .pattern
                    .as_bytes()
                    .as_chunks::<2>()
                    .0
                    .iter()
                    .map(|pair| {
                        let text = std::str::from_utf8(pair).ok()?;
                        let index = BmsIndex::try_from(text).ok()?.normalize(base);
                        bmp_map.get(&BmpIndex(index)).map(|entry| entry.resource_id)
                    })
                    .collect();
                SwBgaResource {
                    frame_ms: def.fr,
                    total_time_ms: def.time,
                    line: def.line,
                    loop_playback: def.r#loop,
                    chroma: [def.a, def.r, def.g, def.b],
                    pattern,
                }
            })
            .collect()
    }

    /// Build a [`VideoAsset`] from `#VIDEOFILE` / `#MOVIE` and video parameters.
    ///
    /// `#VIDEOFILE` (looping) takes priority over `#MOVIE` (single play). Returns
    /// `None` when neither is present. `#VIDEOf/s`, `#VIDEOCOLORS`, and
    /// `#VIDEODLY` are attached as optional parameters; `#VIDEOCOLORS` and
    /// `#VIDEODLY` are semantically integers and are converted from `f64` to
    /// `u32`.
    #[expect(clippy::cast_possible_truncation, reason = "video params fit in u32")]
    #[expect(clippy::cast_sign_loss, reason = "clamped to non-negative before cast")]
    pub fn build_video_asset(&self) -> Option<VideoAsset> {
        let visual = &self.bms.visual;
        let (path, loop_playback) = if let Some(p) = &visual.video_file {
            (p.clone(), true)
        } else {
            (visual.movie.clone()?, false)
        };
        let mut asset = VideoAsset::new(PathBuf::from(path), loop_playback);
        if let Some(fps) = visual.video_fps {
            asset = asset.with_fps(fps);
        }
        if let Some(colors) = visual.video_colors {
            asset = asset.with_colors(colors.max(0.0).round() as u32);
        }
        if let Some(delay) = visual.video_dly {
            asset = asset.with_delay_frames(delay.max(0.0).round() as u32);
        }
        Some(asset)
    }
}

/// Map the BMS semantic layer's layer enum to the format-agnostic
/// [`bmsrs_chart::BgaLayer`].
const fn map_bga_layer(layer: BmsBgaLayer) -> bmsrs_chart::BgaLayer {
    match layer {
        BmsBgaLayer::Base => bmsrs_chart::BgaLayer::Base,
        BmsBgaLayer::Poor => bmsrs_chart::BgaLayer::Poor,
        BmsBgaLayer::Layer => bmsrs_chart::BgaLayer::Layer,
        BmsBgaLayer::Layer2 => bmsrs_chart::BgaLayer::Layer2,
    }
}

/// Entry produced by [`build_bmp_map`](BmsConverter::build_bmp_map): dense
/// resource id, source file path, and optional crop definition.
pub struct BmpEntry {
    /// Zero-based resource id (the vector index into `bga_resources`).
    pub resource_id: BgaResourceId,
    /// Source image file path (the underlying `#BMP` that a crop definition
    /// points to).
    pub path: String,
    /// Crop and placement definition (`#BGA` / `#@BGA`); `None` when absent.
    pub crop: Option<CropRect>,
}

/// Resolve a decimal BMP number into a file path from the `#BMP` table.
///
/// The `bmp_index` field of `#BGA` / `#@BGA` is the **decimal** number of the
/// source `#BMP`: per the BMS command memo the value is the hexadecimal index
/// converted to decimal (so `16` references `#BMP10`), and only indices
/// `00`-`FF` (0-255) can be referenced. Values above 255 are invalid and
/// resolve to nothing.
fn resolve_bmp_path(bmp_files: &BTreeMap<BmpIndex, String>, bmp_index: u16) -> Option<&String> {
    let hex = u8::try_from(bmp_index).ok()?;
    bmp_files
        .iter()
        .find_map(|(k, v)| (k.as_u8_hex() == Some(hex)).then_some(v))
}
