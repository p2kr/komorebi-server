use std::{fs, path::PathBuf, str::FromStr};

use ffmpeg_sidecar::{
    command::ffmpeg_is_installed,
    download::{download_ffmpeg_package, ffmpeg_download_url, unpack_ffmpeg},
};
use walkdir::WalkDir;
use which::which;

fn main() {
    println!("cargo:rerun-if-changed=.env");

    // Loads the .env file and passes variables to your app
    dotenv_build::output(dotenv_build::Config::default()).unwrap();

    // download ffmpeg binary to assets folder
    let path = PathBuf::from_str("assets/").unwrap();
    fs::create_dir_all(&path).unwrap();

    if !ffmpeg_is_installed() {
        let archive: PathBuf = match find_ffmpeg_archive(&path) {
            Some(p) => p,
            _ => download_ffmpeg_package(ffmpeg_download_url().unwrap(), &path).unwrap(),
        };
        unpack_ffmpeg(&archive, &path).unwrap();
    }

    assert!(
        ffmpeg_is_installed()
            || which("ffmpeg").is_ok()
            || which("assets/ffmpeg").is_ok()
            || which("ffprobe").is_ok()
            || which("assets/ffprobe").is_ok()
    );
}

/// Helper function to find an existing ffmpeg archive
fn find_ffmpeg_archive(dir: &PathBuf) -> Option<PathBuf> {
    let entries = WalkDir::new(dir);

    for entry in entries.into_iter().filter_map(Result::ok) {
        let path = entry.path();
        if path.is_file()
            && let Some(name) = path.file_name().and_then(|n| n.to_str())
        {
            let name = name.to_ascii_lowercase();
            // Check if it matches ffmpeg-* and has a valid archive extension
            if name.starts_with("ffmpeg-") && (name.ends_with(".zip") || name.ends_with(".tar.xz"))
            {
                return Some(path.to_path_buf());
            }
        }
    }
    None
}
