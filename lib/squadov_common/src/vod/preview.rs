use crate::SquadOvError;
use tokio::process::Command;

// Generate a (hopefully) relevant clip for use as the VOD's preview.
pub async fn generate_vod_preview(input_fname: &str, output_fname: &std::path::Path) -> Result<(), SquadOvError> {
    let ffmpeg_path = std::env::var("FFMPEG_BINARY_PATH")?;
    let ffmpeg_output = if cfg!(unix) {
        Command::new(&ffmpeg_path)
            // Single threaded so that we can split our CPU bandwidth among multiple videos.
            // I didn't find this to affect encoding performance much since we're just doing
            // straight copies.
            .arg("-threads")
            .arg("1")
            // Need to auto accept overwriting existing files to prevent blocking.
            .arg("-y")
            // TODO: smarter choosing of the clip timing
            .arg("-sseof")
            .arg("-30")
            .arg("-t")
            .arg("25")
            .arg("-f")
            .arg("mp4")
            .arg("-i")
            .arg(input_fname)
            .arg("-vf")
            .arg("fps=fps=25,scale=320:-1,pad=ceil(iw/2)*2:ceil(ih/2)*2")
            .arg("-c:v")
            .arg("h264")
            .arg("-crf")
            .arg("28")
            .arg("-an")
            .arg("-movflags")
            .arg("+faststart")
            .arg("-f")
            .arg("mp4")
            .arg(output_fname.as_os_str())
            .output()
            .await?
    } else {
        Command::new(&ffmpeg_path)
            // Single threaded so that we can split our CPU bandwidth among multiple videos.
            // I didn't find this to affect encoding performance much since we're just doing
            // straight copies.
            .arg("-threads")
            .arg("1")
            // Need to auto accept overwriting existing files to prevent blocking.
            .arg("-y")
            // TODO: smarter choosing of the clip timing
            .arg("-sseof")
            .arg("-30")
            .arg("-t")
            .arg("25")
            .arg("-f")
            .arg("mp4")
            .arg("-i")
            .arg(input_fname)
            .arg("-vf")
            .arg("fps=fps=25,scale=320:-1,pad=ceil(iw/2)*2:ceil(ih/2)*2")
            .arg("-c:v")
            .arg("h264")
            .arg("-an")
            .arg("-movflags")
            .arg("+faststart")
            .arg("-f")
            .arg("mp4")
            .arg(output_fname.as_os_str())
            .output()
            .await?
    };

    if !ffmpeg_output.status.success() {
        log::warn!("Failed to convert generate VOD preview with ffmpeg: {} to {}", input_fname, output_fname.display());
        log::warn!("FFMPEG STDOUT:\n {}", std::str::from_utf8(&ffmpeg_output.stdout).unwrap_or("???"));
        log::warn!("FFMPEG STDERR:\n {}", std::str::from_utf8(&ffmpeg_output.stderr).unwrap_or("???"));
        Err(SquadOvError::InternalError(String::from("FFmpeg VOD Preview Failure")))
    } else {
        Ok(())
    }
}