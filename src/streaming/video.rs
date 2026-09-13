use std::{
    path::{Path, PathBuf},
    sync::{Arc, LazyLock},
    time::Duration,
};

use ffmpeg::{
    Codec, CodecOptions, FFmpegBuilder, Input, Output, PixelFormat, StreamMap, StreamSpecifier,
};
use ffprobe::{ProbeResult, builder::FFprobeBuilder};
use loco_rs::prelude::*;
use regex::Regex;
use tokio::{
    fs,
    process::Command,
    sync::watch::{self},
    time::interval,
};
use which::which;

use crate::{
    core::{
        ResultExt,
        constants::{ENCODED_LOC, FONTS_LOC, SUBTITLES_LOC},
        sanitize_filename,
    },
    dtos::vault::VaultMetadataDto,
    loco_err, loco_err_msg,
    models::{
        audio_tracks::AudioTrack, subtitle_fonts::SubtitleFont, vault::VaultStatus,
        vault_metadata::VaultMetadata, vault_sub_item::VaultSubItem, video_chapters::VideoChapter,
        video_subtitles::VideoSubtitle,
    },
    streaming::{PostProcessor, processor::MediaProcessor},
};

pub struct VideoProcessor {}

fn get_ffmpeg_path(bin_name: &str) -> PathBuf {
    let name = if bin_name.trim().is_empty() {
        "ffmpeg"
    } else {
        bin_name.trim()
    };
    which(name)
        .or_else(|_| which(format!("assets/{name}")))
        .unwrap_or_else(|e| {
            tracing::warn!("which({name}) failed: {e}. Relying on OS PATH.");
            PathBuf::from(name) // Fallback to raw name so OS can attempt resolution
        })
}

pub static FFMPEG: LazyLock<PathBuf> = LazyLock::new(|| get_ffmpeg_path("ffmpeg"));
pub static FFPROBE: LazyLock<PathBuf> = LazyLock::new(|| get_ffmpeg_path("ffprobe"));

/// Integer-typed fields across all `rust_ffprobe` structs that ffprobe sometimes emits as strings.
const FFPROBE_INT_FIELDS: &[&str] = &[
    "bits_per_raw_sample",
    "bits_per_sample",
    "index",
    "width",
    "height",
    "coded_width",
    "coded_height",
    "has_b_frames",
    "level",
    "refs",
    "extradata_size",
    "channels",
    "start_pts",
    "duration_ts",
    "stream_index",
    "key_frame",
    "pts",
    "pkt_pts",
    "pkt_dts",
    "best_effort_timestamp",
    "pkt_duration",
    "coded_picture_number",
    "display_picture_number",
    "interlaced_frame",
    "top_field_first",
    "repeat_pict",
    "nb_samples",
    "nb_streams",
    "nb_programs",
    "probe_score",
    "program_id",
    "program_num",
    "pmt_pid",
    "pcr_pid",
    "end_pts",
];

/// Regex built from [`FFPROBE_INT_FIELDS`] at startup.
static FFPROBE_NUMERIC_STRINGS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(&format!(
        r#""({})":\s*"(-?\d+)""#,
        FFPROBE_INT_FIELDS.join("|")
    ))
    .unwrap()
});

/// Normalizes ffprobe JSON by unquoting numeric-typed fields that ffprobe
/// sometimes emits as strings, then deserializes into [`ProbeResult`].
fn normalize_ffprobe_json(raw: &str) -> Result<ProbeResult> {
    let patched = FFPROBE_NUMERIC_STRINGS.replace_all(raw, r#""$1": $2"#);
    serde_json::from_str(patched.as_ref()).to_loco_err()
}

pub async fn run_ffprobe(builder: FFprobeBuilder) -> Result<ProbeResult> {
    let args = builder.build_args().to_loco_err()?;
    let out = Command::new(FFPROBE.as_path())
        .args(&args)
        .output()
        .await
        .to_loco_inspect("failed to spawn ffprobe")?;

    if !out.status.success() {
        return loco_err!("ffprobe failed: {}", String::from_utf8_lossy(&out.stderr));
    }

    let json = String::from_utf8(out.stdout).to_loco_inspect("ffprobe stdout not utf-8")?;
    normalize_ffprobe_json(&json)
}

impl VideoProcessor {
    pub fn is_compatible_video(codec_name: &str, pix_fmt: Option<&str>) -> bool {
        let fmt = pix_fmt.unwrap_or("").to_ascii_lowercase();
        match codec_name.to_ascii_lowercase().as_str() {
            "h264" | "avc" => matches!(fmt.as_str(), "yuv420p" | "yuvj420p"),
            "hevc" | "h265" | "vp9" | "av1" => {
                matches!(fmt.as_str(), "yuv420p" | "yuvj420p" | "yuv420p10le")
            }
            _ => false,
        }
    }

    pub fn is_compatible_audio(codec_name: &str) -> bool {
        matches!(
            codec_name.to_ascii_lowercase().as_str(),
            "aac" | "opus" | "mp3" | "flac"
        )
    }

    pub fn build_video_codec_args(
        mut out: Output,
        mut ff: FFmpegBuilder,
        probe: &ProbeResult,
    ) -> (Output, FFmpegBuilder) {
        let primary = probe.video_streams().into_iter().find(|v| {
            v.is_video()
                && !matches!(
                    v.codec_name
                        .as_deref()
                        .unwrap_or("")
                        .to_lowercase()
                        .as_str(),
                    "mjpeg" | "png" | "bmp" | "webp" | "gif"
                )
        });

        let is_copyable = primary
            .and_then(|s| {
                s.codec_name
                    .as_deref()
                    .map(|c| Self::is_compatible_video(c, s.pix_fmt.as_deref()))
            })
            .unwrap_or(false);

        if is_copyable {
            out = out.video_codec(Codec::copy());
            if primary.is_some_and(|s| matches!(s.codec_name.as_deref(), Some("hevc" | "h265"))) {
                out = out.option("tag:v", "hvc1");
            }
        } else {
            out = out
                .video_codec_opts(
                    CodecOptions::new(Codec::new("libx264")).pixel_format(PixelFormat::yuv420p()),
                )
                // CRF 18 = visually lossless; veryfast ensures speedy encode when recoding is mandatory
                .option("crf", "18")
                .preset("veryfast");
        }

        if let Some(pv) = primary {
            ff = ff.map(StreamMap::specific(
                0,
                StreamSpecifier::Index(pv.index as usize),
            ));
        } else {
            ff = ff.map(StreamMap::video_from(0));
        }

        (out, ff)
    }

    /// Determines the audio codec arguments per stream for FFmpeg.
    /// Stream copies compatible audio. Incompatible audio is converted to AAC with VBR quality 2.
    pub fn build_audio_codec_args(
        mut out: Output,
        mut ff: FFmpegBuilder,
        probe: &ProbeResult,
    ) -> (Output, FFmpegBuilder) {
        let streams = probe.audio_streams();
        if streams.is_empty() {
            return (out.no_audio(), ff);
        }

        for (idx, s) in streams.iter().enumerate() {
            let is_compatible = s
                .codec_name
                .as_deref()
                .is_some_and(Self::is_compatible_audio);

            if is_compatible {
                out = out.option(format!("c:a:{idx}"), "copy");
            } else {
                out = out
                    .option(format!("c:a:{idx}"), "aac")
                    .option(format!("q:a:{idx}"), "2"); // VBR target for better surround sound handling
            }
        }

        // Map all audio streams
        ff = ff.map(StreamMap::audio_from(0));

        (out, ff)
    }

    fn build_font_codec_args(
        mut inp: Input,
        probe: &ProbeResult,
        fonts_dir: PathBuf,
    ) -> (Vec<SubtitleFont>, Input) {
        let mut fonts = Vec::new();
        for stream in probe.streams.iter() {
            let codec = stream.codec_name.as_deref().unwrap_or("");

            // Manage FONTS
            if stream.codec_type.as_deref() == Some("attachment") {
                // 1. FILTER: Ensure the attachment is actually a font, not Cover Art (mjpeg/png)
                let filename = stream
                    .tags
                    .get("filename")
                    .map(|s| s.to_lowercase())
                    .unwrap_or_default();
                let mimetype = stream
                    .tags
                    .get("mimetype")
                    .map(|s| s.to_lowercase())
                    .unwrap_or_default();

                let is_font = matches!(
                    codec.to_lowercase().as_str(),
                    "ttf" | "otf" | "woff" | "woff2" | "truetype" | "opentype"
                ) || mimetype.contains("font")
                    || filename.ends_with(".ttf")
                    || filename.ends_with(".otf")
                    || filename.ends_with(".woff")
                    || filename.ends_with(".woff2");

                if !is_font {
                    continue;
                }

                // Try to grab the original font filename from MKV tags
                let original_name = stream
                    .tags
                    .get("filename")
                    .cloned()
                    .unwrap_or_else(|| format!("font_{}.{}", fonts.len(), codec));

                // Sanitize the filename to prevent weird characters or path traversal
                let safe_name = sanitize_filename(&original_name);

                let font_loc = fonts_dir.join(&safe_name).to_string_lossy().into_owned();

                fonts.push(SubtitleFont {
                    font_name: safe_name,
                    file_path: font_loc.clone(),
                    ..Default::default()
                });

                inp = inp.option(format!("dump_attachment:{}", stream.index), font_loc);
            }
        }
        (fonts, inp)
    }

    fn build_subtitle_codec_args(
        mut ff: FFmpegBuilder,
        probe: &ProbeResult,
        subs_dir: PathBuf,
    ) -> (Vec<VideoSubtitle>, FFmpegBuilder) {
        let mut subs = vec![];
        for stream in probe.subtitle_streams().iter() {
            let codec = stream.codec_name.as_deref().unwrap_or("");

            if matches!(codec, "dvd_subtitle" | "vobsub") {
                continue; // Skip image-based subs
            }

            let ext = match codec {
                "ass" | "ssa" => "ass",
                "subrip" | "srt" => "srt",
                "webvtt" => "vtt",
                "hdmv_pgs_subtitle" => "sup",
                _ => codec,
            };

            let sub_loc = subs_dir
                .join(format!("sub_{}.{}", subs.len(), ext))
                .to_string_lossy()
                .into_owned();

            subs.push(VideoSubtitle {
                format: ext.to_string(),
                title: stream
                    .title()
                    .filter(|t| !t.trim().is_empty())
                    .map(String::from)
                    .unwrap_or_else(|| format!("Subtitle {}", subs.len())),
                language: stream.language().unwrap_or("unk").to_string(),
                file_path: sub_loc.clone(),
                track: subs.len() as i64,
                is_forced: stream
                    .disposition
                    .as_ref()
                    .is_some_and(|v| v.get("forced") == Some(&1u8)),
                ..Default::default()
            });

            ff = ff.output(
                Output::new(sub_loc)
                    .option("map", format!("0:{}", stream.index))
                    .subtitle_codec(Codec::copy()),
            );
        }
        (subs, ff)
    }

    pub fn build_ffmpeg_args(
        input: &str,
        output_mp4: &str,
        encoded_dir: &Path,
        probe: &ProbeResult,
    ) -> (FFmpegBuilder, Vec<VideoSubtitle>, Vec<SubtitleFont>) {
        let mut ff = FFmpegBuilder::with_executable(FFMPEG.as_path());
        let mut inp = Input::new(input);
        let mut out = Output::new(output_mp4);

        (out, ff) = Self::build_audio_codec_args(out, ff, probe);
        (out, ff) = Self::build_video_codec_args(out, ff, probe);

        out = out.no_subtitles().faststart();

        let (fonts_dir, subs_dir) = (encoded_dir.join(FONTS_LOC), encoded_dir.join(SUBTITLES_LOC));
        let (fonts, subs);

        (fonts, inp) = Self::build_font_codec_args(inp, probe, fonts_dir);
        ff = ff.input(inp).output(out); // Push inp and out before subs

        (subs, ff) = Self::build_subtitle_codec_args(ff, probe, subs_dir);

        (ff, subs, fonts)
    }

    pub fn parse_probe_chapters(probe: &ProbeResult) -> Vec<VideoChapter> {
        probe
            .chapters
            .iter()
            .enumerate()
            .map(|(idx, ch)| {
                let parse_time = |t_str: Option<&String>, ticks, tb| {
                    t_str
                        .and_then(|s| s.parse::<f64>().ok())
                        .unwrap_or_else(|| Self::ticks_to_seconds(ticks, tb))
                };

                VideoChapter {
                    chapter_id: ch.id,
                    title: ch
                        .title()
                        .filter(|t| !t.trim().is_empty())
                        .map(String::from)
                        .unwrap_or_else(|| format!("Chapter {}", idx + 1)),
                    start_time: parse_time(
                        ch.start_time.as_ref(),
                        ch.start,
                        ch.time_base.as_deref(),
                    ),
                    end_time: parse_time(ch.end_time.as_ref(), ch.end, ch.time_base.as_deref()),
                    ..Default::default()
                }
            })
            .collect()
    }

    fn ticks_to_seconds(ticks: i64, time_base: Option<&str>) -> f64 {
        let (num_str, den_str) = time_base
            .unwrap_or("1/1")
            .split_once('/')
            .unwrap_or(("1", "1"));
        let den: f64 = den_str.parse().unwrap_or(1.0);
        if den == 0.0 {
            0.0
        } else {
            ticks as f64 * num_str.parse::<f64>().unwrap_or(1.0) / den
        }
    }

    fn parse_probe_audio_tracks(probe_res: &ProbeResult) -> Vec<AudioTrack> {
        probe_res
            .audio_streams()
            .iter()
            .map(|s| AudioTrack {
                language: s.language().unwrap_or_default().to_string(),
                title: s.title().unwrap_or_default().to_string(),
                channels: s.channels.unwrap_or_default().to_string(),
                is_default: s
                    .disposition
                    .as_ref()
                    .is_some_and(|v| v.get("default") == Some(&1u8)),
                ..Default::default()
            })
            .collect()
    }
}

impl PostProcessor for VideoProcessor {
    async fn post_process(
        processor: Arc<MediaProcessor>,
        item: VaultSubItem,
    ) -> Result<VaultMetadataDto> {
        if let Some(mut item) = processor.active_sub_items.get_mut(&item.id) {
            item.status = VaultStatus::PROCESSING;
            item.error_msg = None;
        }

        let sub_item_id = item.id;
        let source_path = item.source_path.clone();

        tracing::info!(vault_id = %sub_item_id, title=?item.title, source_path, "post_process started");

        let input_path = PathBuf::from(&source_path);
        let parent_dir = input_path
            .parent()
            .ok_or(loco_err_msg!("unable to extract parent dir"))?;
        let stem = input_path
            .file_stem()
            .ok_or(loco_err_msg!("unable to extract file stem"))?;

        let encoded_dir = parent_dir
            .join(ENCODED_LOC.as_str())
            .join(sub_item_id.to_string());
        let output_file = encoded_dir.join(stem).with_extension("mp4");
        let output_path_str = output_file.to_string_lossy().into_owned();

        // Ensure clean working directory
        if encoded_dir.is_dir() {
            fs::remove_dir_all(&encoded_dir).await?;
        }
        fs::create_dir_all(&encoded_dir).await?;

        let probe_res = run_ffprobe(
            FFprobeBuilder::with_executable(FFPROBE.as_path())
                .input(source_path.as_str())
                .show_format()
                .show_streams()
                .show_chapters(),
        )
        .await
        .to_loco_inspect("failed to execute ffprobe")?;

        let chapters = Self::parse_probe_chapters(&probe_res);
        let audio_tracks = Self::parse_probe_audio_tracks(&probe_res);

        // Total duration for progress %. Use -1 if unknown.
        let total_duration: f64 = probe_res.duration().unwrap_or(-1.0);

        let (ff, subtitles, fonts) =
            Self::build_ffmpeg_args(&source_path, &output_path_str, &encoded_dir, &probe_res);

        // Smart directory creation: Only create sub/font folders if there's actually data to write
        if !subtitles.is_empty() {
            fs::create_dir_all(encoded_dir.join(SUBTITLES_LOC)).await?;
        }
        if !fonts.is_empty() {
            fs::create_dir_all(encoded_dir.join(FONTS_LOC)).await?;
        }

        tracing::info!(vault_id = %sub_item_id, output=?output_path_str, args=?&ff.command(), "spawning ffmpeg");

        let (tx, mut rx) = watch::channel((0, 0.0, 0.0, 0.0));

        let progress_task = tokio::spawn(async move {
            let mut ticker = interval(Duration::from_secs(2));
            while rx.changed().await.is_ok() {
                let (size, bitrate, time_secs, speed) = *rx.borrow();

                if let Some(mut item) = processor.active_sub_items.get_mut(&sub_item_id) {
                    item.status = VaultStatus::PROCESSING;
                    item.total_bytes = size;
                    item.speed_bps = (bitrate / 8.0) as i64;

                    item.progress = (time_secs / total_duration * 100.0).clamp(-1.0, 100.0);

                    let remaining_secs = 0f64.max(total_duration - time_secs);
                    item.eta_seconds = if speed > 0.0 {
                        Some((remaining_secs / speed) as i64)
                    } else {
                        None
                    };
                }
                ticker.tick().await;
            }
        });

        let ffmpeg_out = ff
            .on_progress(move |progress| {
                let _ = tx.send((
                    progress.size.unwrap_or_default() as i64,
                    progress.bitrate.unwrap_or_default(),
                    progress.time.unwrap_or_default().as_secs_f64(),
                    progress.speed.unwrap_or_default(),
                ));
            })
            .run()
            .await
            .to_loco_err()?;

        progress_task.abort();

        if ffmpeg_out.success() {
            tracing::info!(vault_id = %sub_item_id, input=?source_path, output=?output_file, "ffmpeg finished successfully");

            // Final metadata is collected and returned to update the DB on success.
            Ok(VaultMetadataDto {
                vault_metadata: VaultMetadata {
                    sub_item_id,
                    file_name: output_file
                        .file_name()
                        .map(|v| v.to_string_lossy().into_owned())
                        .unwrap_or_default(),
                    thumbnail_path: None,
                    file_path: output_path_str,
                    ..Default::default()
                },
                audio_tracks,
                video_chapters: chapters,
                video_subtitles: subtitles,
                subtitle_fonts: fonts,
            })
        } else {
            let code = ffmpeg_out.status.code().unwrap_or(-1);
            tracing::error!(vault_id = %sub_item_id, title=?item.title, stderr=?ffmpeg_out.stderr_str(), "ffmpeg exited with code {}", code);
            loco_err!("ffmpeg exited with code {}", code)
        }
    }
}
