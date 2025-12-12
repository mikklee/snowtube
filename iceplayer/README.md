# iceplayer

Video player widget for Iced, forked from [iced_video_player](https://github.com/jazzfool/iced_video_player).

Built on the excellent GStreamer library.

## Overview

In general, this supports anything that [`gstreamer/playbin`](https://gstreamer.freedesktop.org/documentation/playback/playbin.html?gi-language=c) supports.

Features:
- Load video files from any file path **or URL** (support for streaming over network).
- Video buffering when streaming on a network.
- Audio support.
- Programmatic control.
- Can capture thumbnails from a set of timestamps.
- Good performance (i.e., comparable to other video players). GStreamer (with the right plugins) will perform hardware-accelerated decoding, and the color space (YUV to RGB) is converted on the GPU whilst rendering the frame.

Limitations (hopefully to be fixed):
- GStreamer is a bit annoying to set up on Windows.

The player **does not** come with any surrounding GUI controls, but they should be quite easy to implement should you need them.
See the "minimal" example for a demonstration on how you could implement pausing, looping, and seeking.

## Example Usage

```rust
use iceplayer::{Video, VideoPlayer};

fn main() -> iced::Result {
    iced::run("Video Player", (), App::view)
}

struct App {
    video: Video,
}

impl Default for App {
    fn default() -> Self {
        App {
            video: Video::new(&url::Url::parse("file:///C:/my_video.mp4").unwrap()).unwrap(),
        }
    }
}

impl App {
    fn view(&self) -> iced::Element<()> {
        VideoPlayer::new(&self.video).into()
    }
}
```

## Building

Follow the [GStreamer build instructions](https://github.com/sdroege/gstreamer-rs#installation). This should be able to compile on MSVC, MinGW, Linux, and MacOS.

## License

Licensed under either

- [Apache 2.0](https://www.apache.org/licenses/LICENSE-2.0)
- [MIT](http://opensource.org/licenses/MIT)

at your option.
