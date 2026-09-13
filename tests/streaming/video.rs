use komorebi_server::{
    core::constants::{ENCODED_LOC, FONTS_LOC, SUBTITLES_LOC},
    dtos::media::MediaType,
    models::subtitle_fonts::Model as Font,
    streaming::{processor::cached_resolve_file_paths, video::VideoProcessor},
};
use std::fs;

#[tokio::test]
async fn test_resolve_file_path_skips_encoded_and_zero_bytes() {
    let test_dir = std::env::temp_dir().join(format!("komorebi_test_{}", uuid::Uuid::new_v4()));
    fs::create_dir_all(&test_dir).unwrap();

    // 1. Create a 0-byte mkv file in the root
    let zero_byte_file = test_dir.join("truncated.mkv");
    fs::write(&zero_byte_file, b"").unwrap();

    // 2. Create a valid file in encoded/ (should be ignored by resolve_file_path)
    let encoded_dir = test_dir.join(ENCODED_LOC.as_str());
    fs::create_dir_all(&encoded_dir).unwrap();
    fs::write(encoded_dir.join("encoded.mp4"), b"fake video bytes").unwrap();

    // Should return empty because root only has 0-byte file and encoded dir is excluded
    let res = cached_resolve_file_paths(test_dir.to_str().unwrap()).await;
    assert!(
        res.is_empty(),
        "Expected empty when only 0-byte files or encoded files exist"
    );

    // 3. Now create a valid >0 byte file in root (>= 1MB)
    let test_dir_2 =
        std::env::temp_dir().join(format!("komorebi_test_valid_{}", uuid::Uuid::new_v4()));
    fs::create_dir_all(&test_dir_2).unwrap();
    let valid_file_2 = test_dir_2.join("valid_video.mkv");
    fs::write(&valid_file_2, vec![0u8; 1_000_000]).unwrap();

    let res_valid = cached_resolve_file_paths(test_dir_2.to_str().unwrap()).await;
    assert_eq!(res_valid.len(), 1);
    assert_eq!(res_valid[0].0, valid_file_2);
    assert_eq!(res_valid[0].1, MediaType::Anime);

    // Cleanup
    let _ = fs::remove_dir_all(&test_dir);
    let _ = fs::remove_dir_all(&test_dir_2);
}

#[test]
fn test_is_compatible_video() {
    // H.264: 8-bit is compatible; 10-bit (Hi10P) and 4:2:2 must be re-encoded
    assert!(VideoProcessor::is_compatible_video("h264", Some("yuv420p")));
    assert!(VideoProcessor::is_compatible_video(
        "h264",
        Some("yuvj420p")
    ));
    assert!(VideoProcessor::is_compatible_video("avc", Some("yuv420p")));
    assert!(!VideoProcessor::is_compatible_video(
        "h264",
        Some("yuv420p10le")
    ));
    assert!(!VideoProcessor::is_compatible_video(
        "h264",
        Some("yuv422p")
    ));
    assert!(!VideoProcessor::is_compatible_video(
        "h264",
        Some("yuv444p")
    ));
    assert!(!VideoProcessor::is_compatible_video("h264", None));

    // HEVC: 8-bit and 10-bit (Main / Main 10) are compatible; 4:2:2/4:4:4 are not
    assert!(VideoProcessor::is_compatible_video("hevc", Some("yuv420p")));
    assert!(VideoProcessor::is_compatible_video(
        "hevc",
        Some("yuvj420p")
    ));
    assert!(VideoProcessor::is_compatible_video(
        "hevc",
        Some("yuv420p10le")
    ));
    assert!(VideoProcessor::is_compatible_video(
        "h265",
        Some("yuv420p10le")
    ));
    assert!(!VideoProcessor::is_compatible_video(
        "hevc",
        Some("yuv422p10le")
    ));
    assert!(!VideoProcessor::is_compatible_video(
        "hevc",
        Some("yuv444p10le")
    ));
    assert!(!VideoProcessor::is_compatible_video("hevc", None));

    // VP9: 8-bit and 10-bit are compatible
    assert!(VideoProcessor::is_compatible_video("vp9", Some("yuv420p")));
    assert!(VideoProcessor::is_compatible_video(
        "vp9",
        Some("yuv420p10le")
    ));
    assert!(!VideoProcessor::is_compatible_video("vp9", Some("yuv444p")));

    // AV1: 8-bit and 10-bit are compatible
    assert!(VideoProcessor::is_compatible_video("av1", Some("yuv420p")));
    assert!(VideoProcessor::is_compatible_video(
        "av1",
        Some("yuv420p10le")
    ));
    assert!(!VideoProcessor::is_compatible_video(
        "av1",
        Some("yuv422p10le")
    ));

    // Unsupported codecs must always be re-encoded
    assert!(!VideoProcessor::is_compatible_video(
        "mpeg4",
        Some("yuv420p")
    ));
    assert!(!VideoProcessor::is_compatible_video(
        "mpeg2video",
        Some("yuv420p")
    ));
    assert!(!VideoProcessor::is_compatible_video("vc1", Some("yuv420p")));
}

#[test]
fn test_is_compatible_audio() {
    // Compatible audio codecs in MP4 for modern browsers
    assert!(VideoProcessor::is_compatible_audio("aac"));
    assert!(VideoProcessor::is_compatible_audio("opus"));
    assert!(VideoProcessor::is_compatible_audio("mp3"));
    assert!(VideoProcessor::is_compatible_audio("flac"));
    assert!(VideoProcessor::is_compatible_audio("AAC"));

    // Incompatible codecs requiring re-encoding to AAC
    assert!(!VideoProcessor::is_compatible_audio("ac3"));
    assert!(!VideoProcessor::is_compatible_audio("eac3"));
    assert!(!VideoProcessor::is_compatible_audio("dts"));
    assert!(!VideoProcessor::is_compatible_audio("truehd"));
    assert!(!VideoProcessor::is_compatible_audio("vorbis"));
    assert!(!VideoProcessor::is_compatible_audio("pcm_s16le"));
}

#[test]
fn test_build_ffmpeg_args_hevc_10bit_stream_copy() {
    let probe = ffprobe::ProbeResult {
        format: None,
        streams: vec![
            ffprobe::StreamInfo {
                index: 0,
                codec_type: Some("video".into()),
                codec_name: Some("hevc".into()),
                pix_fmt: Some("yuv420p10le".into()),
                width: Some(1920),
                height: Some(1080),
                ..Default::default()
            },
            ffprobe::StreamInfo {
                index: 1,
                codec_type: Some("audio".into()),
                codec_name: Some("aac".into()),
                ..Default::default()
            },
        ],
        packets: vec![],
        frames: vec![],
        programs: vec![],
        chapters: vec![],
        error: None,
    };

    let (args, _, _) = VideoProcessor::build_ffmpeg_args(
        "input.mkv",
        "output.mp4",
        std::path::Path::new(""),
        &probe,
    );

    let args = args.build_args().expect("expected build args");

    // Both video and audio should be copied
    assert!(args.windows(2).any(|w| w == ["-c:v", "copy"]));
    assert!(args.windows(2).any(|w| w == ["-tag:v", "hvc1"]));
    assert!(args.windows(2).any(|w| w == ["-c:a", "copy"]));
    assert!(args.contains(&"-sn".to_string()));
    assert!(args.contains(&"+faststart".to_string()));
    assert_eq!(args.first().unwrap(), "-y");
    assert!(args.contains(&"output.mp4".to_string()));
}

#[test]
fn test_build_ffmpeg_args_h264_10bit_recodes_video_copies_audio() {
    let probe = ffprobe::ProbeResult {
        format: None,
        streams: vec![
            ffprobe::StreamInfo {
                index: 0,
                codec_type: Some("video".into()),
                codec_name: Some("h264".into()),
                pix_fmt: Some("yuv420p10le".into()), // Hi10P requires recode
                width: Some(1920),
                height: Some(1080),
                ..Default::default()
            },
            ffprobe::StreamInfo {
                index: 1,
                codec_type: Some("audio".into()),
                codec_name: Some("opus".into()),
                ..Default::default()
            },
        ],
        packets: vec![],
        frames: vec![],
        programs: vec![],
        chapters: vec![],
        error: None,
    };

    let (args, _, _) = VideoProcessor::build_ffmpeg_args(
        "input.mkv",
        "output.mp4",
        std::path::Path::new(""),
        &probe,
    );
    let args = args.build_args().expect("expected build args");

    // Video must be re-encoded with libx264 to 8-bit yuv420p
    assert!(args.windows(2).any(|w| w == ["-c:v", "libx264"]));
    assert!(args.windows(2).any(|w| w == ["-pix_fmt", "yuv420p"]));
    // Audio is Opus, which can be copied directly into MP4
    assert!(args.windows(2).any(|w| w == ["-c:a", "copy"]));
}

#[test]
fn test_build_ffmpeg_args_copies_video_recodes_dts_audio() {
    let probe = ffprobe::ProbeResult {
        format: None,
        streams: vec![
            ffprobe::StreamInfo {
                index: 0,
                codec_type: Some("video".into()),
                codec_name: Some("hevc".into()),
                pix_fmt: Some("yuv420p10le".into()),
                width: Some(1920),
                height: Some(1080),
                ..Default::default()
            },
            ffprobe::StreamInfo {
                index: 1,
                codec_type: Some("audio".into()),
                codec_name: Some("dts".into()),
                ..Default::default()
            },
        ],
        packets: vec![],
        frames: vec![],
        programs: vec![],
        chapters: vec![],
        error: None,
    };

    let (args, _, _) = VideoProcessor::build_ffmpeg_args(
        "input.mkv",
        "output.mp4",
        std::path::Path::new(""),
        &probe,
    );
    let args = args.build_args().expect("expected build args");

    // Video should be copied
    assert!(args.windows(2).any(|w| w == ["-c:v", "copy"]));
    assert!(args.windows(2).any(|w| w == ["-tag:v", "hvc1"]));
    // Audio must be transcoded to AAC
    assert!(args.windows(2).any(|w| w == ["-c:a", "aac"]));
}

#[test]
fn test_build_ffmpeg_args_no_audio_stream() {
    let probe = ffprobe::ProbeResult {
        format: None,
        streams: vec![ffprobe::StreamInfo {
            index: 0,
            codec_type: Some("video".into()),
            codec_name: Some("h264".into()),
            pix_fmt: Some("yuv420p".into()),
            width: Some(1280),
            height: Some(720),
            ..Default::default()
        }],
        packets: vec![],
        frames: vec![],
        programs: vec![],
        chapters: vec![],
        error: None,
    };

    let (args, _, _) = VideoProcessor::build_ffmpeg_args(
        "input.mp4",
        "output.mp4",
        std::path::Path::new(""),
        &probe,
    );
    let args = args.build_args().expect("expected build args");

    assert!(args.windows(2).any(|w| w == ["-c:v", "copy"]));
    assert!(args.contains(&"-an".to_string()));
}

#[test]
fn test_build_ffmpeg_args_ignores_attached_pic_video_stream() {
    let probe = ffprobe::ProbeResult {
        format: None,
        streams: vec![
            ffprobe::StreamInfo {
                index: 0,
                codec_type: Some("video".into()),
                codec_name: Some("mjpeg".into()),
                disposition: Some({
                    let mut m = std::collections::HashMap::new();
                    m.insert("attached_pic".to_string(), 1u8);
                    m
                }),
                ..Default::default()
            },
            ffprobe::StreamInfo {
                index: 1,
                codec_type: Some("video".into()),
                codec_name: Some("av1".into()),
                pix_fmt: Some("yuv420p10le".into()),
                width: Some(1920),
                height: Some(1080),
                ..Default::default()
            },
            ffprobe::StreamInfo {
                index: 2,
                codec_type: Some("audio".into()),
                codec_name: Some("flac".into()),
                ..Default::default()
            },
        ],
        packets: vec![],
        frames: vec![],
        programs: vec![],
        chapters: vec![],
        error: None,
    };

    let (args, _, _) = VideoProcessor::build_ffmpeg_args(
        "input.mkv",
        "output.mp4",
        std::path::Path::new(""),
        &probe,
    );
    let args = args.build_args().expect("expected build args");

    // Should choose the main AV1 video stream and FLAC audio for copy
    assert!(args.windows(2).any(|w| w == ["-c:v", "copy"]));
    assert!(!args.contains(&"libx264".to_string()));
    assert!(args.windows(2).any(|w| w == ["-c:a", "copy"]));
}

#[tokio::test]
async fn test_extract_chapters_subtitles_fonts_metadata() {
    let test_dir =
        std::env::temp_dir().join(format!("komorebi_test_meta_{}", uuid::Uuid::new_v4()));
    fs::create_dir_all(&test_dir).unwrap();

    let srt_path = test_dir.join("sub.srt");
    let font_path = test_dir.join("testfont.ttf");
    let meta_path = test_dir.join("meta.txt");
    let mkv_path = test_dir.join("sample.mkv");
    let encoded_dir = test_dir.join("encoded");
    let subs_dir = encoded_dir.join(SUBTITLES_LOC);
    let fonts_dir = encoded_dir.join(FONTS_LOC);
    fs::create_dir_all(&encoded_dir).unwrap();
    fs::create_dir_all(&subs_dir).unwrap();
    fs::create_dir_all(&fonts_dir).unwrap();

    fs::write(
        &srt_path,
        b"1\n00:00:00,000 --> 00:00:02,000\nHello world\n",
    )
    .unwrap();
    fs::write(&font_path, b"fake ttf font data").unwrap();
    fs::write(
        &meta_path,
        b";FFMETADATA1\n[CHAPTER]\nTIMEBASE=1/1000\nSTART=0\nEND=1000\ntitle=Intro\n[CHAPTER]\nTIMEBASE=1/1000\nSTART=1000\nEND=2000\ntitle=Episode\n",
    )
    .unwrap();

    // Create a sample MKV with chapters, an ASS subtitle track, and a font attachment
    let status = std::process::Command::new("ffmpeg")
        .args([
            "-y",
            "-f",
            "lavfi",
            "-i",
            "testsrc=duration=2:size=320x240:rate=1",
            "-i",
            srt_path.to_str().unwrap(),
            "-i",
            meta_path.to_str().unwrap(),
            "-map_metadata",
            "2",
            "-attach",
            font_path.to_str().unwrap(),
            "-metadata:s:t",
            "mimetype=application/x-truetype-font",
            "-c:v",
            "mpeg4",
            "-c:s",
            "ass",
            mkv_path.to_str().unwrap(),
        ])
        .output();

    if let Ok(out) = status
        && out.status.success()
    {
        // 1. Probe with rust_ffprobe
        use ffprobe::builder::FFprobeBuilder;
        let probe = FFprobeBuilder::new()
            .expect("ffprobe builder")
            .input(mkv_path.clone())
            .show_format()
            .show_streams()
            .show_chapters()
            .run()
            .await
            .expect("ffprobe run");

        let output_mp4 = encoded_dir.join("video.mp4");
        let (args, subtitles, fonts) = VideoProcessor::build_ffmpeg_args(
            mkv_path.to_str().unwrap(),
            output_mp4.to_str().unwrap(),
            &encoded_dir,
            &probe,
        );

        assert_eq!(subtitles.len(), 1);
        assert_eq!(subtitles[0].format, "ass");
        // The subtitle track title tag is set to the language/title from the container
        assert_eq!(fonts.len(), 1);
        assert_eq!(
            fonts[0],
            Font {
                font_name: "font_0.ttf".into(),
                file_path: "font_0.ttf".into(),
                ..Default::default()
            }
        );
        let args = args.build_args().expect("expected build args");

        // 3. Run that exact single ffmpeg process (no extra processes created)
        let ffmpeg_out = std::process::Command::new("ffmpeg")
            .args(&args)
            .output()
            .unwrap();
        assert!(ffmpeg_out.status.success());

        // 4. Parse chapters directly from ProbeResult
        let chapters = VideoProcessor::parse_probe_chapters(&probe);
        assert_eq!(chapters.len(), 2);
        assert_eq!(chapters[0].title, "Intro");
        assert_eq!(chapters[1].title, "Episode");

        // 5. Verify files produced by that single process
        assert!(encoded_dir.join("video.mp4").is_file());
        assert!(encoded_dir.join(SUBTITLES_LOC).join("sub_0.ass").is_file());
        assert!(encoded_dir.join(FONTS_LOC).join("font_0.ttf").is_file());
    }

    let _ = fs::remove_dir_all(&test_dir);
}

#[tokio::test]
async fn test_slime_mkv_chapters() {
    let mkv = std::path::Path::new(
        "vault/[Ironclad] Tensei Shitara Slime Datta Ken 4 - S04E20 [WEB.1080p.AV1].mkv",
    );
    if !mkv.exists() {
        return;
    }
    use ffprobe::builder::FFprobeBuilder;
    let probe = FFprobeBuilder::new()
        .expect("ffprobe builder")
        .input(mkv.to_path_buf())
        .show_format()
        .show_streams()
        .show_chapters()
        .run()
        .await
        .expect("ffprobe run");

    let chapters = VideoProcessor::parse_probe_chapters(&probe);
    assert_eq!(chapters.len(), 3);
    assert_eq!(chapters[0].title, "Scene 1");
    assert_eq!(chapters[1].title, "Intro");
    assert_eq!(chapters[2].title, "Scene 3");
}

#[test]
fn test_parse_probe_chapters() {
    use std::collections::HashMap;
    let probe = ffprobe::ProbeResult {
        format: None,
        streams: vec![],
        packets: vec![],
        frames: vec![],
        programs: vec![],
        chapters: vec![
            ffprobe::ChapterInfo {
                id: 0,
                time_base: None,
                start: 0,
                start_time: Some("0.000000".into()),
                end: 0,
                end_time: Some("120.500000".into()),
                tags: HashMap::from([("title".into(), "Prologue".into())]),
            },
            ffprobe::ChapterInfo {
                id: 1,
                time_base: None,
                start: 0,
                start_time: Some("120.500000".into()),
                end: 0,
                end_time: Some("300.000000".into()),
                tags: HashMap::from([("title".into(), "Opening".into())]),
            },
            ffprobe::ChapterInfo {
                id: 2,
                time_base: Some("1/1000".into()),
                start: 300000,
                start_time: None,
                end: 600000,
                end_time: None,
                tags: HashMap::new(),
            },
        ],
        error: None,
    };

    let chapters = VideoProcessor::parse_probe_chapters(&probe);
    assert_eq!(chapters.len(), 3);
    assert_eq!(chapters[0].chapter_id, 0);
    assert_eq!(chapters[0].title, "Prologue");
    assert_eq!(chapters[0].start_time, 0.0);
    assert_eq!(chapters[0].end_time, 120.5);

    assert_eq!(chapters[1].chapter_id, 1);
    assert_eq!(chapters[1].title, "Opening");
    assert_eq!(chapters[1].start_time, 120.5);
    assert_eq!(chapters[1].end_time, 300.0);

    assert_eq!(chapters[2].chapter_id, 2);
    assert_eq!(chapters[2].title, "Chapter 3");
    assert_eq!(chapters[2].start_time, 300.0);
    assert_eq!(chapters[2].end_time, 600.0);
}
