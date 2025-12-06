# YTRS

A work in progress YouTube client and InnerTube API library in Rust. Originally based on [YouTube.JS](https://github.com/LuanRT/YouTube.js/).

Built to solve a specific problem: watching YouTube content in its original language.

## Why?

YouTube's auto-translation often replaces original titles with poor machine translations. If you're multilingual, this makes discovering content in specific languages frustrating.

ytrs automatically detects the language of your search query using [whatlang](https://crates.io/crates/whatlang) and [lingua](https://crates.io/crates/lingua), then requests results in that locale from the InnerTube API. You can also manually change the locale, and even save your preferred locale so you don't have to change it when you restart the application.

**Limitations:** YouTube still uses your IP location for some results regardless of locale settings.

## Features

### Library (`ytrs-lib`)

Rust client library for YouTube's private InnerTube API.

- Search with locale support (auto-detection or manual override)
- Channel information and video listings
- Tab navigation (Videos/Shorts/Streams)
- Sort filters and pagination
- Async API using [reqwest](https://crates.io/crates/reqwest) and [tokio](https://crates.io/crates/tokio)

**Usage:**

Basic example:
```rust
use ytrs_lib::InnerTube;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = InnerTube::new().await?;
    let results = client.search("rust programming").await?;

    for video in results.results {
        println!("{}", video.title);
    }

    Ok(())
}
```

### Client (`ytrs-client`)

GUI client built with [Iced](https://iced.rs/).

- Video search
- Channel browsing (videos/shorts/streams tabs)
- Local subscriptions with per-channel language preferences (no account required)
- Sort filters
- Embedded video player using [iced_video_player](https://github.com/jazzfool/iced_video_player) (GStreamer based)
- Persistent configuration
- Responsive layout
- Theme selection (16 themes including Catppuccin, Tokyo Night, Gruvbox, and more)

**Requirements:** [yt-dlp](https://github.com/yt-dlp/yt-dlp) and GStreamer for video playback

## Screenshots

### Search View
![Search view](ytrs-client/screenshots/1.png)
### Channel View
![Channel view](ytrs-client/screenshots/2.png)
### Channels View
![Channels view](ytrs-client/screenshots/3.png)
### Settings View
![Settings](ytrs-client/screenshots/4.png)
### Video View
![Video loading](ytrs-client/screenshots/5.png)
![Video playing](ytrs-client/screenshots/6.png)

## Dependencies

**GStreamer:** Follow the [GStreamer installation instructions](https://github.com/sdroege/gstreamer-rs#installation) for your platform. You'll also need `glib` and `glib-networking` (for TLS support).

**yt-dlp:** Package repositories often have outdated versions. Consider following the [official installation instructions](https://github.com/yt-dlp/yt-dlp#installation).

**mpv (optional):** For the "Open in mpv" button. Install from [mpv.io](https://mpv.io/installation/).

### Building

```bash
cargo build --release
```

Run the client:
```bash
cargo run -p ytrs-client
```

## Status

Work in progress. Maintained for personal use. Contributions are welcome.

This project is maintained at [Codeberg](https://codeberg.org/mikklee/ytrs) but mirrored to [Github](https://github.com/mikklee/ytrs) for disoverability.

## Development

Parts of this project were built with AI assistance (Claude). Code is reviewed and understood before committing.

ytrs-client uses a [forked](https://github.com/mikklee/iced_video_player/tree/feat/video-from-pipe) version of iced_video_player to support playing the videos. This was specifially configured to work on a Linux desktop using an AMD GPU. You may have to fork that project and make some changes to get it to work with your setup.
