use std::collections::HashMap;
use std::time::Duration;

use crate::cache::{CacheError, CacheItem};
use serde::Deserialize;
use snafu::{self, ResultExt, Snafu};
use tokio::sync::mpsc::{self};
use tokio::sync::oneshot;
use tracing::{info, trace};

mod cache;

#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum YtdlpWrapperError {
    #[snafu(display("YTDLP wrapper send error: {source}"))]
    Send {
        source: tokio::sync::mpsc::error::SendError<YtdlpCommand>,
    },
    #[snafu(display("YTDLP wrapper callback error: {source}"))]
    Callback {
        source: tokio::sync::oneshot::error::RecvError,
    },
    #[snafu(display("YTDLP wrapper command error: {source}"))]
    Command { source: std::io::Error },
    #[snafu(display("YTDLP wrapper parse error: {source}"))]
    Parse { source: serde_json::Error },
    #[snafu(display("YTDLP wrapper cache error: {source}"))]
    Cache { source: crate::cache::CacheError },
    #[snafu(display("YTDLP wrapper no stream url found error"))]
    NoUrlFound,
    #[snafu(display("Unsupported media type: {media_type}"))]
    MediaType { media_type: String },
    #[snafu(display("Invalid object for media type {media_type}: {err}"))]
    MediaTypeObject { media_type: String, err: String },
}

type Result<T> = std::result::Result<T, YtdlpWrapperError>;

/// Request that takes a youtube video url and returns the actual URL to stream the video from
pub struct GetInfoRequest {
    /// The youtube URL
    video_url: String,
    format_selection: Option<String>,
    /// Will be used to return the stream URL
    callback: oneshot::Sender<Result<YtReply>>,
}

pub enum YtdlpCommand {
    /// Request that takes a youtube video url and returns the actual URL to stream the video from
    GetVideo(GetInfoRequest),
    CleanCache,
}

#[derive(Clone, Debug)]
pub enum YtReply {
    Livestream {
        url: String,
    },
    Video {
        video_url: String,
        audio_url: String,
        http_headers: Vec<(String, String)>,
    },
}

#[derive(Clone)]
pub struct YtdlpWrapper {
    sender: mpsc::UnboundedSender<YtdlpCommand>,
}

fn wrap_media_error(media_type: &str) -> impl Fn(&str) -> YtdlpWrapperError {
    |x| YtdlpWrapperError::MediaTypeObject {
        media_type: media_type.to_string(),
        err: x.to_string(),
    }
}

impl YtdlpWrapper {
    pub fn new() -> Self {
        let (sender, mut receiver) = mpsc::unbounded_channel();
        let instance = Self { sender };

        let evictor_instance = instance.clone();
        let evictor_handle = tokio::spawn(async move {
            loop {
                if let Err(err) = evictor_instance.sender.send(YtdlpCommand::CleanCache) {
                    trace!(
                        "Failed to run cache evition from YtdlpWrapper cache: {}",
                        err
                    )
                }
                tokio::time::sleep(Duration::from_secs(5)).await;
            }
        });

        let message_instance = instance.clone();
        tokio::task::spawn(async move {
            let mut cache: HashMap<String, CacheItem<YtReply>> = HashMap::new();
            while let Some(command) = receiver.recv().await {
                let cmd = tokio::process::Command::new("yt-dlp");
                match command {
                    YtdlpCommand::GetVideo(request) => {
                        let info = message_instance
                            .get_video_info(
                                &request.video_url,
                                request.format_selection.as_deref(),
                                cmd,
                                &mut cache,
                            )
                            .await
                            .map(|x| x.item.clone());

                        if let Err(err) = request.callback.send(info) {
                            trace!("Sender dropped result: {:?}", err)
                        };
                    }
                    YtdlpCommand::CleanCache => {
                        let now = std::time::SystemTime::now();
                        cache = cache
                            .into_iter()
                            .filter(|x| !x.1.is_expired(&now))
                            .collect();
                    }
                }
            }
            evictor_handle.abort();
        });

        instance
    }

    pub async fn get_stream_info(
        &self,
        video_url: String,
        format_selection: Option<String>,
    ) -> Result<YtReply> {
        let (sender, receiver) = tokio::sync::oneshot::channel::<Result<YtReply>>();
        self.sender
            .send(YtdlpCommand::GetVideo(GetInfoRequest {
                video_url,
                format_selection,
                callback: sender,
            }))
            .context(SendSnafu)?;
        receiver.await.context(CallbackSnafu)?
    }

    async fn get_video_info<'l>(
        &self,
        video_url: &str,
        format_selection: Option<&str>,
        mut cmd: tokio::process::Command,
        cache: &'l mut HashMap<String, CacheItem<YtReply>>,
    ) -> Result<&'l CacheItem<YtReply>> {
        let key = format!(
            "{}{}",
            video_url,
            format_selection.unwrap_or(&String::with_capacity(0))
        );
        if cache.contains_key(&key) {
            return Ok(&cache[&key]);
        }
        tracing::info!("Starting yt-dlp for URL: {}", video_url);
        if let Some(fmt) = format_selection {
            cmd.args(["-f", fmt]);
        }
        let output = cmd
            .args(["--dump-single-json", video_url])
            .output()
            .await
            .context(CommandSnafu)?;

        if !output.status.success() {
            return Err(wrap_media_error("unknown")(&format!(
                "YT-dlp command failed to complete: {}",
                std::str::from_utf8(output.stderr.as_ref()).unwrap()
            )));
        }

        let dto_raw: serde_json::Value =
            serde_json::from_slice(&output.stdout).context(ParseSnafu)?;

        let raw_media_type = dto_raw["media_type"]
            .as_str()
            .ok_or(wrap_media_error("unknown")("x.media_type is missing"))?
            .to_string();

        let wrap_err = wrap_media_error(&raw_media_type);
        let dto = match raw_media_type.as_str() {
            "livestream" => {
                let url = dto_raw["url"]
                    .as_str()
                    .ok_or(wrap_err("x.url is missing"))?;
                YtReply::Livestream {
                    url: url.to_string(),
                }
            }
            "video" => {
                let requested_formats = dto_raw["requested_formats"]
                    .as_array()
                    .ok_or(wrap_err("x.requested_formats is missing"))?;
                let video_format = requested_formats
                    .iter()
                    .find(|f| f["vcodec"].as_str().map(|v| v != "none").unwrap_or(false))
                    .ok_or(wrap_err("x.requested_formats[_].vcodec is missing"))?;
                let video_url = video_format["url"]
                    .as_str()
                    .ok_or(wrap_err(
                        "x.requested_formats[_].vcodec?.url (Video URL) is missing",
                    ))?
                    .to_string();
                let http_headers = video_format["http_headers"]
                    .as_object()
                    .ok_or(wrap_err(
                        "x.requested_formats[_].vcodec?.http_headers is missing",
                    ))?
                    .iter()
                    .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                    .collect::<Vec<(String, String)>>();

                let audio_url = requested_formats
                    .iter()
                    .find(|f| f["acodec"].as_str().map(|a| a != "none").unwrap_or(false))
                    .map(|f| f["url"].as_str())
                    .flatten()
                    .ok_or(wrap_err(
                        "x.requested_formats[_].acodec?.url (Audio URL) is missing",
                    ))?
                    .to_string();
                YtReply::Video {
                    video_url,
                    audio_url,
                    http_headers,
                }
            }
            _ => {
                return Err(wrap_err("Unsupported media type"));
            }
        };

        let cached_item =
            CacheItem::with_duration(dto, Duration::from_secs(30)).context(CacheSnafu)?;
        cache.insert(key.to_owned(), cached_item);
        if let Some(item) = cache.get(&key) {
            return Ok(item);
        };
        Err(CacheError::RoundtripFailed).context(CacheSnafu)
    }
}
