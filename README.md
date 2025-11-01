# YTRS

This is a work in progress InnerTube API YouTube library written in Rust, originally based on [YouTube.JS](https://github.com/LuanRT/YouTube.js/).

This project was mainly created because I wanted to understand the InnerTube API. Also out of necessity to build a Youtube client that allows me to watch and search for videos in the language they were created in.

It's mainly maintained for my personal use.

### What makes this different from anything else?

- The search function tries to determine the language of your search keywords using [whatlang](https://crates.io/crates/whatlang) and [lingua](https://crates.io/crates/lingua). It then sets that as the locale when querying the InnerTube API. Results are then hopefully returned in the determined language.
- The channel video list fetch function does the same as the search function, but uses the channel description or title to try to determine the language.

*Why?*
- I'm tired of being shown videos with poorly translated english titles when I can read the original language.
- I don't want to manually change the locale everytime I want to search

*limitations*
- YouTube also uses your location (IP) to feed you local videos. It does not matter what locale you set. You will still be served those when searching.

*AI Disclosure*
- I experimented with Claude in supervised mode to create this. I don't commit code I don't understand. There are however many improvements that can be made to the code structure.

## YTRS client

This is a work in progress YTRS client written in rust using [Iced.rs](https://iced.rs/) for the graphical user interface.

### Functionality
- Search
- Channel browsing with separate tabs as well as sorting
  - Videos
  - Shorts
  - Streams
- Playing videos in mpv (requires [mpv](https://mpv.io/) and [yt-dlp](https://github.com/yt-dlp/yt-dlp))
