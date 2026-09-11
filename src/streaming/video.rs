use std::{
    path::{Path, PathBuf},
    sync::{Arc, LazyLock},
};

use ffmpeg::{Codec, CodecOptions, FFmpegBuilder, Input, Output, PixelFormat};
use ffprobe::{ProbeResult, builder::FFprobeBuilder};
use loco_rs::prelude::*;
use regex::Regex;
use tokio::{fs, process::Command};
use which::which;

use crate::{
    core::{
        ResultExt,
        constants::{ENCODED_LOC, FONTS_LOC, METADATA_LOC, SUBTITLES_LOC},
    },
    downloaders::manager::DownloadManager,
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
            tracing::error!("unable to find {name} at PATH or assets/: {e}");
            PathBuf::new()
        })
}

pub static FFMPEG: LazyLock<PathBuf> = LazyLock::new(|| get_ffmpeg_path("ffmpeg"));
pub static FFPROBE: LazyLock<PathBuf> = LazyLock::new(|| get_ffmpeg_path("ffprobe"));

/// Integer-typed fields across all `rust_ffprobe` structs that ffprobe sometimes
/// emits as quoted strings instead of bare numbers.
/// To add a new field, append it here — the regex is built from this list.
const FFPROBE_INT_FIELDS: &[&str] = &[
    // StreamInfo
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
    // FrameInfo
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
    // PacketInfo / FormatInfo / ProgramInfo
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
/// Matches `"fieldname": "digits"` and captures both groups for replacement.
///
/// > **Tag-collision note**: ffprobe tag keys are conventionally SCREAMING_SNAKE_CASE
/// > (`BPS`, `DURATION`, …) while all fields above are lowercase, so false
/// > positives inside `tags` objects are not a practical concern.
static FFPROBE_NUMERIC_STRINGS: LazyLock<Regex> = LazyLock::new(|| {
    let pattern = format!(r#""({})":\s*"(-?\d+)""#, FFPROBE_INT_FIELDS.join("|"));
    Regex::new(&pattern).unwrap()
});

/// Normalizes ffprobe JSON by unquoting numeric-typed fields that ffprobe
/// sometimes emits as strings, then deserializes into [`ProbeResult`].
fn normalize_ffprobe_json(raw: &str) -> Result<ProbeResult> {
    let patched = FFPROBE_NUMERIC_STRINGS.replace_all(raw, r#""$1": $2"#);
    serde_json::from_str(&patched).to_loco_err()
}

/// Runs `ffprobe` with the given builder config, captures its stdout, normalizes
/// any string-typed numeric fields, and returns a parsed [`ProbeResult`].
pub async fn run_ffprobe(builder: FFprobeBuilder) -> Result<ProbeResult> {
    let args = builder.build_args().to_loco_err()?;

    let out = Command::new(FFPROBE.as_path())
        .args(&args)
        .output()
        .await
        .to_loco_inspect("failed to spawn ffprobe")?;

    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        return loco_err!("ffprobe failed: {stderr}");
    }

    let json = String::from_utf8(out.stdout).to_loco_inspect("ffprobe stdout not utf-8")?;

    normalize_ffprobe_json(&json)
}

impl VideoProcessor {
    /// Checks whether the video codec and pixel format can be copied directly
    /// without re-encoding, targeting modern browsers (last 2 years: Chrome, Safari, Edge, Firefox).
    pub fn is_compatible_video(codec_name: &str, pix_fmt: Option<&str>) -> bool {
        let fmt = pix_fmt.unwrap_or("").to_ascii_lowercase();
        match codec_name.to_ascii_lowercase().as_str() {
            // H.264 / AVC: 8-bit 4:2:0 is supported in all browsers.
            // 10-bit H.264 (Hi10P) has no browser decoder and must be re-encoded.
            "h264" | "avc" => matches!(fmt.as_str(), "yuv420p" | "yuvj420p"),

            // HEVC, VP9, AV1: modern browsers support 8-bit and 10-bit (Main / Profile 0/2).
            "hevc" | "h265" | "vp9" | "av1" => {
                matches!(fmt.as_str(), "yuv420p" | "yuvj420p" | "yuv420p10le")
            }

            _ => false,
        }
    }

    /// Checks whether the audio codec can be copied directly into an MP4 container for modern browsers.
    pub fn is_compatible_audio(codec_name: &str) -> bool {
        matches!(
            codec_name.to_ascii_lowercase().as_str(),
            "aac" | "opus" | "mp3" | "flac"
        )
    }

    /// Determines the video codec arguments for FFmpeg.
    /// Prefers stream copy (`-c:v copy`), adding `-tag:v hvc1` for HEVC streams on Apple/Safari.
    /// Incompatible video streams are re-encoded to universal 8-bit H.264 (`yuv420p`).
    pub fn build_video_codec_args(mut out: Output, probe: &ProbeResult) -> Output {
        let primary_video = probe.primary_video_stream();

        let is_video_copyable = primary_video
            .and_then(|s| s.codec_name.as_deref().map(|c| (c, s.pix_fmt.as_deref())))
            .is_some_and(|(codec, pix_fmt)| Self::is_compatible_video(codec, pix_fmt));

        let is_hevc = primary_video
            .and_then(|s| s.codec_name.as_deref())
            .is_some_and(|c| matches!(c.to_ascii_lowercase().as_str(), "hevc" | "h265"));

        if is_video_copyable {
            out = out.video_codec(Codec::copy());
            if is_hevc {
                out = out.option("tag:v", "hvc1");
            }
            out
        } else {
            out.video_codec_opts(
                CodecOptions::new(Codec::new("libx264"))
                    // CRF 18 = visually lossless; veryfast ensures speedy encode when recoding is mandatory
                    .quality(18)
                    .pixel_format(PixelFormat::yuv420p()),
            )
            .preset("veryfast")
        }
    }

    /// Determines the audio codec arguments for FFmpeg.
    /// Stream copies compatible audio (AAC, Opus, MP3, FLAC).
    /// If audio is incompatible (e.g. AC3, DTS, TrueHD), re-encodes to 192k AAC.
    /// If no audio stream is present, passes `-an`.
    pub fn build_audio_codec_args(out: Output, probe: &ProbeResult) -> Output {
        let Some(primary_audio) = probe.primary_audio_stream() else {
            return out.no_audio();
        };

        let is_audio_copyable = primary_audio
            .codec_name
            .as_deref()
            .is_some_and(Self::is_compatible_audio);

        if is_audio_copyable {
            out.audio_codec(Codec::copy())
        } else {
            out.audio_codec_opts(CodecOptions::new(Codec::aac()).bitrate("192k"))
        }
    }

    /// Assembles the complete FFmpeg command arguments for post-processing in a SINGLE process.
    ///
    /// In one invocation:
    /// - Dumps font attachments to `fonts/` via input option `-dump_attachment:t:{idx}`.
    /// - Transcodes/stream-copies video and audio to faststart MP4.
    /// - Extracts all raw subtitle streams into `subs/` via `-map 0:s:{idx} -c:s copy`.
    ///
    /// Returns the command arguments, the subtitle tracks list, and font filenames.
    pub fn build_ffmpeg_args(
        input: &str,
        output_mp4: &str,
        encoded_dir: &Path,
        probe: &ProbeResult,
    ) -> (FFmpegBuilder, Vec<VideoSubtitle>, Vec<SubtitleFont>) {
        let mut ff = FFmpegBuilder::with_executable(FFMPEG.as_path());
        let inp = Input::new(input);
        let mut out = Output::new(output_mp4);

        out = Self::build_audio_codec_args(out, probe);
        out = Self::build_video_codec_args(out, probe);

        let fonts_dir = encoded_dir.join(FONTS_LOC);
        let subs_dir = encoded_dir.join(SUBTITLES_LOC);

        let mut fonts = Vec::new();
        let mut subs = Vec::new();

        for stream in probe.streams.iter() {
            // Is attachment
            if stream.codec_type.as_deref() == Some("attachment") {
                let ext = stream.codec_name.as_deref().unwrap_or("ttf");
                let font_name = format!("font_{}.{}", fonts.len(), ext);
                let font_loc = fonts_dir
                    .join(font_name.as_str())
                    .to_string_lossy()
                    .into_owned();
                fonts.push(SubtitleFont {
                    file_name: font_name,
                    file_path: font_loc.clone(),
                    ..Default::default()
                });
                ff = ff.output(
                    Output::new(font_loc)
                        .format("data")
                        .option("map", format!("0:{}", stream.index))
                        .option("c", "copy"),
                );
            }
            // Is subtitle
            else if stream.codec_type.as_deref() == Some("subtitle") {
                let ext = match stream.codec_name.as_deref().unwrap_or("ass") {
                    "ass" | "ssa" => "ass",
                    "subrip" | "srt" => "srt",
                    "webvtt" => "vtt",
                    other => other,
                };
                let sub_name = format!("sub_{}.{}", subs.len(), ext);
                let sub_loc = subs_dir.join(sub_name).to_string_lossy().into_owned();

                let is_forced = stream
                    .disposition
                    .as_ref()
                    .and_then(|v| v.get("forced"))
                    .map(|v| v == &1u8)
                    .unwrap_or_default();

                subs.push(VideoSubtitle {
                    format: ext.to_string(),
                    title: stream
                        .title()
                        .filter(|t| !t.trim().is_empty())
                        .map(String::from)
                        .unwrap_or(format!("Subtitle {}", subs.len())),
                    language: stream.language().unwrap_or("unk").to_string(),
                    file_path: sub_loc.clone(),
                    track: subs.len() as i64,
                    is_forced,
                    ..Default::default()
                });

                ff = ff.output(
                    Output::new(sub_loc)
                        .option("map", format!("0:{}", stream.index))
                        .subtitle_codec(Codec::copy()),
                );
            }
        }

        out = out.no_subtitles().faststart();

        ff = ff.input(inp).output(out);

        (ff, subs, fonts)
    }

    /// Extracts chapters directly from a [`ProbeResult`].
    /// Parses `start_time` / `end_time` strings as seconds, falling back to
    /// `start` / `end` ticks converted via `time_base` if the string fields
    /// are absent.
    pub fn parse_probe_chapters(probe: &ProbeResult) -> Vec<VideoChapter> {
        probe
            .chapters
            .iter()
            .enumerate()
            .map(|(idx, ch)| {
                let start_time = ch
                    .start_time
                    .as_deref()
                    .and_then(|s| s.parse::<f64>().ok())
                    .unwrap_or_else(|| Self::ticks_to_seconds(ch.start, ch.time_base.as_deref()));

                let end_time = ch
                    .end_time
                    .as_deref()
                    .and_then(|s| s.parse::<f64>().ok())
                    .unwrap_or_else(|| Self::ticks_to_seconds(ch.end, ch.time_base.as_deref()));

                let title = ch
                    .title()
                    .filter(|t| !t.trim().is_empty())
                    .map(String::from)
                    .unwrap_or_else(|| format!("Chapter {}", idx + 1));

                VideoChapter {
                    chapter_id: ch.id,
                    title,
                    start_time,
                    end_time,
                    ..Default::default()
                }
            })
            .collect()
    }

    /// Converts `ticks` at `time_base` (e.g. `"1/1000"`) to fractional seconds.
    /// Returns 0.0 if the time base is missing or malformed.
    fn ticks_to_seconds(ticks: i64, time_base: Option<&str>) -> f64 {
        let tb = time_base.unwrap_or("1/1");
        let mut parts = tb.split('/');
        let num: f64 = parts.next().and_then(|s| s.parse().ok()).unwrap_or(1.0);
        let den: f64 = parts.next().and_then(|s| s.parse().ok()).unwrap_or(1.0);
        if den == 0.0 {
            return 0.0;
        }
        ticks as f64 * num / den
    }

    fn parse_probe_audio_tracks(probe_res: &ProbeResult) -> Vec<AudioTrack> {
        probe_res
            .audio_streams()
            .iter()
            .map(|stream| AudioTrack {
                language: stream.language().unwrap_or_default().to_string(),
                title: stream.title().unwrap_or_default().to_string(),
                channels: stream.channels.unwrap_or_default().to_string(),
                is_default: stream
                    .disposition
                    .as_ref()
                    .map(|v| v.get("default") == Some(&1u8))
                    .unwrap_or_default(),
                ..Default::default()
            })
            .collect()
    }
}

impl PostProcessor for VideoProcessor {
    async fn post_process(
        processor: Arc<MediaProcessor>,
        manager: Arc<DownloadManager>,
        mut item: VaultSubItem,
    ) -> Result<VaultMetadataDto> {
        // Mark as PROCESSING before handing off to ffmpeg.
        item.status = VaultStatus::PROCESSING;
        item.error_msg = None;
        manager.wake_daemon();

        let sub_item_id = item.id;
        let source_path = item.source_path.clone();
        let title = item.title.clone();

        tracing::info!(vault_id = %sub_item_id, title, source_path, "post_process started");

        let input = PathBuf::from(source_path.clone());
        let parent_dir = input
            .parent()
            .ok_or(loco_err_msg!("unable to extract parent dir"))?;
        let stem = input
            .file_stem()
            .ok_or(loco_err_msg!("unable to extract file stem"))?;

        //from vault/<vault id>/ in vault/<vault id>/encoded/<sub id>/
        let encoded_dir = parent_dir
            .to_path_buf()
            .join(ENCODED_LOC.as_str())
            .join(sub_item_id.to_string());

        // Truncate encoded dir and prepare subs/fonts folders before spawning ffmpeg.
        if encoded_dir.is_dir() {
            fs::remove_dir_all(&encoded_dir).await?;
        }
        fs::create_dir_all(encoded_dir.join(SUBTITLES_LOC)).await?;
        fs::create_dir_all(encoded_dir.join(FONTS_LOC)).await?;

        let output_file = encoded_dir.join(stem).with_extension("mp4");
        let output = output_file.to_string_lossy().into_owned();

        let probe_res = run_ffprobe(
            FFprobeBuilder::with_executable(FFPROBE.as_path())
                .input(source_path.clone())
                .show_format()
                .show_streams()
                .show_chapters(),
        )
        .await
        .to_loco_inspect("failed to execute ffprobe")?;

        // Extract chapters directly from the ProbeResult.
        let chapters = Self::parse_probe_chapters(&probe_res);
        tracing::debug!(vault_id = %sub_item_id, chapter_count = chapters.len(), "probed container chapters");

        let audio_tracks = Self::parse_probe_audio_tracks(&probe_res);
        tracing::debug!(vault_id = %sub_item_id, audio_tracks_count = audio_tracks.len(), "probed container audio_tracks");

        // Total duration in microseconds for progress % calculation.
        // ProbeResult::duration() returns Option<f64> in seconds.
        let total_duration: f64 = probe_res
            .duration()
            .unwrap_or_else(|| {
                tracing::warn!(vault_id = %sub_item_id, "ffprobe returned no duration; progress % will be -1");
                -1f64 // Negative means no duration and avoids div/0 error. ui must understand this to omit progress
            });

        let (ff, subtitles, fonts) =
            Self::build_ffmpeg_args(&source_path, &output, &encoded_dir, &probe_res);

        tracing::info!(vault_id = %sub_item_id, output, args=?&ff.command(), "spawning single ffmpeg process for video, subtitles, and fonts");

        processor.active_sub_items.insert(item.id, item);

        let ffmpeg_out = ff
            .on_progress(move |progress| {
                if let Some(mut item) = processor.active_sub_items.get_mut(&sub_item_id) {
                    item.status = VaultStatus::PROCESSING;
                    item.total_bytes = progress.size.unwrap_or_default() as i64;

                    let current_secs = progress.time.unwrap_or_default().as_secs_f64();

                    let percent = (current_secs / total_duration) * 100.0;
                    item.progress = percent.min(100.0);

                    // bitrate is in bits/s
                    item.speed_bps = (progress.bitrate.unwrap_or_default() / 8.0) as i64;

                    let remaining_media_secs = 0f64.max(total_duration - current_secs);

                    item.eta_seconds = progress
                        .speed
                        .filter(|&speed| speed > 0.0)
                        .map(|speed| (remaining_media_secs / speed) as i64);
                }
            })
            .run()
            .await
            .to_loco_err()?;

        if ffmpeg_out.success() {
            tracing::info!(vault_id = %sub_item_id, input=?source_path, output=?output_file, "ffmpeg finished successfully");

            // Id will be assigned before saving in db.
            let metadata = VaultMetadataDto {
                vault_metadata: VaultMetadata {
                    sub_item_id,
                    file_name: output_file
                        .file_name()
                        .map(|v| v.to_string_lossy().into_owned())
                        .unwrap_or_default(),
                    thumbnail_path: None,
                    file_path: output,
                    ..Default::default()
                },
                audio_tracks,
                video_chapters: chapters,
                video_subtitles: subtitles,
                subtitle_fonts: fonts,
            };

            let metadata_path = encoded_dir.join(METADATA_LOC);
            if let Ok(json_str) = serde_json::to_string_pretty(&metadata) {
                let _ = fs::write(&metadata_path, json_str).await;
            }

            // Remove empty sub/font folders if none were extracted
            let subs_dir = encoded_dir.join(SUBTITLES_LOC);
            if metadata.video_subtitles.is_empty() && subs_dir.is_dir() {
                let _ = fs::remove_dir_all(&subs_dir).await;
            }
            let fonts_dir = encoded_dir.join(FONTS_LOC);
            if metadata.subtitle_fonts.is_empty() && fonts_dir.is_dir() {
                let _ = fs::remove_dir_all(&fonts_dir).await;
            }
            Ok(metadata)
        } else {
            let msg = format!(
                "ffmpeg exited with code {}",
                ffmpeg_out.status.code().unwrap_or(-1)
            );
            tracing::error!(vault_id = %sub_item_id, title, stderr=?ffmpeg_out.stderr_str(), "{}", msg);
            loco_err!("{msg}")
        }
    }
}
