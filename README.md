# YTRS

A work in progress YouTube client and InnerTube API library in Rust. Originally based on [YouTube.JS](https://github.com/LuanRT/YouTube.js/).

Built to solve a specific problem: watching YouTube content in its original language.

## Screenshots

![Search view](ytrs-client/screenshots/1.png)
![Channel view](ytrs-client/screenshots/2.png)

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

- Search with language override
- Channel browsing (videos/shorts/streams tabs)
- Sort filters
- Video playback via mpv
- Persistent configuration
- Responsive layout

**Requirements:** [mpv](https://mpv.io/) and [yt-dlp](https://github.com/yt-dlp/yt-dlp) for video playback

## Installation

### Dependencies

Install mpv and yt-dlp:

**Note:** Package repositories often have outdated yt-dlp versions. Consider following the [official installation instructions](https://github.com/yt-dlp/yt-dlp#installation) instead.

**Arch Linux:**
```bash
sudo pacman -S mpv yt-dlp
```

**Ubuntu/Debian:**
```bash
sudo apt install mpv yt-dlp
```

**NixOS:**
```bash
nix-env -iA nixpkgs.mpv nixpkgs.yt-dlp
```

Or add to `configuration.nix`:
```nix
environment.systemPackages = with pkgs; [
  mpv
  yt-dlp
];
```

**macOS:**
```bash
brew install mpv yt-dlp
```

**Windows:**
- Download mpv from [mpv.io](https://mpv.io/installation/)
- Install yt-dlp: `pip install yt-dlp` or download from [releases](https://github.com/yt-dlp/yt-dlp/releases)

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
