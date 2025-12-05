//! Video stream handling - fetching and demuxing YouTube videos.

use std::io::Cursor;

/// A video stream from YouTube
pub struct VideoStream {
    /// Video URL (from yt-dlp)
    url: String,
    /// Video format ID
    format_id: String,
    /// Total duration in seconds
    duration: Option<f64>,
    /// Current position in seconds
    position: f64,
}

impl VideoStream {
    /// Create a new video stream from a URL
    pub fn new(url: String, format_id: String, duration: Option<f64>) -> Self {
        Self {
            url,
            format_id,
            duration,
            position: 0.0,
        }
    }

    /// Get the stream URL
    pub fn url(&self) -> &str {
        &self.url
    }

    /// Get current position
    pub fn position(&self) -> f64 {
        self.position
    }

    /// Get duration
    pub fn duration(&self) -> Option<f64> {
        self.duration
    }
}

/// Extract H.264 NAL units from an MP4 container
pub struct Mp4Demuxer {
    reader: mp4::Mp4Reader<Cursor<Vec<u8>>>,
    video_track_id: u32,
    current_sample: u32,
    total_samples: u32,
}

impl Mp4Demuxer {
    /// Create a demuxer from MP4 data
    pub fn new(data: Vec<u8>) -> Result<Self, String> {
        let size = data.len() as u64;
        let cursor = Cursor::new(data);

        let reader = mp4::Mp4Reader::read_header(cursor, size)
            .map_err(|e| format!("Failed to read MP4 header: {}", e))?;

        // Find the video track (H.264)
        let video_track_id = reader
            .tracks()
            .iter()
            .find(|(_, track)| track.media_type().ok() == Some(mp4::MediaType::H264))
            .map(|(id, _)| *id)
            .ok_or("No H.264 video track found")?;

        let track = reader.tracks().get(&video_track_id).unwrap();
        let total_samples = track.sample_count();

        Ok(Self {
            reader,
            video_track_id,
            current_sample: 1, // MP4 samples are 1-indexed
            total_samples,
        })
    }

    /// Read the next H.264 sample (NAL units)
    pub fn next_sample(&mut self) -> Result<Option<Vec<u8>>, String> {
        if self.current_sample > self.total_samples {
            return Ok(None);
        }

        let sample = self
            .reader
            .read_sample(self.video_track_id, self.current_sample)
            .map_err(|e| format!("Failed to read sample: {}", e))?;

        self.current_sample += 1;

        match sample {
            Some(sample) => {
                // Convert from AVCC format (length-prefixed) to Annex B format (start codes)
                let nal_units = avcc_to_annexb(&sample.bytes);
                Ok(Some(nal_units))
            }
            None => Ok(None),
        }
    }

    /// Get total number of samples
    pub fn total_samples(&self) -> u32 {
        self.total_samples
    }

    /// Get current sample index
    pub fn current_sample(&self) -> u32 {
        self.current_sample
    }
}

/// Convert AVCC format (length-prefixed NAL units) to Annex B format (start code prefixed)
fn avcc_to_annexb(avcc_data: &[u8]) -> Vec<u8> {
    let mut annexb = Vec::new();
    let mut pos = 0;

    while pos + 4 <= avcc_data.len() {
        // Read 4-byte length prefix (big endian)
        let nal_len = u32::from_be_bytes([
            avcc_data[pos],
            avcc_data[pos + 1],
            avcc_data[pos + 2],
            avcc_data[pos + 3],
        ]) as usize;

        pos += 4;

        if pos + nal_len > avcc_data.len() {
            break;
        }

        // Add Annex B start code
        annexb.extend_from_slice(&[0x00, 0x00, 0x00, 0x01]);
        // Add NAL unit data
        annexb.extend_from_slice(&avcc_data[pos..pos + nal_len]);

        pos += nal_len;
    }

    annexb
}
