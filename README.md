## MiniQuartz is a "simple" music player.
Current state: `Barely Functional`

The reason this exists is:
- Available music players sacrifice convenience for locality. This scares away most users, especially when switching from streaming platforms. But local files can be convenient, too!
- Most major streaming platforms are, in my opinion, genuinely evil. While MiniQuartz may not pay artists, I hope that the money saved can be directly sent to your favourites instead of funding wars & harming artists.
- Frankly, we can do better than Spotify `(￢‿￢ )` 

Goals, in order of priority:
- [ ] - Fast and lightweight cross-platform file playback. Windows and Android are the main focus, but Linux would be nice to have too. Basic features; queue, playlists, search, no audio stutter at high load

- [ ] - Soulseek (and/or torrent) search in-app. Search bar & results should behave similarly to the ones on streaming services.

- [ ] - Last.fm implementation: Artist pages, recommended playlists, scrobbles, show what's now playing on last.fm

If all goals are met, then it should provide a similar experience to streaming platforms, while being entirely local files.


Currently using egui for the GUI, and gstreamer for the audio playback.

### How to contribute
Check out the issues page, and pick (or create) something that someone isn't already working on. Make a fork, make a pr `(≧◡≦)`
We're not super strict, so most things will get merged.

### Development instructions for GStreamer
MiniQuartz doesn't currently compile GStreamer in the application. You must add it manually for now:
Instructions are for Windows, linux users can probably figure it out `( •̀ ω •́ )✧` - but really, it should be compiled into the application in the future.

1. Download the MSVC Runtime and MSVC Development Runtime
2. Copy the `lib` folder, and the *contents* of the `bin` folder from `gstreamer/1.0/msvc_xxx_xx` into `MiniQuartz/target/release`