MiniQuartz is a simple music player.
It is currently non-functional

The reason this exists is:
- I fucking hate Spotify, but it holds a grip on my soul unlike any other app. The only solution is to invest so much time into making an alternative that I *have* to use it.
- I want to learn Rust

Goals, in order of priority:
- [ ] - Fast and lightweight cross-platform file playback. Windows and Android are the main focus, but Linux would be nice to have too. Basic features; queue, playlists, search, no audio stutter at high load

- [ ] - Soulseek (or torrent) search in-app. Search bar & results should behave similarly to the ones on streaming services.

- [ ] - Last.fm implementation: Artist pages, recommended playlists, scrobbles, show what's now playing on last.fm

If all goals are met, then it should provide a similar experience to streaming platforms, while being entirely local files.


Currently using egui for the GUI, and gstreamer for the audio playback.

### Development instructions for GStreamer
MiniQuartz doesn't currently compile GStreamer in the application. You must add it manually for now:
Instructions are for Windows, linux users can probably figure it out ( •̀ ω •́ )✧ - but really, it should be compiled into the application in the future.

1. Download the MSVC Runtime and MSVC Development Runtime
2. Copy the `lib` folder, and the *contents* of the `bin` folder from `gstreamer/1.0/msvc_xxx_xx` into `MiniQuartz/target/release`