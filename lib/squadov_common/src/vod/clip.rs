use crate::SquadOvError;
use tokio::process::Command;

pub async fn generate_clip(input_fname: &str, output_fname: &std::path::Path, start: i64, end: i64) -> Result<(), SquadOvError> {
    let ffmpeg_path = std::env::var("FFMPEG_BINARY_PATH")?;
    let ffmpeg_output = Command::new(&ffmpeg_path)
        // Single threaded so that we can split our CPU bandwidth among multiple videos.
        // I didn't find this to affect encoding performance much since we're just doing
        // straight copies.
        .arg("-threads")
        .arg("1")
        // Need to auto accept overwriting existing files to prevent blocking.
        .arg("-y")
        .arg("-f")
        .arg("mp4")
        .arg("-probesize")
        .arg("100M")
        .arg("-analyzeduration")
        .arg("100M")
        .arg("-i")
        .arg(input_fname)
        .arg("-ss")
        .arg(format!("{}ms", start))
        .arg("-to")
        .arg(format!("{}ms", end))
        .arg("-c:v")
        .arg("copy")
        .arg("-c:a")
        .arg("copy")
        .arg("-max_muxing_queue_size")
        .arg("9999")
        .arg("-movflags")
        .arg("+faststart")
        .arg("-f")
        .arg("mp4")
        .arg(output_fname.as_os_str())
        .output()
        .await?;
    
    if !ffmpeg_output.status.success() {
        log::warn!("Failed to generate VOD clip with ffmpeg: {} to {}", input_fname, output_fname.display());
        log::warn!("FFMPEG STDOUT:\n {}", std::str::from_utf8(&ffmpeg_output.stdout).unwrap_or("???"));
        log::warn!("FFMPEG STDERR:\n {}", std::str::from_utf8(&ffmpeg_output.stderr).unwrap_or("???"));
        Err(SquadOvError::InternalError(String::from("FFmpeg VOD Clip Failure")))
    } else {
        Ok(())
    }
}