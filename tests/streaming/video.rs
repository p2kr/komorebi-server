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

    let args = VideoProcessor::build_ffmpeg_args("input.mkv", "output.mp4", &probe);

    // Both video and audio should be copied
    assert!(args.windows(2).any(|w| w == ["-c:v", "copy"]));
    assert!(args.windows(2).any(|w| w == ["-tag:v", "hvc1"]));
    assert!(args.windows(2).any(|w| w == ["-c:a", "copy"]));
    assert!(args.contains(&"-sn".to_string()));
    assert!(args.contains(&"+faststart".to_string()));
    assert_eq!(args.first().unwrap(), "-y");
    assert_eq!(args.last().unwrap(), "output.mp4");
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

    let args = VideoProcessor::build_ffmpeg_args("input.mkv", "output.mp4", &probe);

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

    let args = VideoProcessor::build_ffmpeg_args("input.mkv", "output.mp4", &probe);

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

    let args = VideoProcessor::build_ffmpeg_args("input.mp4", "output.mp4", &probe);

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

    let args = VideoProcessor::build_ffmpeg_args("input.mkv", "output.mp4", &probe);

    // Should choose the main AV1 video stream and FLAC audio for copy
    assert!(args.windows(2).any(|w| w == ["-c:v", "copy"]));
    assert!(!args.contains(&"libx264".to_string()));
    assert!(args.windows(2).any(|w| w == ["-c:a", "copy"]));
}

