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
