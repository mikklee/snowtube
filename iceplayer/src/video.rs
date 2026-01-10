use crate::Error;
use gstreamer as gst;
use gstreamer_app as gst_app;
use gstreamer_app::prelude::*;
use gstreamer_video::VideoMeta;
use iced::widget::image as img;
use std::num::NonZeroU8;
use std::ops::{Deref, DerefMut};
use std::os::unix::io::IntoRawFd;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};

/// Get the best available video converter element name.
/// Prefers hardware-accelerated conversion: NVIDIA (nvvideoconvert), VA-API (vapostproc),
/// falling back to software (videoconvert).
fn get_best_video_convert() -> &'static str {
    if gst::ElementFactory::find("nvvideoconvert").is_some() {
        tracing::info!(
            "Using nvvideoconvert (NVIDIA CUDA) for hardware-accelerated video conversion"
        );
        "nvvideoconvert"
    } else if gst::ElementFactory::find("vapostproc").is_some() {
        tracing::info!("Using vapostproc (VA-API) for hardware-accelerated video conversion");
        "vapostproc"
    } else {
        tracing::info!("Using videoconvert (software) for video conversion");
        "videoconvert"
    }
}

/// Position in the media.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Position {
    /// Position based on time.
    ///
    /// Not the most accurate format for videos.
    Time(Duration),
    /// Position based on nth frame.
    Frame(u64),
}

impl From<Position> for gst::GenericFormattedValue {
    fn from(pos: Position) -> Self {
        match pos {
            Position::Time(t) => gst::ClockTime::from_nseconds(t.as_nanos() as _).into(),
            Position::Frame(f) => gst::format::Default::from_u64(f).into(),
        }
    }
}

impl From<Duration> for Position {
    fn from(t: Duration) -> Self {
        Position::Time(t)
    }
}

impl From<u64> for Position {
    fn from(f: u64) -> Self {
        Position::Frame(f)
    }
}

#[derive(Debug)]
pub(crate) struct Frame(gst::Sample);

impl Frame {
    pub fn empty() -> Self {
        Self(gst::Sample::builder().build())
    }

    pub fn readable(&self) -> Option<gst::BufferMap<'_, gst::buffer::Readable>> {
        self.0.buffer().and_then(|x| x.map_readable().ok())
    }

    /// Get the Y-plane stride (line pitch) in bytes from the frame's VideoMeta.
    /// This is critical for proper NV12 decoding, as the stride may differ from width.
    pub fn stride(&self) -> Option<u32> {
        self.0.buffer().and_then(|buffer| {
            buffer
                .meta::<VideoMeta>()
                .map(|meta| meta.stride()[0] as u32)
        })
    }
}

/// Number of frequency bands for audio spectrum analysis.
pub const SPECTRUM_BANDS: usize = 64;

#[derive(Debug)]
pub(crate) struct Internal {
    pub(crate) id: u64,

    pub(crate) bus: gst::Bus,
    pub(crate) source: gst::Pipeline,
    pub(crate) alive: Arc<AtomicBool>,
    pub(crate) worker: Option<std::thread::JoinHandle<()>>,

    pub(crate) width: i32,
    pub(crate) height: i32,
    pub(crate) framerate: f64,
    pub(crate) duration: Duration,
    pub(crate) speed: f64,
    pub(crate) sync_av: bool,

    pub(crate) frame: Arc<Mutex<Frame>>,
    pub(crate) upload_frame: Arc<AtomicBool>,
    pub(crate) last_frame_time: Arc<Mutex<Instant>>,
    pub(crate) looping: bool,
    pub(crate) is_eos: bool,
    pub(crate) restart_stream: bool,
    pub(crate) sync_av_avg: u64,
    pub(crate) sync_av_counter: u64,

    pub(crate) subtitle_text: Arc<Mutex<Option<String>>>,
    pub(crate) upload_text: Arc<AtomicBool>,

    /// Child process for yt-dlp streaming (if using from_ytdlp)
    pub(crate) ytdlp_process: Option<Child>,

    /// Audio spectrum data (frequency magnitudes in dB, normalized 0.0-1.0).
    pub(crate) spectrum: Arc<Mutex<[f32; SPECTRUM_BANDS]>>,
}

impl Internal {
    pub(crate) fn seek(&self, position: impl Into<Position>, accurate: bool) -> Result<(), Error> {
        let position = position.into();

        // Query if the pipeline is seekable
        let mut query = gst::query::Seeking::new(gst::Format::Time);
        if self.source.query(&mut query) {
            let (seekable, start, end) = query.result();
            tracing::info!(
                "Seek query: seekable={}, start={:?}, end={:?}",
                seekable,
                start,
                end
            );
            if !seekable {
                tracing::warn!("Pipeline reports as not seekable");
            }
        } else {
            tracing::warn!("Seeking query failed");
        }

        // Don't use KEY_UNIT - it causes audio/video desync with separate streams
        // because video keyframes and audio frames don't align
        let seek_flags = gst::SeekFlags::FLUSH
            | if accurate {
                gst::SeekFlags::ACCURATE
            } else {
                gst::SeekFlags::empty()
            };

        // gstreamer complains if the start & end value types aren't the same
        match &position {
            Position::Time(_) => self.source.seek(
                self.speed,
                seek_flags,
                gst::SeekType::Set,
                gst::GenericFormattedValue::from(position),
                gst::SeekType::Set,
                gst::ClockTime::NONE,
            )?,
            Position::Frame(_) => self.source.seek(
                self.speed,
                seek_flags,
                gst::SeekType::Set,
                gst::GenericFormattedValue::from(position),
                gst::SeekType::Set,
                gst::format::Default::NONE,
            )?,
        };

        *self.subtitle_text.lock().expect("lock subtitle_text") = None;
        self.upload_text.store(true, Ordering::SeqCst);

        Ok(())
    }

    pub(crate) fn set_speed(&mut self, speed: f64) -> Result<(), Error> {
        let Some(position) = self.source.query_position::<gst::ClockTime>() else {
            return Err(Error::Caps);
        };
        if speed > 0.0 {
            self.source.seek(
                speed,
                gst::SeekFlags::FLUSH | gst::SeekFlags::ACCURATE,
                gst::SeekType::Set,
                position,
                gst::SeekType::End,
                gst::ClockTime::from_seconds(0),
            )?;
        } else {
            self.source.seek(
                speed,
                gst::SeekFlags::FLUSH | gst::SeekFlags::ACCURATE,
                gst::SeekType::Set,
                gst::ClockTime::from_seconds(0),
                gst::SeekType::Set,
                position,
            )?;
        }
        self.speed = speed;
        Ok(())
    }

    pub(crate) fn restart_stream(&mut self) -> Result<(), Error> {
        self.is_eos = false;
        self.set_paused(false);
        self.seek(0, false)?;
        Ok(())
    }

    pub(crate) fn set_paused(&mut self, paused: bool) {
        self.source
            .set_state(if paused {
                gst::State::Paused
            } else {
                gst::State::Playing
            })
            .unwrap(/* state was changed in ctor; state errors caught there */);

        // Set restart_stream flag to make the stream restart on the next Message::NextFrame
        if self.is_eos && !paused {
            self.restart_stream = true;
        }
    }

    pub(crate) fn paused(&self) -> bool {
        self.source.state(gst::ClockTime::ZERO).1 == gst::State::Paused
    }

    /// Syncs audio with video when there is (inevitably) latency presenting the frame.
    pub(crate) fn set_av_offset(&mut self, offset: Duration) {
        if self.sync_av {
            self.sync_av_counter += 1;
            self.sync_av_avg = self.sync_av_avg * (self.sync_av_counter - 1) / self.sync_av_counter
                + offset.as_nanos() as u64 / self.sync_av_counter;
            if self.sync_av_counter.is_multiple_of(128) {
                self.source
                    .set_property("av-offset", -(self.sync_av_avg as i64));
            }
        }
    }
}

/// A multimedia video loaded from a URI (e.g., a local file path or HTTP stream).
#[derive(Debug)]
pub struct Video(pub(crate) RwLock<Internal>);

impl Drop for Video {
    fn drop(&mut self) {
        let inner = self.0.get_mut().expect("failed to lock");

        inner
            .source
            .set_state(gst::State::Null)
            .expect("failed to set state");

        inner.alive.store(false, Ordering::SeqCst);
        if let Some(worker) = inner.worker.take()
            && let Err(err) = worker.join()
        {
            match err.downcast_ref::<String>() {
                Some(e) => tracing::error!("Video thread panicked: {e}"),
                None => tracing::error!("Video thread panicked with unknown reason"),
            }
        }

        // Kill yt-dlp process if running
        if let Some(mut child) = inner.ytdlp_process.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

impl Video {
    /// Create a new video player from a given video which loads from `uri`.
    /// Note that live sources will report the duration to be zero.
    pub fn new(uri: &url::Url) -> Result<Self, Error> {
        gst::init()?;

        let pipeline = format!(
            "playbin uri=\"{}\" text-sink=\"appsink name=iced_text sync=true drop=true\" video-sink=\"videoscale ! videoconvert ! appsink name=iced_video drop=true caps=video/x-raw,format=NV12,pixel-aspect-ratio=1/1\"",
            uri.as_str()
        );
        let pipeline = gst::parse::launch(pipeline.as_ref())?
            .downcast::<gst::Pipeline>()
            .map_err(|_| Error::Cast)?;

        let video_sink: gst::Element = pipeline.property("video-sink");
        let pad = video_sink.pads().first().cloned().unwrap();
        let pad = pad.dynamic_cast::<gst::GhostPad>().unwrap();
        let bin = pad
            .parent_element()
            .unwrap()
            .downcast::<gst::Bin>()
            .unwrap();
        let video_sink = bin.by_name("iced_video").unwrap();
        let video_sink = video_sink.downcast::<gst_app::AppSink>().unwrap();

        let text_sink: gst::Element = pipeline.property("text-sink");
        let text_sink = text_sink.downcast::<gst_app::AppSink>().unwrap();

        Self::from_gst_pipeline(pipeline, video_sink, Some(text_sink))
    }

    /// Create an audio-only player from a PeerTube video URL.
    ///
    /// PeerTube doesn't have separate audio streams, so we use the video URL
    /// but discard video frames to save CPU. Includes spectrum analyzer for visualizations.
    pub fn peertube_audio_only(uri: &url::Url) -> Result<Self, Error> {
        gst::init()?;
        static NEXT_ID: AtomicU64 = AtomicU64::new(0);
        let id = NEXT_ID.fetch_add(1, Ordering::SeqCst);

        // Use playbin with fakesink for video (discards video frames)
        // Audio goes through spectrum analyzer for visualizations
        let pipeline_str = format!(
            "playbin uri=\"{}\" \
             video-sink=\"fakesink sync=false\" \
             audio-sink=\"audioconvert ! audioresample ! \
                 tee name=t ! queue ! autoaudiosink sync=true \
                 t. ! queue ! spectrum name=spectrum bands={SPECTRUM_BANDS} interval=50000000 threshold=-80 post-messages=true message-magnitude=true ! fakesink sync=true\"",
            uri.as_str()
        );

        tracing::info!("Creating PeerTube audio-only pipeline");

        let pipeline = gst::parse::launch(&pipeline_str)?
            .downcast::<gst::Pipeline>()
            .map_err(|_| Error::Cast)?;

        let bus = pipeline.bus().ok_or(Error::Bus)?;

        macro_rules! cleanup {
            ($expr:expr) => {
                $expr.map_err(|e| {
                    let _ = pipeline.set_state(gst::State::Null);
                    e
                })
            };
        }

        cleanup!(pipeline.set_state(gst::State::Playing))?;
        cleanup!(pipeline.state(gst::ClockTime::from_seconds(5)).0)?;

        let duration = Duration::from_nanos(
            pipeline
                .query_duration::<gst::ClockTime>()
                .map(|duration| duration.nseconds())
                .unwrap_or(0),
        );

        // Audio-only: use 1080p dimensions for proper 16:9 aspect ratio
        let width = 1920;
        let height = 1080;
        let framerate = 1.0;

        let sync_av = pipeline.has_property("av-offset", None);

        // Empty frame (no video)
        let frame = Arc::new(Mutex::new(Frame::empty()));
        let upload_frame = Arc::new(AtomicBool::new(false));
        let alive = Arc::new(AtomicBool::new(true));
        let last_frame_time = Arc::new(Mutex::new(Instant::now()));

        let subtitle_text = Arc::new(Mutex::new(None));
        let upload_text = Arc::new(AtomicBool::new(false));
        let spectrum = Arc::new(Mutex::new([0.0f32; SPECTRUM_BANDS]));

        Ok(Video(RwLock::new(Internal {
            id,
            bus,
            source: pipeline,
            alive,
            worker: None,
            width,
            height,
            framerate,
            duration,
            speed: 1.0,
            sync_av,
            frame,
            upload_frame,
            last_frame_time,
            looping: false,
            is_eos: false,
            restart_stream: false,
            sync_av_avg: 0,
            sync_av_counter: 0,
            subtitle_text,
            upload_text,
            ytdlp_process: None,
            spectrum,
        })))
    }

    /// Create a new video player from direct video and audio URLs with custom HTTP headers.
    ///
    /// This is useful for playing YouTube videos where yt-dlp provides separate
    /// video and audio stream URLs. Unlike `from_ytdlp`, this method supports
    /// seeking via downloadbuffer elements.
    ///
    /// # Arguments
    /// * `video_url` - The direct video stream URL
    /// * `audio_url` - The direct audio stream URL
    /// * `headers` - HTTP headers as key-value pairs (e.g., User-Agent, Accept, etc.)
    pub fn from_url_with_headers(
        video_url: &str,
        audio_url: &str,
        headers: &[(&str, &str)],
    ) -> Result<Self, Error> {
        gst::init()?;

        // Extract User-Agent from headers (YouTube requires this)
        let user_agent = headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("User-Agent"))
            .map(|(_, v)| *v)
            .unwrap_or("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36");

        // Use souphttpsrc (requires GIO_EXTRA_MODULES for TLS) + downloadbuffer (enables seeking)
        // Note: curlhttpsrc fails when combined with downloadbuffer
        // multiqueue with sync-by-running-time=true ensures A/V synchronization after seeking
        let video_convert = get_best_video_convert();

        let pipeline_str = format!(
            "souphttpsrc name=videosrc location=\"{video_url}\" user-agent=\"{user_agent}\" ! \
             downloadbuffer name=videobuf max-size-bytes=104857600 max-size-time=60000000000 low-percent=1 high-percent=99 temp-template=/tmp/iced-video-XXXXXX ! \
             decodebin name=videodec \
             souphttpsrc name=audiosrc location=\"{audio_url}\" user-agent=\"{user_agent}\" ! \
             downloadbuffer name=audiobuf max-size-bytes=20971520 max-size-time=60000000000 low-percent=1 high-percent=99 temp-template=/tmp/iced-audio-XXXXXX ! \
             decodebin name=audiodec \
             multiqueue name=mq sync-by-running-time=true \
             videodec. ! mq.sink_0 mq.src_0 ! {video_convert} ! videoscale ! appsink name=iced_video drop=true sync=true caps=video/x-raw,format=NV12,pixel-aspect-ratio=1/1 \
             audiodec. ! mq.sink_1 mq.src_1 ! audioconvert ! audioresample ! autoaudiosink sync=true",
        );

        tracing::info!("Creating pipeline with downloadbuffer for seeking support");

        let pipeline = gst::parse::launch(&pipeline_str)?
            .downcast::<gst::Pipeline>()
            .map_err(|_| Error::Cast)?;

        // Origin and Referer headers seems to prevent connection termination
        let extra_headers = gst::Structure::builder("extra-headers")
            .field("Origin", "https://www.youtube.com")
            .field("Referer", "https://www.youtube.com/")
            .build();

        if let Some(videosrc) = pipeline.by_name("videosrc") {
            videosrc.set_property("extra-headers", &extra_headers);
        }
        if let Some(audiosrc) = pipeline.by_name("audiosrc") {
            audiosrc.set_property("extra-headers", &extra_headers);
        }

        let video_sink = pipeline
            .by_name("iced_video")
            .ok_or(Error::Cast)?
            .downcast::<gst_app::AppSink>()
            .map_err(|_| Error::Cast)?;

        Self::from_gst_pipeline(pipeline, video_sink, None)
    }

    /// Create an audio-only player from a direct audio URL with custom HTTP headers.
    ///
    /// This is useful for playing YouTube audio without video (lower bandwidth,
    /// background listening). No video frames are produced.
    ///
    /// # Arguments
    /// * `audio_url` - The direct audio stream URL
    /// * `headers` - HTTP headers as key-value pairs (e.g., User-Agent, Accept, etc.)
    pub fn from_audio_url_only(audio_url: &str, headers: &[(&str, &str)]) -> Result<Self, Error> {
        gst::init()?;
        static NEXT_ID: AtomicU64 = AtomicU64::new(0);
        let id = NEXT_ID.fetch_add(1, Ordering::SeqCst);

        // Extract User-Agent from headers (YouTube requires this)
        let user_agent = headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("User-Agent"))
            .map(|(_, v)| *v)
            .unwrap_or("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36");

        // Audio-only pipeline with spectrum analyzer for visualizations
        // The spectrum element posts bus messages with frequency magnitude data
        // Using tee to split audio: one path for playback, one for spectrum analysis
        let pipeline_str = format!(
            "souphttpsrc name=audiosrc location=\"{audio_url}\" user-agent=\"{user_agent}\" ! \
             downloadbuffer name=audiobuf max-size-bytes=20971520 max-size-time=60000000000 low-percent=1 high-percent=99 temp-template=/tmp/iced-audio-XXXXXX ! \
             decodebin name=audiodec ! \
             audioconvert ! audioresample ! \
             tee name=t ! queue ! autoaudiosink sync=true \
             t. ! queue ! spectrum name=spectrum bands={SPECTRUM_BANDS} interval=50000000 threshold=-80 post-messages=true message-magnitude=true ! fakesink sync=true",
        );

        tracing::info!("Creating audio-only pipeline");

        let pipeline = gst::parse::launch(&pipeline_str)?
            .downcast::<gst::Pipeline>()
            .map_err(|_| Error::Cast)?;

        // Origin and Referer headers seems to prevent connection termination
        let extra_headers = gst::Structure::builder("extra-headers")
            .field("Origin", "https://www.youtube.com")
            .field("Referer", "https://www.youtube.com/")
            .build();

        if let Some(audiosrc) = pipeline.by_name("audiosrc") {
            audiosrc.set_property("extra-headers", &extra_headers);
        }

        let bus = pipeline.bus().ok_or(Error::Bus)?;

        // We need to ensure we stop the pipeline if we hit an error
        macro_rules! cleanup {
            ($expr:expr) => {
                $expr.map_err(|e| {
                    let _ = pipeline.set_state(gst::State::Null);
                    e
                })
            };
        }

        cleanup!(pipeline.set_state(gst::State::Playing))?;

        // Wait for up to 5 seconds until the pipeline is ready
        cleanup!(pipeline.state(gst::ClockTime::from_seconds(5)).0)?;

        // Get duration (may not be available immediately for streams)
        let duration = Duration::from_nanos(
            pipeline
                .query_duration::<gst::ClockTime>()
                .map(|duration| duration.nseconds())
                .unwrap_or(0),
        );

        // Audio-only: use 1080p dimensions for proper 16:9 aspect ratio
        let width = 1920;
        let height = 1080;
        let framerate = 1.0;

        let sync_av = pipeline.has_property("av-offset", None);

        // Empty frame (no video)
        let frame = Arc::new(Mutex::new(Frame::empty()));
        let upload_frame = Arc::new(AtomicBool::new(false));
        let alive = Arc::new(AtomicBool::new(true));
        let last_frame_time = Arc::new(Mutex::new(Instant::now()));

        let subtitle_text = Arc::new(Mutex::new(None));
        let upload_text = Arc::new(AtomicBool::new(false));
        let spectrum = Arc::new(Mutex::new([0.0f32; SPECTRUM_BANDS]));

        Ok(Video(RwLock::new(Internal {
            id,
            bus,
            source: pipeline,
            alive,
            worker: None, // No worker thread needed for audio-only
            width,
            height,
            framerate,
            duration,
            speed: 1.0,
            sync_av,
            frame,
            upload_frame,
            last_frame_time,
            looping: false,
            is_eos: false,
            restart_stream: false,
            sync_av_avg: 0,
            sync_av_counter: 0,
            subtitle_text,
            upload_text,
            ytdlp_process: None,
            spectrum,
        })))
    }

    /// Create a new video player that streams from yt-dlp.
    ///
    /// This spawns yt-dlp as a subprocess, streaming video data to stdout,
    /// which GStreamer reads via a file descriptor source.
    ///
    /// # Arguments
    /// * `video_url` - The YouTube video URL (e.g., "https://www.youtube.com/watch?v=...")
    pub fn from_ytdlp(video_url: &str) -> Result<Self, Error> {
        gst::init()?;

        // Spawn yt-dlp to stream video to stdout
        // -f "bv*+ba/b" = best video + best audio, fallback to best combined
        // -o - = output to stdout
        // --downloader ffmpeg = use ffmpeg for proper muxing when streaming
        let mut child = Command::new("yt-dlp")
            .args([
                "-f",
                "bv*+ba/b",
                "-o",
                "-",
                "--downloader",
                "ffmpeg",
                video_url,
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        // Spawn a thread to read stderr and log it as trace
        if let Some(stderr) = child.stderr.take() {
            std::thread::spawn(move || {
                use std::io::{BufRead, BufReader};
                let reader = BufReader::new(stderr);
                for line in reader.lines() {
                    match line {
                        Ok(line) => tracing::trace!("yt-dlp: {}", line),
                        Err(e) => {
                            tracing::trace!("yt-dlp stderr read error: {}", e);
                            break;
                        }
                    }
                }
            });
        }

        // Get the stdout file descriptor
        let stdout = child.stdout.take().ok_or(Error::Io(std::io::Error::other(
            "Failed to capture yt-dlp stdout",
        )))?;
        let fd = stdout.into_raw_fd();

        // Create a GStreamer pipeline that reads from the file descriptor
        // fdsrc reads from a file descriptor
        // decodebin automatically detects and decodes the stream
        // We use playbin3 with fd:// URI scheme for better handling
        let pipeline = format!(
            "playbin uri=\"fd://{}\" video-sink=\"videoscale ! videoconvert ! appsink name=iced_video drop=true caps=video/x-raw,format=NV12,pixel-aspect-ratio=1/1\"",
            fd
        );

        let pipeline = gst::parse::launch(pipeline.as_ref())?
            .downcast::<gst::Pipeline>()
            .map_err(|_| Error::Cast)?;

        let video_sink: gst::Element = pipeline.property("video-sink");
        let pad = video_sink.pads().first().cloned().unwrap();
        let pad = pad.dynamic_cast::<gst::GhostPad>().unwrap();
        let bin = pad
            .parent_element()
            .unwrap()
            .downcast::<gst::Bin>()
            .unwrap();
        let video_sink = bin.by_name("iced_video").unwrap();
        let video_sink = video_sink.downcast::<gst_app::AppSink>().unwrap();

        Self::from_gst_pipeline_with_process(pipeline, video_sink, None, child)
    }

    /// Like `from_gst_pipeline` but also stores a child process to be killed on drop.
    fn from_gst_pipeline_with_process(
        pipeline: gst::Pipeline,
        video_sink: gst_app::AppSink,
        text_sink: Option<gst_app::AppSink>,
        ytdlp_process: Child,
    ) -> Result<Self, Error> {
        gst::init()?;
        static NEXT_ID: AtomicU64 = AtomicU64::new(0);
        let id = NEXT_ID.fetch_add(1, Ordering::SeqCst);

        // We need to ensure we stop the pipeline if we hit an error,
        // or else there may be audio left playing in the background.
        // Also kill the yt-dlp process on error.
        let mut ytdlp_process = Some(ytdlp_process);
        macro_rules! cleanup {
            ($expr:expr) => {
                $expr.map_err(|e| {
                    let _ = pipeline.set_state(gst::State::Null);
                    if let Some(mut proc) = ytdlp_process.take() {
                        let _ = proc.kill();
                        let _ = proc.wait();
                    }
                    e
                })
            };
        }

        let pad = video_sink.pads().first().cloned().unwrap();

        cleanup!(pipeline.set_state(gst::State::Playing))?;

        // wait for up to 10 seconds for yt-dlp streams (they can be slow to start)
        cleanup!(pipeline.state(gst::ClockTime::from_seconds(10)).0)?;

        // extract resolution and framerate
        let caps = cleanup!(pad.current_caps().ok_or(Error::Caps))?;
        let s = cleanup!(caps.structure(0).ok_or(Error::Caps))?;
        let width = cleanup!(s.get::<i32>("width").map_err(|_| Error::Caps))?;
        let height = cleanup!(s.get::<i32>("height").map_err(|_| Error::Caps))?;
        let framerate = cleanup!(s.get::<gst::Fraction>("framerate").map_err(|_| Error::Caps))?;
        let framerate = framerate.numer() as f64 / framerate.denom() as f64;

        if framerate.is_nan()
            || framerate.is_infinite()
            || framerate < 0.0
            || framerate.abs() < f64::EPSILON
        {
            let _ = pipeline.set_state(gst::State::Null);
            if let Some(mut proc) = ytdlp_process.take() {
                let _ = proc.kill();
                let _ = proc.wait();
            }
            return Err(Error::Framerate(framerate));
        }

        // For streams, duration may not be known
        let duration = Duration::from_nanos(
            pipeline
                .query_duration::<gst::ClockTime>()
                .map(|duration| duration.nseconds())
                .unwrap_or(0),
        );

        let sync_av = pipeline.has_property("av-offset", None);

        let frame = Arc::new(Mutex::new(Frame::empty()));
        let upload_frame = Arc::new(AtomicBool::new(false));
        let alive = Arc::new(AtomicBool::new(true));
        let last_frame_time = Arc::new(Mutex::new(Instant::now()));

        let frame_ref = Arc::clone(&frame);
        let upload_frame_ref = Arc::clone(&upload_frame);
        let alive_ref = Arc::clone(&alive);
        let last_frame_time_ref = Arc::clone(&last_frame_time);

        let subtitle_text = Arc::new(Mutex::new(None));
        let upload_text = Arc::new(AtomicBool::new(false));
        let subtitle_text_ref = Arc::clone(&subtitle_text);
        let upload_text_ref = Arc::clone(&upload_text);

        let pipeline_ref = pipeline.clone();

        let worker = std::thread::spawn(move || {
            let mut clear_subtitles_at = None;

            while alive_ref.load(Ordering::Acquire) {
                if let Err(gst::FlowError::Error) = (|| -> Result<(), gst::FlowError> {
                    let sample =
                        if pipeline_ref.state(gst::ClockTime::ZERO).1 != gst::State::Playing {
                            video_sink
                                .try_pull_preroll(gst::ClockTime::from_mseconds(16))
                                .ok_or(gst::FlowError::Eos)?
                        } else {
                            video_sink
                                .try_pull_sample(gst::ClockTime::from_mseconds(16))
                                .ok_or(gst::FlowError::Eos)?
                        };

                    *last_frame_time_ref
                        .lock()
                        .map_err(|_| gst::FlowError::Error)? = Instant::now();

                    let _frame_segment = sample.segment().cloned().ok_or(gst::FlowError::Error)?;
                    let buffer = sample.buffer().ok_or(gst::FlowError::Error)?;
                    let frame_pts = buffer.pts().ok_or(gst::FlowError::Error)?;
                    let _frame_duration = buffer.duration().unwrap_or(gst::ClockTime::ZERO);
                    {
                        let mut frame_guard =
                            frame_ref.lock().map_err(|_| gst::FlowError::Error)?;
                        *frame_guard = Frame(sample);
                    }

                    upload_frame_ref.swap(true, Ordering::SeqCst);

                    if let Some(at) = clear_subtitles_at
                        && frame_pts >= at
                    {
                        *subtitle_text_ref
                            .lock()
                            .map_err(|_| gst::FlowError::Error)? = None;
                        upload_text_ref.store(true, Ordering::SeqCst);
                        clear_subtitles_at = None;
                    }

                    // No subtitle handling for yt-dlp streams (text_sink is None)
                    let text = text_sink
                        .as_ref()
                        .and_then(|sink| sink.try_pull_sample(gst::ClockTime::from_seconds(0)));
                    if let Some(text) = text
                        && let (Some(_text_segment), Some(text_buffer)) =
                            (text.segment(), text.buffer())
                        && let (Some(text_pts), Some(text_duration)) =
                            (text_buffer.pts(), text_buffer.duration())
                    {
                        let duration = text_duration;
                        if let Ok(map) = text_buffer.map_readable()
                            && let Ok(text_str) = std::str::from_utf8(map.as_slice())
                        {
                            *subtitle_text_ref
                                .lock()
                                .map_err(|_| gst::FlowError::Error)? = Some(text_str.to_string());
                            upload_text_ref.store(true, Ordering::SeqCst);
                            clear_subtitles_at = Some(text_pts + duration);
                        }
                    }

                    Ok(())
                })() {
                    tracing::error!("error pulling frame");
                }
            }
        });

        Ok(Video(RwLock::new(Internal {
            id,

            bus: pipeline.bus().unwrap(),
            source: pipeline,
            alive,
            worker: Some(worker),

            width,
            height,
            framerate,
            duration,
            speed: 1.0,
            sync_av,

            frame,
            upload_frame,
            last_frame_time,
            looping: false,
            is_eos: false,
            restart_stream: false,
            sync_av_avg: 0,
            sync_av_counter: 0,

            subtitle_text,
            upload_text,
            ytdlp_process,
            spectrum: Arc::new(Mutex::new([0.0f32; SPECTRUM_BANDS])),
        })))
    }

    /// Creates a new video based on an existing GStreamer pipeline and appsink.
    /// Expects an `appsink` plugin with `caps=video/x-raw,format=NV12`.
    ///
    /// An optional `text_sink` can be provided, which enables subtitle messages
    /// to be emitted.
    ///
    /// **Note:** Many functions of [`Video`] assume a `playbin` pipeline.
    /// Non-`playbin` pipelines given here may not have full functionality.
    pub fn from_gst_pipeline(
        pipeline: gst::Pipeline,
        video_sink: gst_app::AppSink,
        text_sink: Option<gst_app::AppSink>,
    ) -> Result<Self, Error> {
        gst::init()?;
        static NEXT_ID: AtomicU64 = AtomicU64::new(0);
        let id = NEXT_ID.fetch_add(1, Ordering::SeqCst);

        // We need to ensure we stop the pipeline if we hit an error,
        // or else there may be audio left playing in the background.
        macro_rules! cleanup {
            ($expr:expr) => {
                $expr.map_err(|e| {
                    let _ = pipeline.set_state(gst::State::Null);
                    e
                })
            };
        }

        let pad = video_sink.pads().first().cloned().unwrap();

        cleanup!(pipeline.set_state(gst::State::Playing))?;

        // wait for up to 5 seconds until the decoder gets the source capabilities
        cleanup!(pipeline.state(gst::ClockTime::from_seconds(5)).0)?;

        // Log all elements in the pipeline to help debug decoder selection
        for element in pipeline.iterate_recurse().into_iter().flatten() {
            let factory = element
                .factory()
                .map(|f| f.name().to_string())
                .unwrap_or_default();
            if factory.contains("dec") || factory.contains("parse") {
                tracing::info!("Pipeline element: {} ({})", element.name(), factory);
            }
        }

        // Get caps, polling if necessary (up to 30 seconds)
        // With downloadbuffer, the pipeline may report Playing but caps aren't ready yet
        let caps = if let Some(caps) = pad.current_caps() {
            caps
        } else {
            let start = std::time::Instant::now();
            let timeout = std::time::Duration::from_secs(30);
            loop {
                std::thread::sleep(std::time::Duration::from_millis(100));
                if let Some(caps) = pad.current_caps() {
                    break caps;
                }
                if start.elapsed() > timeout {
                    let _ = pipeline.set_state(gst::State::Null);
                    return Err(Error::Caps);
                }
            }
        };
        let s = cleanup!(caps.structure(0).ok_or(Error::Caps))?;
        let width = cleanup!(s.get::<i32>("width").map_err(|_| Error::Caps))?;
        let height = cleanup!(s.get::<i32>("height").map_err(|_| Error::Caps))?;
        let framerate = cleanup!(s.get::<gst::Fraction>("framerate").map_err(|_| Error::Caps))?;
        let framerate = framerate.numer() as f64 / framerate.denom() as f64;

        if framerate.is_nan()
            || framerate.is_infinite()
            || framerate < 0.0
            || framerate.abs() < f64::EPSILON
        {
            let _ = pipeline.set_state(gst::State::Null);
            return Err(Error::Framerate(framerate));
        }

        let duration = Duration::from_nanos(
            pipeline
                .query_duration::<gst::ClockTime>()
                .map(|duration| duration.nseconds())
                .unwrap_or(0),
        );

        let sync_av = pipeline.has_property("av-offset", None);

        // NV12 = 12bpp
        let frame = Arc::new(Mutex::new(Frame::empty()));
        let upload_frame = Arc::new(AtomicBool::new(false));
        let alive = Arc::new(AtomicBool::new(true));
        let last_frame_time = Arc::new(Mutex::new(Instant::now()));

        let frame_ref = Arc::clone(&frame);
        let upload_frame_ref = Arc::clone(&upload_frame);
        let alive_ref = Arc::clone(&alive);
        let last_frame_time_ref = Arc::clone(&last_frame_time);

        let subtitle_text = Arc::new(Mutex::new(None));
        let upload_text = Arc::new(AtomicBool::new(false));
        let subtitle_text_ref = Arc::clone(&subtitle_text);
        let upload_text_ref = Arc::clone(&upload_text);

        let pipeline_ref = pipeline.clone();

        let worker = std::thread::spawn(move || {
            let mut clear_subtitles_at = None;

            while alive_ref.load(Ordering::Acquire) {
                if let Err(gst::FlowError::Error) = (|| -> Result<(), gst::FlowError> {
                    let sample =
                        if pipeline_ref.state(gst::ClockTime::ZERO).1 != gst::State::Playing {
                            video_sink
                                .try_pull_preroll(gst::ClockTime::from_mseconds(16))
                                .ok_or(gst::FlowError::Eos)?
                        } else {
                            video_sink
                                .try_pull_sample(gst::ClockTime::from_mseconds(16))
                                .ok_or(gst::FlowError::Eos)?
                        };

                    *last_frame_time_ref
                        .lock()
                        .map_err(|_| gst::FlowError::Error)? = Instant::now();

                    let frame_segment = sample.segment().cloned().ok_or(gst::FlowError::Error)?;
                    let buffer = sample.buffer().ok_or(gst::FlowError::Error)?;
                    let frame_pts = buffer.pts().ok_or(gst::FlowError::Error)?;
                    let frame_duration = buffer.duration().ok_or(gst::FlowError::Error)?;
                    {
                        let mut frame_guard =
                            frame_ref.lock().map_err(|_| gst::FlowError::Error)?;
                        *frame_guard = Frame(sample);
                    }

                    upload_frame_ref.swap(true, Ordering::SeqCst);

                    if let Some(at) = clear_subtitles_at
                        && frame_pts >= at
                    {
                        *subtitle_text_ref
                            .lock()
                            .map_err(|_| gst::FlowError::Error)? = None;
                        upload_text_ref.store(true, Ordering::SeqCst);
                        clear_subtitles_at = None;
                    }

                    let text = text_sink
                        .as_ref()
                        .and_then(|sink| sink.try_pull_sample(gst::ClockTime::from_seconds(0)));
                    if let Some(text) = text {
                        let text_segment = text.segment().ok_or(gst::FlowError::Error)?;
                        let text = text.buffer().ok_or(gst::FlowError::Error)?;
                        let text_pts = text.pts().ok_or(gst::FlowError::Error)?;
                        let text_duration = text.duration().ok_or(gst::FlowError::Error)?;

                        let frame_running_time = frame_segment.to_running_time(frame_pts).value();
                        let frame_running_time_end = frame_segment
                            .to_running_time(frame_pts + frame_duration)
                            .value();

                        let text_running_time = text_segment.to_running_time(text_pts).value();
                        let text_running_time_end = text_segment
                            .to_running_time(text_pts + text_duration)
                            .value();

                        // see gst-plugins-base/ext/pango/gstbasetextoverlay.c (gst_base_text_overlay_video_chain)
                        // as an example of how to correctly synchronize the text+video segments
                        if text_running_time_end > frame_running_time
                            && frame_running_time_end > text_running_time
                        {
                            let duration = text.duration().unwrap_or(gst::ClockTime::ZERO);
                            let map = text.map_readable().map_err(|_| gst::FlowError::Error)?;

                            let text = std::str::from_utf8(map.as_slice())
                                .map_err(|_| gst::FlowError::Error)?
                                .to_string();
                            *subtitle_text_ref
                                .lock()
                                .map_err(|_| gst::FlowError::Error)? = Some(text);
                            upload_text_ref.store(true, Ordering::SeqCst);

                            clear_subtitles_at = Some(text_pts + duration);
                        }
                    }

                    Ok(())
                })() {
                    tracing::error!("error pulling frame");
                }
            }
        });

        Ok(Video(RwLock::new(Internal {
            id,

            bus: pipeline.bus().unwrap(),
            source: pipeline,
            alive,
            worker: Some(worker),

            width,
            height,
            framerate,
            duration,
            speed: 1.0,
            sync_av,

            frame,
            upload_frame,
            last_frame_time,
            looping: false,
            is_eos: false,
            restart_stream: false,
            sync_av_avg: 0,
            sync_av_counter: 0,

            subtitle_text,
            upload_text,
            ytdlp_process: None,
            spectrum: Arc::new(Mutex::new([0.0f32; SPECTRUM_BANDS])),
        })))
    }

    pub(crate) fn read(&self) -> impl Deref<Target = Internal> + '_ {
        self.0.read().expect("lock")
    }

    pub(crate) fn write(&self) -> impl DerefMut<Target = Internal> + '_ {
        self.0.write().expect("lock")
    }

    pub(crate) fn get_mut(&mut self) -> impl DerefMut<Target = Internal> + '_ {
        self.0.get_mut().expect("lock")
    }

    /// Get the size/resolution of the video as `(width, height)`.
    pub fn size(&self) -> (i32, i32) {
        (self.read().width, self.read().height)
    }

    /// Get the framerate of the video as frames per second.
    pub fn framerate(&self) -> f64 {
        self.read().framerate
    }

    /// Get a clone of the spectrum data arc for audio visualization.
    /// Returns frequency magnitudes normalized to 0.0-1.0 range.
    pub fn spectrum(&self) -> Arc<Mutex<[f32; SPECTRUM_BANDS]>> {
        Arc::clone(&self.read().spectrum)
    }

    /// Set the volume multiplier of the audio.
    /// `0.0` = 0% volume, `1.0` = 100% volume.
    ///
    /// This uses a linear scale, for example `0.5` is perceived as half as loud.
    pub fn set_volume(&self, volume: f64) {
        self.write().source.set_property("volume", volume);
        self.set_muted(self.muted()); // for some reason gstreamer unmutes when changing volume?
    }

    /// Get the volume multiplier of the audio.
    pub fn volume(&self) -> f64 {
        self.read().source.property("volume")
    }

    /// Set if the audio is muted or not, without changing the volume.
    pub fn set_muted(&self, muted: bool) {
        self.write().source.set_property("mute", muted);
    }

    /// Get if the audio is muted or not.
    pub fn muted(&self) -> bool {
        self.read().source.property("mute")
    }

    /// Get if the stream ended or not.
    pub fn eos(&self) -> bool {
        self.read().is_eos
    }

    /// Get if the media will loop or not.
    pub fn looping(&self) -> bool {
        self.read().looping
    }

    /// Set if the media will loop or not.
    pub fn set_looping(&self, looping: bool) {
        self.write().looping = looping;
    }

    /// Set if the media is paused or not.
    pub fn set_paused(&self, paused: bool) {
        self.write().set_paused(paused)
    }

    /// Get if the media is paused or not.
    pub fn paused(&self) -> bool {
        self.read().paused()
    }

    /// Jumps to a specific position in the media.
    /// Passing `true` to the `accurate` parameter will result in more accurate seeking,
    /// however, it is also slower. For most seeks (e.g., scrubbing) this is not needed.
    pub fn seek(&self, position: impl Into<Position>, accurate: bool) -> Result<(), Error> {
        self.write().seek(position, accurate)
    }

    /// Set the playback speed of the media.
    /// The default speed is `1.0`.
    pub fn set_speed(&self, speed: f64) -> Result<(), Error> {
        self.write().set_speed(speed)
    }

    /// Get the current playback speed.
    pub fn speed(&self) -> f64 {
        self.read().speed
    }

    /// Get the current playback position in time.
    pub fn position(&self) -> Duration {
        Duration::from_nanos(
            self.read()
                .source
                .query_position::<gst::ClockTime>()
                .map_or(0, |pos| pos.nseconds()),
        )
    }

    /// Get the media duration.
    pub fn duration(&self) -> Duration {
        let stored = self.read().duration;
        if stored > Duration::ZERO {
            stored
        } else {
            // Query dynamically if stored duration is 0 (e.g., audio-only streams)
            Duration::from_nanos(
                self.read()
                    .source
                    .query_duration::<gst::ClockTime>()
                    .map_or(0, |d| d.nseconds()),
            )
        }
    }

    /// Restarts a stream; seeks to the first frame and unpauses, sets the `eos` flag to false.
    pub fn restart_stream(&mut self) -> Result<(), Error> {
        self.get_mut().restart_stream()
    }

    /// Set the subtitle URL to display.
    pub fn set_subtitle_url(&mut self, url: &url::Url) -> Result<(), Error> {
        let paused = self.paused();
        let mut inner = self.get_mut();
        inner.source.set_state(gst::State::Ready)?;
        inner.source.set_property("suburi", url.as_str());
        inner.set_paused(paused);
        Ok(())
    }

    /// Get the current subtitle URL.
    pub fn subtitle_url(&self) -> Option<url::Url> {
        url::Url::parse(
            &self
                .read()
                .source
                .property::<Option<String>>("current-suburi")?,
        )
        .ok()
    }

    /// Get the underlying GStreamer pipeline.
    pub fn pipeline(&self) -> gst::Pipeline {
        self.read().source.clone()
    }

    /// Generates a list of thumbnails based on a set of positions in the media, downscaled by a given factor.
    ///
    /// Slow; only needs to be called once for each instance.
    /// It's best to call this at the very start of playback, otherwise the position may shift.
    pub fn thumbnails<I>(
        &mut self,
        positions: I,
        downscale: NonZeroU8,
    ) -> Result<Vec<img::Handle>, Error>
    where
        I: IntoIterator<Item = Position>,
    {
        let downscale = u8::from(downscale) as u32;

        let paused = self.paused();
        let muted = self.muted();
        let pos = self.position();

        self.set_paused(false);
        self.set_muted(true);

        let out = {
            let inner = self.read();
            let width = inner.width;
            let height = inner.height;
            positions
                .into_iter()
                .map(|pos| {
                    inner.seek(pos, true)?;
                    inner.upload_frame.store(false, Ordering::SeqCst);
                    while !inner.upload_frame.load(Ordering::SeqCst) {
                        std::hint::spin_loop();
                    }
                    let frame_guard = inner.frame.lock().map_err(|_| Error::Lock)?;
                    let frame = frame_guard.readable().ok_or(Error::Lock)?;
                    let stride = frame_guard.stride();

                    Ok(img::Handle::from_rgba(
                        inner.width as u32 / downscale,
                        inner.height as u32 / downscale,
                        yuv_to_rgba(frame.as_slice(), width as _, height as _, downscale, stride),
                    ))
                })
                .collect()
        };

        self.set_paused(paused);
        self.set_muted(muted);
        self.seek(pos, true)?;

        out
    }
}

fn yuv_to_rgba(
    yuv: &[u8],
    width: u32,
    height: u32,
    downscale: u32,
    stride: Option<u32>,
) -> Vec<u8> {
    // Use stride from VideoMeta if available, otherwise assume stride == width
    let stride = stride.unwrap_or(width);

    let uv_start = stride * height;
    let mut rgba = vec![];

    for y in 0..height / downscale {
        for x in 0..width / downscale {
            let x_src = x * downscale;
            let y_src = y * downscale;

            // NV12 memory layout with stride:
            // Y plane: stride bytes per row, starting at offset 0
            // UV plane: stride bytes per row (same stride), starting at offset stride * height
            // Each pixel is 1 byte Y, and every 2x2 block shares 2 bytes (U, V)
            let y_offset = (y_src * stride + x_src) as usize;
            let uv_offset = (uv_start + (y_src / 2) * stride + (x_src / 2) * 2) as usize;

            let y = yuv[y_offset] as f32;
            let u = yuv[uv_offset] as f32;
            let v = yuv[uv_offset + 1] as f32;

            let r = 1.164 * (y - 16.0) + 1.596 * (v - 128.0);
            let g = 1.164 * (y - 16.0) - 0.813 * (v - 128.0) - 0.391 * (u - 128.0);
            let b = 1.164 * (y - 16.0) + 2.018 * (u - 128.0);

            rgba.push(r as u8);
            rgba.push(g as u8);
            rgba.push(b as u8);
            rgba.push(0xFF);
        }
    }

    rgba
}
