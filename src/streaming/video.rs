use std::{path::PathBuf, str::FromStr, sync::LazyLock};

use cached::cached;
use loco_rs::prelude::*;
use tokio::{
    fs,
    io::{AsyncBufReadExt, BufReader},
    task,
};
use walkdir::WalkDir;
use which::which;

use crate::{
    core::{ResultExt, constants::ENCODED_LOC},
    downloaders::manager::DownloadManager,
    dtos::vault_metadata::{ChapterItem, SubtitleTrackItem, VaultMetadata},
    loco_err, loco_err_msg,
    models::{
        media::MediaType,
        vault::{VaultItem, VaultItemStatus},
    },
    streaming::{EXT_VS_TYPE, PostProcessor},
};

pub struct VideoProcessor {}

fn get_ffmpeg_path(mut bin_name: &str) -> PathBuf {
    if bin_name.trim().is_empty() {
        bin_name = "ffmpeg";
    }
    match which(bin_name) {
        Ok(v) => v,
        Err(e) => {
            tracing::error!("unable to find ffmpeg at PATH: {}", e);
            match which(format!("assets/{}", bin_name)) {
                Ok(v) => v,
                Err(e) => {
                    tracing::error!("unable to find ffmpeg at assets/: {}", e);
                    Default::default()
                }
            }
        }
    }
}

static FFMPEG: LazyLock<PathBuf> = LazyLock::new(|| get_ffmpeg_path("ffmpeg"));
static FFPROBE: LazyLock<PathBuf> = LazyLock::new(|| get_ffmpeg_path("ffprobe"));

#[cached(max_size = 100)]
fn cached_resolve_file_path(folder: &str) -> Result<(PathBuf, MediaType)> {
    let dir = PathBuf::from_str(folder).to_loco_err()?;
    for entry in WalkDir::new(&dir)
        .into_iter()
        .filter_entry(|v| v.file_name() != ENCODED_LOC.as_str())
        .filter_map(|v| v.ok())
    {
        let path = entry.path();
        if path.is_file() && entry.metadata().map(|m| m.len() > 0).unwrap_or(false) {
            let file_ext = path
                .extension()
                .map(|v| v.to_ascii_lowercase())
                .unwrap_or_default()
                .to_string_lossy()
                .to_lowercase();

            if EXT_VS_TYPE
                .get(file_ext.as_ref())
                .is_some_and(|v| v.eq(&MediaType::Anime))
            {
                return Ok((path.to_path_buf(), MediaType::Anime));
            }
        }
    }

    loco_err!("No video file found in the specified folder.")
}

impl VideoProcessor {
    pub async fn find_processed_file(folder: &str) -> Result<Option<PathBuf>> {
        let f = folder.to_string();
        task::spawn_blocking(move || {
            let dir = PathBuf::from_str(&f).to_loco_err()?;
            let sub_dir = dir.join(ENCODED_LOC.as_str());
            if sub_dir.is_dir() {
                for entry in WalkDir::new(&sub_dir).into_iter().filter_map(|v| v.ok()) {
                    let path = entry.path();
                    if path.is_file() && entry.metadata().map(|m| m.len() > 0).unwrap_or(false) {
                        let file_ext = path
                            .extension()
                            .map(|v| v.to_ascii_lowercase())
                            .unwrap_or_default()
                            .to_string_lossy()
                            .to_lowercase();

                        if EXT_VS_TYPE
                            .get(file_ext.as_ref())
                            .is_some_and(|v| v.eq(&MediaType::Anime))
                        {
                            return Ok(Some(path.to_path_buf()));
                        }
                    }
                }
            }
            Ok(None)
        })
        .await
        .to_loco_err()?
    }

    /// Checks whether the video codec and pixel format can be copied directly
    /// without re-encoding, targeting modern browsers (last 2 years: Chrome, Safari, Edge, Firefox).
    pub fn is_compatible_video(codec_name: &str, pix_fmt: Option<&str>) -> bool {
        let codec = codec_name.to_ascii_lowercase();
        let fmt = pix_fmt.unwrap_or("").to_ascii_lowercase();

        match codec.as_str() {
            // H.264 / AVC: 8-bit 4:2:0 is supported in all browsers.
            // 10-bit H.264 (Hi10P) has no browser hardware/software decoder and must be re-encoded to 8-bit.
            "h264" | "avc" => matches!(fmt.as_str(), "yuv420p" | "yuvj420p"),

            // HEVC (H.265): modern browsers (Chrome 107+, Safari, Edge, Firefox 120+) support 8-bit
            // and 10-bit (Main / Main 10).
            "hevc" | "h265" => matches!(fmt.as_str(), "yuv420p" | "yuvj420p" | "yuv420p10le"),

            // VP9: modern browsers support 8-bit and 10-bit (Profile 0 / Profile 2).
            "vp9" => matches!(fmt.as_str(), "yuv420p" | "yuvj420p" | "yuv420p10le"),

            // AV1: modern browsers support 8-bit and 10-bit (Main profile).
            "av1" => matches!(fmt.as_str(), "yuv420p" | "yuvj420p" | "yuv420p10le"),

            _ => false,
        }
    }

    /// Checks whether the audio codec can be copied directly into an MP4 container for modern browsers.
    pub fn is_compatible_audio(codec_name: &str) -> bool {
        let codec = codec_name.to_ascii_lowercase();
        matches!(codec.as_str(), "aac" | "opus" | "mp3" | "flac")
    }

    /// Determines the video codec arguments for FFmpeg.
    /// Prefers stream copy (`-c:v copy`), adding `-tag:v hvc1` for HEVC streams on Apple/Safari.
    /// Incompatible video streams are re-encoded to universal 8-bit H.264 (`yuv420p`).
    /// Determines the video codec arguments for FFmpeg.
    /// Prefers stream copy (`-c:v copy`), adding `-tag:v hvc1` for HEVC streams on Apple/Safari.
    /// Incompatible video streams are re-encoded to universal 8-bit H.264 (`yuv420p`).
    /// Determines the video codec arguments for FFmpeg.
    /// Prefers stream copy (`-c:v copy`), adding `-tag:v hvc1` for HEVC streams on Apple/Safari.
    /// Incompatible video streams are re-encoded to universal 8-bit H.264 (`yuv420p`).
    pub fn build_video_codec_args(probe: &ffprobe::FfProbe) -> Vec<String> {
        let primary_video = probe
            .streams
            .iter()
            .filter(|s| s.codec_type.as_deref() == Some("video") && s.disposition.attached_pic != 1)
            .max_by_key(|s| s.width.unwrap_or(0) * s.height.unwrap_or(0))
            .or_else(|| {
                probe
                    .streams
                    .iter()
                    .find(|s| s.codec_type.as_deref() == Some("video"))
            });

        let is_video_copyable = primary_video
            .and_then(|s| s.codec_name.as_deref().map(|c| (c, s.pix_fmt.as_deref())))
            .map(|(codec, pix_fmt)| Self::is_compatible_video(codec, pix_fmt))
            .unwrap_or(false);

        let is_hevc = primary_video
            .and_then(|s| s.codec_name.as_deref())
            .map(|c| matches!(c.to_ascii_lowercase().as_str(), "hevc" | "h265"))
            .unwrap_or(false);

        if is_video_copyable {
            let mut args = vec!["-c:v".into(), "copy".into()];
            if is_hevc {
                // Apple devices / Safari require the hvc1 tag for HEVC playback in MP4
                args.push("-tag:v".into());
                args.push("hvc1".into());
            }
            args
        } else {
            // CRF 18 = visually lossless; veryfast ensures speedy encode when recoding is mandatory
            [
                "-c:v", "libx264", "-crf", "18", "-preset", "veryfast", "-pix_fmt", "yuv420p",
            ]
            .iter()
            .map(|s| s.to_string())
            .collect()
        }
    }

    /// Determines the audio codec arguments for FFmpeg.
    /// Stream copies compatible audio (AAC, Opus, MP3, FLAC).
    /// If audio is incompatible (e.g. AC3, DTS, TrueHD), re-encodes to 192k AAC.
    /// If no audio stream is present, passes `-an`.
    pub fn build_audio_codec_args(probe: &ffprobe::FfProbe) -> Vec<String> {
        let has_audio = probe
            .streams
            .iter()
            .any(|s| s.codec_type.as_deref() == Some("audio"));

        if !has_audio {
            return vec!["-an".into()];
        }

        let primary_audio = probe
            .streams
            .iter()
            .filter(|s| s.codec_type.as_deref() == Some("audio"))
            .max_by_key(|s| (s.disposition.default, s.channels.unwrap_or(0)));

        let is_audio_copyable = primary_audio
            .and_then(|s| s.codec_name.as_deref())
            .map(Self::is_compatible_audio)
            .unwrap_or(false);

        if is_audio_copyable {
            vec!["-c:a".into(), "copy".into()]
        } else {
            ["-c:a", "aac", "-b:a", "192k"]
                .iter()
                .map(|s| s.to_string())
                .collect()
        }
    }

    /// Parses chapters from an FFmetadata file dumped during FFmpeg processing.
    pub fn parse_ffmetadata_chapters(content: &str) -> Vec<ChapterItem> {
        let mut chapters = Vec::new();
        let mut id = 0;
        let mut current_start: Option<f64> = None;
        let mut current_end: Option<f64> = None;
        let mut current_title: Option<String> = None;
        let mut timebase_den: f64 = 1_000_000_000.0;

        for line in content.lines() {
            let line = line.trim();
            if line == "[CHAPTER]" {
                if let (Some(start), Some(end)) = (current_start, current_end) {
                    chapters.push(ChapterItem {
                        id,
                        title: current_title.unwrap_or_else(|| format!("Chapter {}", id + 1)),
                        start_time: start,
                        end_time: end,
                    });
                    id += 1;
                }
                current_start = None;
                current_end = None;
                current_title = None;
                timebase_den = 1_000_000_000.0;
            } else if let Some((k, v)) = line.split_once('=') {
                let k = k.trim();
                let v = v.trim();
                match k {
                    "TIMEBASE" => {
                        if let Some((_, den)) = v.split_once('/') {
                            timebase_den = den.parse().unwrap_or(1_000_000_000.0);
                        }
                    }
                    "START" => {
                        let raw: f64 = v.parse().unwrap_or(0.0);
                        current_start = Some(raw / timebase_den);
                    }
                    "END" => {
                        let raw: f64 = v.parse().unwrap_or(0.0);
                        current_end = Some(raw / timebase_den);
                    }
                    "title" => {
                        current_title = Some(v.to_string());
                    }
                    _ => {}
                }
            }
        }

        if let (Some(start), Some(end)) = (current_start, current_end) {
            chapters.push(ChapterItem {
                id,
                title: current_title.unwrap_or_else(|| format!("Chapter {}", id + 1)),
                start_time: start,
                end_time: end,
            });
        }

        chapters
    }

    /// Parses chapters directly from ffprobe raw JSON output (`-show_chapters`).
    pub fn parse_probe_chapters(raw_json: &serde_json::Value) -> Vec<ChapterItem> {
        raw_json["chapters"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .enumerate()
                    .filter_map(|(idx, ch)| {
                        let start_time = ch["start_time"]
                            .as_str()
                            .and_then(|s| s.parse::<f64>().ok())
                            .or_else(|| ch["start_time"].as_f64())
                            .or_else(|| {
                                let start = ch["start"].as_f64()?;
                                let time_base = ch["time_base"].as_str()?;
                                let (num, den) = time_base.split_once('/')?;
                                let scale = num.parse::<f64>().ok()? / den.parse::<f64>().ok()?;
                                Some(start * scale)
                            })?;

                        let end_time = ch["end_time"]
                            .as_str()
                            .and_then(|s| s.parse::<f64>().ok())
                            .or_else(|| ch["end_time"].as_f64())
                            .or_else(|| {
                                let end = ch["end"].as_f64()?;
                                let time_base = ch["time_base"].as_str()?;
                                let (num, den) = time_base.split_once('/')?;
                                let scale = num.parse::<f64>().ok()? / den.parse::<f64>().ok()?;
                                Some(end * scale)
                            })?;

                        let title = ch["tags"]["title"]
                            .as_str()
                            .map(|s| s.trim().to_string())
                            .filter(|s| !s.is_empty())
                            .or_else(|| {
                                ch["tags"]["TITLE"]
                                    .as_str()
                                    .map(|s| s.trim().to_string())
                                    .filter(|s| !s.is_empty())
                            })
                            .unwrap_or_else(|| format!("Chapter {}", idx + 1));

                        Some(ChapterItem {
                            id: idx as i64,
                            title,
                            start_time,
                            end_time,
                        })
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Assembles the complete FFmpeg command arguments for post-processing in a SINGLE process.
    /// In one invocation:
    /// - Dumps font attachments to `fonts/` via input option `-dump_attachment:t:{idx}`.
    /// - Transcodes/stream-copies video and audio to faststart MP4.
    /// - Dumps chapters metadata to `chapters.txt` via `-map_metadata 0 -f ffmetadata`.
    /// - Extracts all raw subtitle streams into `subs/` via `-map 0:s:{idx} -c:s copy`.
    /// Returns the command arguments, the subtitle tracks list, and font filenames.
    pub fn build_ffmpeg_args(
        input: &str,
        output_mp4: &str,
        encoded_dir: &std::path::Path,
        probe: &ffprobe::FfProbe,
        stream_titles: &std::collections::HashMap<i64, String>,
    ) -> (Vec<String>, Vec<SubtitleTrackItem>, Vec<String>) {
        let mut args: Vec<String> = vec!["-y".into()];

        // 1. Font attachments dump (input options before -i)
        let mut fonts = Vec::new();
        let attachment_streams: Vec<(usize, &ffprobe::Stream)> = probe
            .streams
            .iter()
            .filter(|s| s.codec_type.as_deref() == Some("attachment"))
            .enumerate()
            .collect();

        if !attachment_streams.is_empty() {
            let fonts_dir = encoded_dir.join("fonts");
            for (att_idx, stream) in attachment_streams {
                let ext = stream
                    .codec_name
                    .as_deref()
                    .unwrap_or("ttf")
                    .to_ascii_lowercase();

                let font_name = format!("font_{}.{}", att_idx, ext);
                let font_path = fonts_dir.join(&font_name);

                args.push(format!("-dump_attachment:t:{}", att_idx));
                args.push(font_path.to_string_lossy().to_string());
                fonts.push(font_name);
            }
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
            .iter()
            .map(|s| s.to_string()),
        );
        args.push(output_mp4.to_string());

        // 4. Dump chapters via ffmetadata in the same process
        let chapters_txt = encoded_dir.join("chapters.txt");
        args.push("-map_metadata".into());
        args.push("0".into());
        args.push("-f".into());
        args.push("ffmetadata".into());
        args.push(chapters_txt.to_string_lossy().to_string());

        // 5. Secondary outputs: Subtitle tracks extracted in the same process
        let mut subtitles = Vec::new();
        let sub_streams: Vec<(usize, &ffprobe::Stream)> = probe
            .streams
            .iter()
            .filter(|s| s.codec_type.as_deref() == Some("subtitle"))
            .enumerate()
            .collect();

        if !sub_streams.is_empty() {
            let subs_dir = encoded_dir.join("subs");
            for (track_idx, stream) in sub_streams {
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

                let file_name = format!("sub_{}.{}", track_idx, format);
                let sub_path = subs_dir.join(&file_name);

                let lang = stream
                    .tags
                    .as_ref()
                    .and_then(|t| t.language.clone())
                    .unwrap_or_else(|| "und".into());

                let title = stream_titles
                    .get(&stream.index)
                    .cloned()
                    .or_else(|| {
                        stream
                            .tags
                            .as_ref()
                            .and_then(|t| t.handler_name.clone())
                            .filter(|h| !h.trim().is_empty() && !h.eq_ignore_ascii_case("SubtitleHandler"))
                    })
                    .unwrap_or_else(|| format!("Subtitle {}", track_idx + 1));

                args.push("-map".into());
                args.push(format!("0:s:{}", track_idx));
                args.push("-c:s".into());
                args.push("copy".into());
                args.push(sub_path.to_string_lossy().to_string());

                subtitles.push(SubtitleTrackItem {
                    track: track_idx,
                    language: lang,
                    title,
                    format: format.to_string(),
                    file_name,
                });
            }
        }

        (args, subtitles, fonts)
    }
}

impl PostProcessor for VideoProcessor {
    async fn resolve_file_path(folder: &str) -> Result<(PathBuf, MediaType)> {
        let f = folder.to_string();
        task::spawn_blocking(move || cached_resolve_file_path(&f))
            .await
            .to_loco_err()?
    }

    async fn post_process(
        file_path: PathBuf,
        manager: &DownloadManager,
        mut item: VaultItem,
    ) -> Result<()> {
        // Mark as PROCESSING before handing off to ffmpeg.
        item.status = VaultItemStatus::PROCESSING;
        item.error_msg = None;
        manager.active_items.insert(item.id, item.clone());
        manager.wake_daemon();

        let vault_id = item.id;
        let dest_path = item.destination_path.clone();
        let title = item.title.clone();

        tracing::info!(vault_id = %vault_id, title, dest_path, "post_process started");

        tracing::debug!(vault_id = %vault_id, path = %file_path.display(), "resolved video file");

        let input = file_path
            .to_str()
            .ok_or(loco_err_msg!("cannot convert input file path to string"))?
            .to_owned();

        let stem = file_path
            .file_stem()
            .ok_or(loco_err_msg!("unable to extract file stem"))?;

        let mut new_file = PathBuf::from(&item.destination_path);
        new_file.push(ENCODED_LOC.as_str());

        let encoded_dir = new_file.clone();

        // Truncate encoded dir and prepare subs/fonts folders before spawning ffmpeg.
        if new_file.is_dir() {
            fs::remove_dir_all(&new_file).await?;
        }
        fs::create_dir_all(&new_file).await?;
        fs::create_dir_all(encoded_dir.join("subs")).await?;
        fs::create_dir_all(encoded_dir.join("fonts")).await?;

        new_file.push(stem);
        new_file.set_extension("mp4");

        let output = new_file
            .to_str()
            .ok_or(loco_err_msg!("cannot convert output file path to string"))?
            .to_owned();

        let probe_output = tokio::process::Command::new(FFPROBE.as_path())
            .args(["-v", "quiet", "-print_format", "json", "-show_format", "-show_streams", "-show_chapters"])
            .arg(file_path.as_os_str())
            .output()
            .await
            .map_err(|e| loco_err_msg!("failed to run ffprobe: {}", e))?;

        if !probe_output.status.success() {
            return loco_err!("ffprobe failed with exit code {:?}", probe_output.status.code());
        }

        let probe_res: ffprobe::FfProbe = serde_json::from_slice(&probe_output.stdout)
            .map_err(|e| loco_err_msg!("failed to parse ffprobe json: {}", e))?;

        // Extract subtitle stream titles directly from the raw JSON (ffprobe crate drops tags.title)
        let raw_json: serde_json::Value = serde_json::from_slice(&probe_output.stdout).unwrap_or_default();
        let stream_titles: std::collections::HashMap<i64, String> = raw_json["streams"]
            .as_array()
            .map(|streams| {
                streams
                    .iter()
                    .filter_map(|s| {
                        let idx = s["index"].as_i64()?;
                        let title = s["tags"]["title"].as_str()?.trim();
                        if !title.is_empty() {
                            Some((idx, title.to_string()))
                        } else {
                            None
                        }
                    })
                    .collect()
            })
            .unwrap_or_default();

        // Extract chapters directly from ffprobe raw JSON
        let mut chapters = Self::parse_probe_chapters(&raw_json);
        tracing::debug!(vault_id = %vault_id, chapter_count = chapters.len(), "probed container chapters");

        // Total duration in microseconds for progress % calculation.
        let total_duration_us: u64 = probe_res
            .format
            .get_duration()
            .map(|d| d.as_micros() as u64)
            .unwrap_or_else(|| {
                tracing::warn!(vault_id = %vault_id, "ffprobe returned no duration; progress % will be 0");
                0
            });

        let (args, subtitles, fonts) =
            Self::build_ffmpeg_args(&input, &output, &encoded_dir, &probe_res, &stream_titles);

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
            tracing::info!(vault_id = %vault_id, input=?file_path, output=?new_file, "ffmpeg finished successfully");

            // If probe chapters was empty, fall back to chapters.txt dumped by ffmpeg
            let chapters_file = encoded_dir.join("chapters.txt");
            if chapters.is_empty() && chapters_file.is_file() {
                if let Ok(content) = fs::read_to_string(&chapters_file).await {
                    chapters = Self::parse_ffmetadata_chapters(&content);
                }
            }
            let _ = fs::remove_file(&chapters_file).await;

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
            let subs_dir = encoded_dir.join("subs");
            if metadata.subtitles.is_empty() && subs_dir.is_dir() {
                let _ = fs::remove_dir_all(&subs_dir).await;
            }
            let fonts_dir = encoded_dir.join("fonts");
            if metadata.fonts.is_empty() && fonts_dir.is_dir() {
                let _ = fs::remove_dir_all(&fonts_dir).await;
            }

            if let Some(mut item) = manager.active_items.get_mut(&vault_id) {
                item.temp_path = new_file.to_str().map(|x| x.to_owned());
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
