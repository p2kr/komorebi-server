use std::{
    path::{Path, PathBuf},
    sync::{Arc, LazyLock},
};

use ffprobe::{ProbeResult, builder::FFprobeBuilder};
use loco_rs::prelude::*;
use tokio::{
    fs,
    io::{AsyncBufReadExt, BufReader},
};
use which::which;

use crate::{
    core::{
        ResultExt,
        constants::{ENCODED_LOC, FONTS_LOC, SUBTITLES_LOC},
    },
    downloaders::manager::DownloadManager,
    dtos::{Chapter, Subtitle},
    loco_err, loco_err_msg,
    models::{vault::VaultItemStatus, vault_sub_item::VaultSubItem},
    streaming::PostProcessor,
};

/// Local metadata written to `encoded/metadata.json` after a successful encode.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct VaultMetadata {
    chapters: Vec<Chapter>,
    subtitles: Vec<Subtitle>,
    fonts: Vec<String>,
}

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

static FFMPEG: LazyLock<PathBuf> = LazyLock::new(|| get_ffmpeg_path("ffmpeg"));
static FFPROBE: LazyLock<PathBuf> = LazyLock::new(|| get_ffmpeg_path("ffprobe"));

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
    pub fn build_video_codec_args(probe: &ProbeResult) -> Vec<String> {
        let primary_video = probe
            .streams
            .iter()
            .filter(|s| {
                s.is_video()
                    && s.disposition.as_ref().and_then(|v| v.get("attached_pic")) != Some(&1u8)
            })
            .max_by_key(|s| s.width.unwrap_or(0) * s.height.unwrap_or(0))
            .or_else(|| probe.streams.iter().find(|s| s.is_video()));

        let is_video_copyable = primary_video
            .and_then(|s| s.codec_name.as_deref().map(|c| (c, s.pix_fmt.as_deref())))
            .is_some_and(|(codec, pix_fmt)| Self::is_compatible_video(codec, pix_fmt));

        let is_hevc = primary_video
            .and_then(|s| s.codec_name.as_deref())
            .is_some_and(|c| matches!(c.to_ascii_lowercase().as_str(), "hevc" | "h265"));

        if is_video_copyable {
            let mut args = vec!["-c:v".into(), "copy".into()];
            if is_hevc {
                args.extend(["-tag:v".into(), "hvc1".into()]);
            }
            args
        } else {
            // CRF 18 = visually lossless; veryfast ensures speedy encode when recoding is mandatory
            [
                "-c:v", "libx264", "-crf", "18", "-preset", "veryfast", "-pix_fmt", "yuv420p",
            ]
            .into_iter()
            .map(String::from)
            .collect()
        }
    }

    /// Determines the audio codec arguments for FFmpeg.
    /// Stream copies compatible audio (AAC, Opus, MP3, FLAC).
    /// If audio is incompatible (e.g. AC3, DTS, TrueHD), re-encodes to 192k AAC.
    /// If no audio stream is present, passes `-an`.
    pub fn build_audio_codec_args(probe: &ProbeResult) -> Vec<String> {
        let Some(primary_audio) = probe
            .streams
            .iter()
            .filter(|s| s.is_audio())
            .max_by_key(|s| {
                (
                    s.disposition
                        .as_ref()
                        .and_then(|v| v.get("default"))
                        .copied()
                        .unwrap_or(0u8),
                    s.channels.unwrap_or(0),
                )
            })
        else {
            return vec!["-an".into()];
        };

        let is_audio_copyable = primary_audio
            .codec_name
            .as_deref()
            .is_some_and(Self::is_compatible_audio);

        if is_audio_copyable {
            vec!["-c:a".into(), "copy".into()]
        } else {
            ["-c:a", "aac", "-b:a", "192k"]
                .into_iter()
                .map(String::from)
                .collect()
        }
    }

    /// Assembles the complete FFmpeg command arguments for post-processing in a SINGLE process.
    /// In one invocation:
    /// - Dumps font attachments to `fonts/` via input option `-dump_attachment:t:{idx}`.
    /// - Transcodes/stream-copies video and audio to faststart MP4.
    /// - Extracts all raw subtitle streams into `subs/` via `-map 0:s:{idx} -c:s copy`.
    /// Returns the command arguments, the subtitle tracks list, and font filenames.
    pub fn build_ffmpeg_args(
        input: &str,
        output_mp4: &str,
        encoded_dir: &Path,
        probe: &ProbeResult,
    ) -> (Vec<String>, Vec<Subtitle>, Vec<String>) {
        let mut args: Vec<String> = vec!["-y".into()];

        // 1. Font attachments dump (input options before -i)
        let fonts_dir = encoded_dir.join(FONTS_LOC);
        let mut fonts = Vec::new();
        for (att_idx, stream) in probe
            .streams
            .iter()
            .filter(|s| s.codec_type.as_deref() == Some("attachment"))
            .enumerate()
        {
            let ext = stream
                .codec_name
                .as_deref()
                .unwrap_or("ttf")
                .to_ascii_lowercase();

            let font_name = format!("font_{att_idx}.{ext}");
            args.push(format!("-dump_attachment:t:{att_idx}"));
            args.push(fonts_dir.join(&font_name).to_string_lossy().into_owned());
            fonts.push(font_name);
        }

        // 2. Input file
        args.push("-i".into());
        args.push(input.to_string());

        // 3. Primary output: faststart MP4 (video + audio)
        args.extend(Self::build_video_codec_args(probe));
        args.extend(Self::build_audio_codec_args(probe));
        args.push("-sn".into()); // disable subtitles in the MP4 container
        args.extend(
            [
                "-movflags",
                "+faststart",
                "-progress",
                "pipe:1",
                "-nostats",
                "-stats_period",
                "2",
            ]
            .into_iter()
            .map(String::from),
        );
        args.push(output_mp4.to_string());

        // 4. Secondary outputs: Subtitle tracks extracted in the same process
        let subs_dir = encoded_dir.join(SUBTITLES_LOC);
        let mut subtitles = Vec::new();
        for (track_idx, stream) in probe.streams.iter().filter(|s| s.is_subtitle()).enumerate() {
            let codec = stream
                .codec_name
                .as_deref()
                .unwrap_or("ass")
                .to_ascii_lowercase();

            let format = match codec.as_str() {
                "ass" | "ssa" => "ass",
                "subrip" | "srt" => "srt",
                "webvtt" => "vtt",
                other => other,
            };

            let file_name = format!("sub_{track_idx}.{format}");
            let lang = stream.language().unwrap_or("und").to_string();
            let label = stream
                .title()
                .filter(|t| !t.trim().is_empty())
                .map(String::from)
                .unwrap_or_else(|| format!("Subtitle {}", track_idx + 1));

            args.extend([
                "-map".into(),
                format!("0:s:{track_idx}"),
                "-c:s".into(),
                "copy".into(),
                subs_dir.join(&file_name).to_string_lossy().into_owned(),
            ]);

            subtitles.push(Subtitle {
                track: track_idx,
                lang,
                label,
                format: format.to_string(),
                path: file_name,
                is_forced: None,
            });
        }

        (args, subtitles, fonts)
    }

    /// Extracts chapters directly from a [`ProbeResult`].
    /// Parses `start_time` / `end_time` strings as seconds, falling back to
    /// `start` / `end` ticks converted via `time_base` if the string fields
    /// are absent.
    pub fn parse_probe_chapters(probe: &ProbeResult) -> Vec<Chapter> {
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

                Chapter {
                    id: ch.id,
                    title,
                    start_time,
                    end_time,
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

    /// Searches the vault item directory for a processed video file inside
    /// the `encoded/` sub-directory.
    pub async fn find_processed_file(source_path: &str) -> Result<Option<PathBuf>> {
        let encoded_dir = PathBuf::from(source_path).join(ENCODED_LOC.as_str());
        if !encoded_dir.is_dir() {
            return Ok(None);
        }

        let mut rd = fs::read_dir(&encoded_dir).await?;
        while let Some(entry) = rd.next_entry().await? {
            let path = entry.path();
            if path.is_file()
                && let Some(ext) = path.extension().and_then(|e| e.to_str())
                && matches!(ext.to_ascii_lowercase().as_str(), "mp4" | "webm" | "mkv")
            {
                return Ok(Some(path));
            }
        }
        Ok(None)
    }
}

impl PostProcessor for VideoProcessor {
    async fn post_process(
        source_path: PathBuf,
        manager: Arc<DownloadManager>,
        mut item: VaultSubItem,
    ) -> Result<()> {
        // Mark as PROCESSING before handing off to ffmpeg.
        item.status = VaultItemStatus::PROCESSING;
        item.error_msg = None;
        manager.wake_daemon();

        let vault_id = item.id;
        let dest_path = item.source_path.clone();
        let title = item.title.clone();

        tracing::info!(vault_id = %vault_id, title, dest_path, "post_process started");

        tracing::debug!(vault_id = %vault_id, path = %source_path.display(), "resolved video file");

        let input = source_path
            .to_str()
            .ok_or(loco_err_msg!("cannot convert input file path to string"))?
            .to_owned();

        let stem = source_path
            .file_stem()
            .ok_or(loco_err_msg!("unable to extract file stem"))?;

        let encoded_dir = PathBuf::from(&item.source_path).join(ENCODED_LOC.as_str());

        // Truncate encoded dir and prepare subs/fonts folders before spawning ffmpeg.
        if encoded_dir.is_dir() {
            fs::remove_dir_all(&encoded_dir).await?;
        }
        fs::create_dir_all(encoded_dir.join(SUBTITLES_LOC)).await?;
        fs::create_dir_all(encoded_dir.join(FONTS_LOC)).await?;

        let output_file = encoded_dir.join(stem).with_extension("mp4");
        let output = output_file
            .to_str()
            .ok_or(loco_err_msg!("cannot convert output file path to string"))?
            .to_owned();

        let probe_res = FFprobeBuilder::with_executable(FFPROBE.as_path())
            .input(source_path.clone())
            .show_format()
            .show_streams()
            .show_chapters()
            .run()
            .await
            .to_loco_err()?;

        // Extract chapters directly from the ProbeResult.
        let chapters = Self::parse_probe_chapters(&probe_res);
        tracing::debug!(vault_id = %vault_id, chapter_count = chapters.len(), "probed container chapters");

        // Total duration in microseconds for progress % calculation.
        // ProbeResult::duration() returns Option<f64> in seconds.
        let total_duration_us: u64 = probe_res
            .duration()
            .map(|d| (d * 1_000_000.0) as u64)
            .unwrap_or_else(|| {
                tracing::warn!(vault_id = %vault_id, "ffprobe returned no duration; progress % will be 0");
                0
            });

        let (args, subtitles, fonts) =
            Self::build_ffmpeg_args(&input, &output, &encoded_dir, &probe_res);

        tracing::info!(vault_id = %vault_id, output, ?args, "spawning single ffmpeg process for video, subtitles, and fonts");

        let mut child = tokio::process::Command::new(FFMPEG.as_path())
            .args(&args)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .map_err(|e| loco_err_msg!("failed to spawn ffmpeg: {}", e))?;

        let stdout = child
            .stdout
            .take()
            .ok_or(loco_err_msg!("ffmpeg stdout not available"))?;

        // Drain stderr in a background task so it never blocks stdout reads.
        // Lines are collected and logged only on failure.
        let stderr_drain = {
            let stderr = child
                .stderr
                .take()
                .ok_or(loco_err_msg!("ffmpeg stderr not available"))?;
            tokio::spawn(async move {
                let mut lines = BufReader::new(stderr).lines();
                let mut collected: Vec<String> = Vec::new();
                while let Ok(Some(line)) = lines.next_line().await {
                    collected.push(line);
                }
                collected
            })
        };

        let mut lines = BufReader::new(stdout).lines();

        // Running state parsed from ffmpeg's progress key=value pairs.
        let mut out_time_us: u64 = 0;
        let mut processed_bytes: i64 = 0;
        let mut bitrate_kbps: f64 = 0.0;
        let mut speed: f64 = 1.0;

        while let Some(line) = lines
            .next_line()
            .await
            .map_err(|e| loco_err_msg!("ffmpeg stdout read error: {}", e))?
        {
            let Some((key, val)) = line.split_once('=') else {
                continue;
            };
            let val = val.trim();

            match key.trim() {
                "out_time_us" => out_time_us = val.parse().unwrap_or(out_time_us),
                "total_size" => processed_bytes = val.parse().unwrap_or(processed_bytes),
                // "1234.5kbits/s" or "N/A"
                "bitrate" => {
                    bitrate_kbps = val
                        .trim_end_matches("kbits/s")
                        .parse::<f64>()
                        .unwrap_or(bitrate_kbps);
                }
                // "1.23x" or "N/A"
                "speed" => {
                    speed = val
                        .trim_end_matches('x')
                        .parse::<f64>()
                        .unwrap_or(speed)
                        .max(0.001);
                }
                // Fires after each stats block ("continue" mid-encode, "end" when done).
                "progress" => {
                    let progress = if total_duration_us > 0 {
                        (out_time_us as f64 / total_duration_us as f64 * 100.0).min(100.0)
                    } else {
                        0.0
                    };

                    let remaining_us = total_duration_us.saturating_sub(out_time_us);

                    tracing::debug!(
                        vault_id = %vault_id,
                        progress = format_args!("{:.1}%", progress),
                        speed_x = format_args!("{:.2}x", speed),
                        bitrate_kbps,
                        "ffmpeg progress"
                    );

                    if let Some(mut item) = manager.active_items.get_mut(&vault_id) {
                        item.status = VaultItemStatus::PROCESSING;
                        item.downloaded_bytes = processed_bytes;
                        item.progress = progress;
                        item.speed_bps = (bitrate_kbps * 1_000.0 / 8.0) as i64;
                        item.eta_seconds = (speed > 0.0)
                            .then(|| (remaining_us as f64 / 1_000_000.0 / speed) as i64);
                    }
                }
                _ => {}
            }
        }

        let exit_status = child
            .wait()
            .await
            .map_err(|e| loco_err_msg!("ffmpeg wait error: {}", e))?;

        // Collect stderr now that the process has exited.
        let stderr_lines = stderr_drain.await.unwrap_or_default();

        if exit_status.success() {
            tracing::info!(vault_id = %vault_id, input=?source_path, output=?output_file, "ffmpeg finished successfully");

            let metadata = VaultMetadata {
                chapters,
                subtitles,
                fonts,
            };

            let metadata_path = encoded_dir.join("metadata.json");
            if let Ok(json_str) = serde_json::to_string_pretty(&metadata) {
                let _ = fs::write(&metadata_path, json_str).await;
            }

            // Remove empty sub/font folders if none were extracted
            let subs_dir = encoded_dir.join(SUBTITLES_LOC);
            if metadata.subtitles.is_empty() && subs_dir.is_dir() {
                let _ = fs::remove_dir_all(&subs_dir).await;
            }
            let fonts_dir = encoded_dir.join(FONTS_LOC);
            if metadata.fonts.is_empty() && fonts_dir.is_dir() {
                let _ = fs::remove_dir_all(&fonts_dir).await;
            }
            Ok(())
        } else {
            let msg = format!(
                "ffmpeg exited with code {}",
                exit_status.code().unwrap_or(-1)
            );
            tracing::error!(vault_id = %vault_id, title, "{}", msg);
            for line in &stderr_lines {
                tracing::error!(vault_id = %vault_id, "[ffmpeg stderr] {}", line);
            }
            loco_err!("{}", msg)
        }
    }
}
