use gstreamer::prelude::*; // $env:PKG_CONFIG_PATH="C:\Program Files\gstreamer\1.0\msvc_x86_64\lib\pkgconfig"

use crate::TemplateApp;
use crate::utilities::{path_to_uri, show_error};

/// PLAYBACK ///
/// Functions for managing the audio stream
pub fn play_song(app: &mut TemplateApp, path: std::path::PathBuf) {
    if let Err(e) = app.playbin.set_state(gstreamer::State::Null){
        println!("play_song GStreamer error: {}",e);
    }

    let cubic_volume = (app.volume * app.volume * app.volume) as f64; // cubic slider & gstreamer needs f64
    app.playbin.set_property("volume", cubic_volume); // set volume when you play a song for some reason i forgot. check probably. didnt make a comment b4, whoops

    let uri = path_to_uri(path.clone());

    app.playbin.set_property("uri", &uri);

    if let Err(_) = app.playbin.set_state(gstreamer::State::Playing) {
        show_error(
            app,
            "GStreamer: State change failed. Check if file exists or audio device is ready."
                .to_owned(),
        );
        eprintln!("GStreamer StateChangeError @ play_song: {}", "State change failed. Check if file exists or audio device is ready.".to_string());
        if let Err(e) = app.playbin.set_state(gstreamer::State::Null){
            eprintln!("play_song GStreamer error: {}",e);
        }
    } else {
        app.now_playing = Some(path);
        app.duration_ms = 0;
        while app.duration_ms == 0 {
            // this while loop is here because querying immediately returns 0. i believe gstreamer checks the duration in a diff thread, but this function would otherwise end before it can get it.
            if let Some(dur) = app.playbin.query_duration::<gstreamer::ClockTime>() {
                app.duration_ms = dur.mseconds();
            }
        }
    }
}

pub fn initialize_gstreamer(app: &mut TemplateApp) {
    gstreamer::init().expect("Failed to init GStreamer");
    app.playbin = gstreamer::ElementFactory::make("playbin")
        .build()
        .expect("Could not create playbin");
}
