use std::{path::PathBuf, str::FromStr, sync::LazyLock};

use cached::cached;
use ffprobe::{Config, ffprobe_config};
use loco_rs::prelude::*;
use tokio::{
    fs,
    io::{AsyncBufReadExt, BufReader},
    task,
};
use walkdir::WalkDir;
use which::which;

use crate::{
    core::ResultExt,
    downloaders::manager::DownloadManager,
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
        .filter_entry(|v| v.file_name() != "temp")
        .filter_map(|v| v.ok())
    {
        let path = entry.path();
        if path.is_file() {
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
        new_file.push("temp");

        //  Truncate temp dir.
        if new_file.is_dir() {
            fs::remove_dir_all(&new_file).await?;
        }
        fs::create_dir_all(&new_file).await?;

        new_file.push(stem);
        new_file.set_extension("mp4"); // TODO: Decide between mp4 and webm

        let output = new_file
            .to_str()
            .ok_or(loco_err_msg!("cannot convert output file path to string"))?
            .to_owned();

        let probe_res = ffprobe_config(
            Config::builder().ffprobe_bin(FFPROBE.as_path()).build(),
            file_path.as_path(),
        )
        .to_loco_err()?;

        // Total duration in microseconds for progress % calculation.
        let total_duration_us: u64 = probe_res
            .format
            .get_duration()
            .map(|d| d.as_micros() as u64)
            .unwrap_or_else(|| {
                tracing::warn!(vault_id = %vault_id, "ffprobe returned no duration; progress % will be 0");
                0
            });

        // Default to "needs recode" -- cleared when the codec is already compatible.
        let mut needs_video_recode = true;
        let mut needs_audio_recode = true;

        const COMPATIBLE_VIDEO: &[&str] = &["h264", "vp9", "av1"];
        const COMPATIBLE_AUDIO: &[&str] = &["aac", "opus", "mp3", "flac"];

        for stream in probe_res.streams {
            if stream.codec_type == Some("video".into())
                && let Some(ref name) = stream.codec_name
                && COMPATIBLE_VIDEO.contains(&name.as_str())
                && stream.pix_fmt == Some("yuv420p".into())
            {
                needs_video_recode = false;
            }
            if stream.codec_type == Some("audio".into())
                && let Some(ref name) = stream.codec_name
                && COMPATIBLE_AUDIO.contains(&name.as_str())
            {
                needs_audio_recode = false;
            }
        }

        tracing::debug!(
            vault_id = %vault_id,
            needs_video_recode,
            needs_audio_recode,
            "codec decision"
        );

        let video_codec: &[&str] = if needs_video_recode {
            // CRF 18 = visually lossless; preset slow gives ~15% smaller file vs fast at same CRF.
            // pix_fmt yuv420p is required for playback in Safari, QuickTime, iOS, and most
            // consumer players — without it 10-bit sources silently fail in those contexts.
            &[
                "-c:v", "libx264", "-crf", "18", "-preset", "veryfast", "-pix_fmt", "yuv420p",
            ]
        } else {
            &["-c:v", "copy"]
        };

        let audio_codec: &[&str] = if needs_audio_recode {
            &["-c:a", "aac", "-b:a", "192k"]
        } else {
            &["-c:a", "copy"]
        };

        // -movflags +faststart  moves moov atom to the front so the player can
        //                       start streaming before the full file is downloaded.
        // -progress pipe:1      streams key=value stats to stdout every 2 s.
        // -nostats / -stats_period  suppress the default stderr progress line.
        let mut args: Vec<String> = vec!["-y".into(), "-i".into(), input];
        args.extend(video_codec.iter().map(|s| s.to_string()));
        args.extend(audio_codec.iter().map(|s| s.to_string()));
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
        args.push(output.clone());

        tracing::info!(vault_id = %vault_id, output, "spawning ffmpeg");

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
