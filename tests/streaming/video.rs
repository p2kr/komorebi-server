use komorebi_server::{
    core::constants::ENCODED_LOC,
    models::media::MediaType,
    streaming::{processor::Streaming, video::VideoProcessor},
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

    // Should return error because root only has 0-byte file and encoded dir is excluded
    let res = Streaming::resolve_file_path(test_dir.to_str().unwrap()).await;
    assert!(
        res.is_err(),
        "Expected error when only 0-byte files or encoded files exist"
    );

    // 3. Now create a valid >0 byte file in root
    let test_dir_2 =
        std::env::temp_dir().join(format!("komorebi_test_valid_{}", uuid::Uuid::new_v4()));
    fs::create_dir_all(&test_dir_2).unwrap();
    let valid_file_2 = test_dir_2.join("valid_video.mkv");
    fs::write(&valid_file_2, b"valid video bytes").unwrap();

    let res_valid = Streaming::resolve_file_path(test_dir_2.to_str().unwrap())
        .await
        .unwrap();
    assert_eq!(res_valid.0, valid_file_2);
    assert_eq!(res_valid.1, MediaType::Anime);

    // Cleanup
    let _ = fs::remove_dir_all(&test_dir);
    let _ = fs::remove_dir_all(&test_dir_2);
}

#[tokio::test]
async fn test_find_processed_file() {
    let test_dir =
        std::env::temp_dir().join(format!("komorebi_test_find_{}", uuid::Uuid::new_v4()));
    let encoded_sub = test_dir.join(ENCODED_LOC.as_str());
    fs::create_dir_all(&encoded_sub).unwrap();

    // When encoded dir is empty
    let found = VideoProcessor::find_processed_file(test_dir.to_str().unwrap())
        .await
        .unwrap();
    assert!(found.is_none());

    // When encoded dir has non-video file
    fs::write(encoded_sub.join("notes.txt"), b"some text").unwrap();
    let found = VideoProcessor::find_processed_file(test_dir.to_str().unwrap())
        .await
        .unwrap();
    assert!(found.is_none());

    // When encoded dir has valid mp4
    let mp4_file = encoded_sub.join("output.mp4");
    fs::write(&mp4_file, b"processed video bytes").unwrap();
    let found = VideoProcessor::find_processed_file(test_dir.to_str().unwrap())
        .await
        .unwrap();
    assert_eq!(found, Some(mp4_file));

    // Cleanup
    let _ = fs::remove_dir_all(&test_dir);
}

#[test]
fn test_is_compatible_video() {
    // H.264: 8-bit is compatible; 10-bit (Hi10P) and 4:2:2 must be re-encoded
    assert!(VideoProcessor::is_compatible_video("h264", Some("yuv420p")));
    assert!(VideoProcessor::is_compatible_video("h264", Some("yuvj420p")));
    assert!(VideoProcessor::is_compatible_video("avc", Some("yuv420p")));
    assert!(!VideoProcessor::is_compatible_video("h264", Some("yuv420p10le")));
    assert!(!VideoProcessor::is_compatible_video("h264", Some("yuv422p")));
    assert!(!VideoProcessor::is_compatible_video("h264", Some("yuv444p")));
    assert!(!VideoProcessor::is_compatible_video("h264", None));

    // HEVC: 8-bit and 10-bit (Main / Main 10) are compatible; 4:2:2/4:4:4 are not
    assert!(VideoProcessor::is_compatible_video("hevc", Some("yuv420p")));
    assert!(VideoProcessor::is_compatible_video("hevc", Some("yuvj420p")));
    assert!(VideoProcessor::is_compatible_video("hevc", Some("yuv420p10le")));
    assert!(VideoProcessor::is_compatible_video("h265", Some("yuv420p10le")));
    assert!(!VideoProcessor::is_compatible_video("hevc", Some("yuv422p10le")));
    assert!(!VideoProcessor::is_compatible_video("hevc", Some("yuv444p10le")));
    assert!(!VideoProcessor::is_compatible_video("hevc", None));

    // VP9: 8-bit and 10-bit are compatible
    assert!(VideoProcessor::is_compatible_video("vp9", Some("yuv420p")));
    assert!(VideoProcessor::is_compatible_video("vp9", Some("yuv420p10le")));
    assert!(!VideoProcessor::is_compatible_video("vp9", Some("yuv444p")));

    // AV1: 8-bit and 10-bit are compatible
    assert!(VideoProcessor::is_compatible_video("av1", Some("yuv420p")));
    assert!(VideoProcessor::is_compatible_video("av1", Some("yuv420p10le")));
    assert!(!VideoProcessor::is_compatible_video("av1", Some("yuv422p10le")));

    // Unsupported codecs must always be re-encoded
    assert!(!VideoProcessor::is_compatible_video("mpeg4", Some("yuv420p")));
    assert!(!VideoProcessor::is_compatible_video("mpeg2video", Some("yuv420p")));
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
    let mut probe = ffprobe::FfProbe::default();
    probe.streams = vec![
        ffprobe::Stream {
            index: 0,
            codec_type: Some("video".into()),
            codec_name: Some("hevc".into()),
            pix_fmt: Some("yuv420p10le".into()),
            width: Some(1920),
            height: Some(1080),
            ..Default::default()
        },
        ffprobe::Stream {
            index: 1,
            codec_type: Some("audio".into()),
            codec_name: Some("aac".into()),
            ..Default::default()
        },
    ];

    let empty_titles = std::collections::HashMap::new();
    let (args, _, _) = VideoProcessor::build_ffmpeg_args(
        "input.mkv",
        "output.mp4",
        std::path::Path::new(""),
        &probe,
        &empty_titles,
    );

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
    let mut probe = ffprobe::FfProbe::default();
    probe.streams = vec![
        ffprobe::Stream {
            index: 0,
            codec_type: Some("video".into()),
            codec_name: Some("h264".into()),
            pix_fmt: Some("yuv420p10le".into()), // Hi10P requires recode
            width: Some(1920),
            height: Some(1080),
            ..Default::default()
        },
        ffprobe::Stream {
            index: 1,
            codec_type: Some("audio".into()),
            codec_name: Some("opus".into()),
            ..Default::default()
        },
    ];

    let empty_titles = std::collections::HashMap::new();
    let (args, _, _) = VideoProcessor::build_ffmpeg_args(
        "input.mkv",
        "output.mp4",
        std::path::Path::new(""),
        &probe,
        &empty_titles,
    );

    // Video must be re-encoded with libx264 to 8-bit yuv420p
    assert!(args.windows(2).any(|w| w == ["-c:v", "libx264"]));
    assert!(args.windows(2).any(|w| w == ["-pix_fmt", "yuv420p"]));
    // Audio is Opus, which can be copied directly into MP4
    assert!(args.windows(2).any(|w| w == ["-c:a", "copy"]));
}

#[test]
fn test_build_ffmpeg_args_copies_video_recodes_dts_audio() {
    let mut probe = ffprobe::FfProbe::default();
    probe.streams = vec![
        ffprobe::Stream {
            index: 0,
            codec_type: Some("video".into()),
            codec_name: Some("hevc".into()),
            pix_fmt: Some("yuv420p10le".into()),
            width: Some(1920),
            height: Some(1080),
            ..Default::default()
        },
        ffprobe::Stream {
            index: 1,
            codec_type: Some("audio".into()),
            codec_name: Some("dts".into()),
            ..Default::default()
        },
    ];

    let empty_titles = std::collections::HashMap::new();
    let (args, _, _) = VideoProcessor::build_ffmpeg_args(
        "input.mkv",
        "output.mp4",
        std::path::Path::new(""),
        &probe,
        &empty_titles,
    );

    // Video should be copied
    assert!(args.windows(2).any(|w| w == ["-c:v", "copy"]));
    assert!(args.windows(2).any(|w| w == ["-tag:v", "hvc1"]));
    // Audio must be transcoded to AAC
    assert!(args.windows(2).any(|w| w == ["-c:a", "aac"]));
}

#[test]
fn test_build_ffmpeg_args_no_audio_stream() {
    let mut probe = ffprobe::FfProbe::default();
    probe.streams = vec![ffprobe::Stream {
        index: 0,
        codec_type: Some("video".into()),
        codec_name: Some("h264".into()),
        pix_fmt: Some("yuv420p".into()),
        width: Some(1280),
        height: Some(720),
        ..Default::default()
    }];

    let empty_titles = std::collections::HashMap::new();
    let (args, _, _) = VideoProcessor::build_ffmpeg_args(
        "input.mp4",
        "output.mp4",
        std::path::Path::new(""),
        &probe,
        &empty_titles,
    );

    assert!(args.windows(2).any(|w| w == ["-c:v", "copy"]));
    assert!(args.contains(&"-an".to_string()));
}

#[test]
fn test_build_ffmpeg_args_ignores_attached_pic_video_stream() {
    let mut probe = ffprobe::FfProbe::default();
    probe.streams = vec![
        ffprobe::Stream {
            index: 0,
            codec_type: Some("video".into()),
            codec_name: Some("mjpeg".into()),
            disposition: ffprobe::Disposition {
                attached_pic: 1,
                ..Default::default()
            },
            ..Default::default()
        },
        ffprobe::Stream {
            index: 1,
            codec_type: Some("video".into()),
            codec_name: Some("av1".into()),
            pix_fmt: Some("yuv420p10le".into()),
            width: Some(1920),
            height: Some(1080),
            ..Default::default()
        },
        ffprobe::Stream {
            index: 2,
            codec_type: Some("audio".into()),
            codec_name: Some("flac".into()),
            ..Default::default()
        },
    ];

    let empty_titles = std::collections::HashMap::new();
    let (args, _, _) = VideoProcessor::build_ffmpeg_args(
        "input.mkv",
        "output.mp4",
        std::path::Path::new(""),
        &probe,
        &empty_titles,
    );

    // Should choose the main AV1 video stream and FLAC audio for copy
    assert!(args.windows(2).any(|w| w == ["-c:v", "copy"]));
    assert!(!args.contains(&"libx264".to_string()));
    assert!(args.windows(2).any(|w| w == ["-c:a", "copy"]));
}

#[tokio::test]
async fn test_extract_chapters_subtitles_fonts_metadata() {
    let test_dir = std::env::temp_dir().join(format!("komorebi_test_meta_{}", uuid::Uuid::new_v4()));
    fs::create_dir_all(&test_dir).unwrap();

    let srt_path = test_dir.join("sub.srt");
    let font_path = test_dir.join("testfont.ttf");
    let meta_path = test_dir.join("meta.txt");
    let mkv_path = test_dir.join("sample.mkv");
    let encoded_dir = test_dir.join("encoded");
    let subs_dir = encoded_dir.join("subs");
    let fonts_dir = encoded_dir.join("fonts");
    fs::create_dir_all(&encoded_dir).unwrap();
    fs::create_dir_all(&subs_dir).unwrap();
    fs::create_dir_all(&fonts_dir).unwrap();

    fs::write(&srt_path, b"1\n00:00:00,000 --> 00:00:02,000\nHello world\n").unwrap();
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
            "libx264",
            "-c:s",
            "ass",
            mkv_path.to_str().unwrap(),
        ])
        .output();

    if let Ok(out) = status
        && out.status.success()
    {
        // 1. Probe with standard ffprobe crate
        let probe = ffprobe::ffprobe_config(
            ffprobe::Config::builder().build(),
            mkv_path.as_path(),
        )
        .unwrap();

        // 2. Build args with explicit track title directly from file metadata
        let mut stream_titles = std::collections::HashMap::new();
        stream_titles.insert(1, "English".to_string());

        let output_mp4 = encoded_dir.join("video.mp4");
        let (args, subtitles, fonts) = VideoProcessor::build_ffmpeg_args(
            mkv_path.to_str().unwrap(),
            output_mp4.to_str().unwrap(),
            &encoded_dir,
            &probe,
            &stream_titles,
        );

        assert_eq!(subtitles.len(), 1);
        assert_eq!(subtitles[0].format, "ass");
        assert_eq!(subtitles[0].title, "English");
        assert_eq!(fonts.len(), 1);
        assert_eq!(fonts[0], "font_0.ttf");

        // 3. Run that exact single ffmpeg process (no extra processes created)
        let ffmpeg_out = std::process::Command::new("ffmpeg")
            .args(&args)
            .output()
            .unwrap();
        assert!(ffmpeg_out.status.success());

        // 4. Parse chapters dumped by ffmpeg during that exact same process
        let chapters_file = encoded_dir.join("chapters.txt");
        let chapters_content = fs::read_to_string(&chapters_file).unwrap();
        let chapters = VideoProcessor::parse_ffmetadata_chapters(&chapters_content);
        assert_eq!(chapters.len(), 2);
        assert_eq!(chapters[0].title, "Intro");
        assert_eq!(chapters[1].title, "Episode");

        // 5. Verify files produced by that single process
        assert!(encoded_dir.join("video.mp4").is_file());
        assert!(encoded_dir.join("subs").join("sub_0.ass").is_file());
        assert!(encoded_dir.join("fonts").join("font_0.ttf").is_file());

        let meta = komorebi_server::dtos::vault_metadata::VaultMetadata {
            chapters,
            subtitles,
            fonts,
        };

        // 6. Write and verify metadata.json
        let meta_json_path = encoded_dir.join("metadata.json");
        fs::write(
            &meta_json_path,
            serde_json::to_string_pretty(&meta).unwrap(),
        )
        .unwrap();
        assert!(meta_json_path.is_file());

        let json_content = fs::read_to_string(&meta_json_path).unwrap();
        let deserialized: komorebi_server::dtos::vault_metadata::VaultMetadata =
            serde_json::from_str(&json_content).unwrap();
        assert_eq!(deserialized, meta);
    }

    let _ = fs::remove_dir_all(&test_dir);
}

#[tokio::test]
async fn test_slime_mkv_chapters() {
    let mkv = std::path::Path::new("vault/[Ironclad] Tensei Shitara Slime Datta Ken 4 - S04E20 [WEB.1080p.AV1].mkv");
    if !mkv.exists() { return; }
    let probe = ffprobe::ffprobe_config(ffprobe::Config::builder().build(), mkv).unwrap();
    let temp = std::env::temp_dir().join(format!("slime_test_{}", uuid::Uuid::new_v4()));
    fs::create_dir_all(&temp).unwrap();
    fs::create_dir_all(temp.join("subs")).unwrap();
    fs::create_dir_all(temp.join("fonts")).unwrap();

    let titles = std::collections::HashMap::new();
    let output_mp4 = temp.join("out.mp4");
    let (args, _, _) = VideoProcessor::build_ffmpeg_args(
        mkv.to_str().unwrap(),
        output_mp4.to_str().unwrap(),
        &temp,
        &probe,
        &titles,
    );

    let status = std::process::Command::new("ffmpeg").args(&args).output().unwrap();
    assert!(status.status.success());

    let chapters_file = temp.join("chapters.txt");
    assert!(chapters_file.exists(), "chapters.txt does not exist!");
    let content = fs::read_to_string(&chapters_file).unwrap();
    let chapters = VideoProcessor::parse_ffmetadata_chapters(&content);
    assert_eq!(chapters.len(), 3);
    assert_eq!(chapters[0].title, "Scene 1");
    assert_eq!(chapters[1].title, "Intro");
    assert_eq!(chapters[2].title, "Scene 3");
    let _ = fs::remove_dir_all(&temp);
}

#[test]
fn test_parse_probe_chapters() {
    let raw: serde_json::Value = serde_json::json!({
        "chapters": [
            {
                "id": 0,
                "start_time": "0.000000",
                "end_time": "120.500000",
                "tags": { "title": "Prologue" }
            },
            {
                "id": 1,
                "start_time": "120.500000",
                "end_time": "300.000000",
                "tags": { "title": "Opening" }
            },
            {
                "id": 2,
                "start": 300000,
                "end": 600000,
                "time_base": "1/1000",
                "tags": {}
            }
        ]
    });

    let chapters = VideoProcessor::parse_probe_chapters(&raw);
    assert_eq!(chapters.len(), 3);
    assert_eq!(chapters[0].id, 0);
    assert_eq!(chapters[0].title, "Prologue");
    assert_eq!(chapters[0].start_time, 0.0);
    assert_eq!(chapters[0].end_time, 120.5);

    assert_eq!(chapters[1].id, 1);
    assert_eq!(chapters[1].title, "Opening");
    assert_eq!(chapters[1].start_time, 120.5);
    assert_eq!(chapters[1].end_time, 300.0);

    assert_eq!(chapters[2].id, 2);
    assert_eq!(chapters[2].title, "Chapter 3");
    assert_eq!(chapters[2].start_time, 300.0);
    assert_eq!(chapters[2].end_time, 600.0);
}


