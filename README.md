MiniQuartz is a simple music player.
It is currently non-functional

The reason this exists is:
- I fucking hate Spotify, but it holds a grip on my soul unlike any other app. The only solution is to invest so much time into making an alternative that I *have* to use it.
- I want to learn Rust

Goals, in order of priority:
1. Fast and lightweight cross-platform file playback. Windows and Android are the main focus, but Linux would be nice to have.
2. Soulseek (or torrent) search in-app
3. Last.fm implementation for recommendation algorithm

If all goals are met, then it should provide a similar experience to streaming platforms, while being entirely local files.


Currently using egui for the GUI, and gstreamer for the audio playback.