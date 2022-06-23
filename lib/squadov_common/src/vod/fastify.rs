// LOL. Fastify. Sorry. :')
use crate::SquadOvError;
use tokio::process::Command;

// Converts the mp4 to have the faststart ffmpeg movflag so that users
// can start viewing the video immediately.
pub async fn fastify_mp4(input_fname: &str, container_format: &str, output_fname: &std::path::Path, output_container: &str) -> Result<(), SquadOvError> {
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
        .arg(container_format)
        .arg("-probesize")
        .arg("100M")
        .arg("-analyzeduration")
        .arg("100M")
        .arg("-i")
        .arg(input_fname)
        // The general use case of this function is to take an already encoded video (mp4) that
        // the user is streaming into Google Storage with the +empty_moov+frag_keyframe+default_base_moof
        // flags and convert it to an mp4 that has the +faststart flag.
        .arg("-c:v")
        .arg("copy")
        .arg("-c:a")
        .arg("copy")
        .arg("-max_muxing_queue_size")
        .arg("9999")
        .arg("-movflags")
        .arg("+faststart")
        .arg("-f")
        .arg(output_container)
        .arg(output_fname.as_os_str())
        .output()
        .await?;

    if !ffmpeg_output.status.success() {
        log::warn!("Failed to convert fastify mp4 with ffmpeg: {} to {}", input_fname, output_fname.display());
        log::warn!("FFMPEG STDOUT:\n {}", std::str::from_utf8(&ffmpeg_output.stdout).unwrap_or("???"));
        log::warn!("FFMPEG STDERR:\n {}", std::str::from_utf8(&ffmpeg_output.stderr).unwrap_or("???"));
        Err(SquadOvError::InternalError(String::from("FFmpeg Fastify Failure")))
    } else {
        Ok(())
    }
}