//use std::collections::{binary_heap::{IntoIter, Iter}, hash_map::Iter};
use anyhow;
use egui::{Color32, Id, Modal, ScrollArea};
use gstreamer::prelude::*; // $env:PKG_CONFIG_PATH="C:\Program Files\gstreamer\1.0\msvc_x86_64\lib\pkgconfig"
use gstreamer::tags;
use image::imageops::FilterType;
use serde::Deserialize;
use serde::Serialize;
use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::fs::File;
use std::hash::{Hash, Hasher};
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, Sender};

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct TemplateApp {
    #[serde(skip)]
    songs: Songs,

    #[serde(skip)]
    row_height: Option<f32>,

    #[serde(skip)]
    col1_width: Option<f32>,

    col2_width: Option<f32>,

    #[serde(skip)]
    playbin: gstreamer::Element,

    #[serde(skip)]
    position_ms: u64,

    #[serde(skip)]
    duration_ms: u64,

    #[serde(skip)]
    last_update: std::time::Instant,

    #[serde(skip)]
    error_show: bool,

    #[serde(skip)]
    error_value: String,

    volume: f32,

    #[serde(skip)]
    folders: Vec<std::path::PathBuf>,

    #[serde(skip)]
    playlists: Vec<std::path::PathBuf>,

    currently_selected_playlist: Option<String>,
    currently_selected_playlist_path: Option<PathBuf>,

    now_playing: Option<PathBuf>,

    now_playing_song: Option<SongCardData>,

    #[serde(skip)]
    metadata_receiver: Receiver<MetadataResult>,
    #[serde(skip)]
    metadata_sender: Sender<MetadataRequest>,

    title_header_width: f32,
    total_header_width: f32,

    //popup
    align4: egui::RectAlign,
    gap: f32,
    #[serde(skip)]
    close_behavior: egui::PopupCloseBehavior,
    popup_open: bool,
    checked: bool,
    color: egui::Color32,
}

fn get_folders(path: &str) -> std::io::Result<Vec<PathBuf>> {
    let entries = fs::read_dir(path)?; // Read the directory contents
    let folders = entries
        .filter_map(|entry| entry.ok()) // Ignore entries with errors (e.g., permission issues)
        .filter(|entry| entry.path().is_dir()) // Keep only directories
        .map(|entry| entry.path()) // Convert DirEntry to PathBuf
        .collect();
    Ok(folders)
}

fn get_playlists(path: &str) -> std::io::Result<Vec<PathBuf>> {
    let entries = fs::read_dir(path)?;
    let json_files = entries
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            let path = entry.path();
            path.is_file() && path.extension().and_then(|ext| ext.to_str()) == Some("json")
        })
        .map(|entry| entry.path())
        .collect();
    Ok(json_files)
}

fn add_to_playlist(file_path: &str, new_song: &str) -> Result<(), Box<dyn std::error::Error>> {
    let file = File::open(file_path)?;
    let reader = BufReader::new(file);
    let mut data: serde_json::Value = serde_json::from_reader(reader)?;

    if let Some(locations) = data
        .get_mut("song_locations")
        .and_then(|v| v.as_array_mut())
    {
        locations.push(serde_json::json!(new_song));
    } // else {error; couldn't find song_locations in playlist file}

    let mut file = File::create(file_path)?;
    let formatter = serde_json::ser::PrettyFormatter::with_indent(b"    ");
    let mut ser = serde_json::Serializer::with_formatter(&mut file, formatter);
    data.serialize(&mut ser)?;

    Ok(())
}

fn remove_from_playlist(
    file_path: &str,
    target_song: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let file = File::open(file_path)?;
    let reader = BufReader::new(file);
    let mut data: serde_json::Value = serde_json::from_reader(reader)?;

    if let Some(locations) = data
        .get_mut("song_locations")
        .and_then(|v| v.as_array_mut())
    {
        if let Some(index) = locations.iter().position(|x| x == target_song) {
            locations.remove(index);
        } // else error could not find song
    } // else error could not find song_locations arrayxz

    let mut file = File::create(file_path)?;
    let formatter = serde_json::ser::PrettyFormatter::with_indent(b"    ");
    let mut ser = serde_json::Serializer::with_formatter(&mut file, formatter);
    data.serialize(&mut ser)?;

    Ok(())
}

impl Default for TemplateApp {
    fn default() -> Self {
        gstreamer::init().expect("Failed to init GStreamer"); // todo: expect should be an error message

        let (result_tx, result_rx) = std::sync::mpsc::channel::<MetadataResult>();
        let (req_tx, req_rx) = std::sync::mpsc::channel::<MetadataRequest>();

        std::thread::spawn(move || {
            let discoverer =
                gstreamer_pbutils::Discoverer::new(gstreamer::ClockTime::from_seconds(5))
                    .expect("Failed to create discoverer");

            while let Ok(request) = req_rx.recv() {
                if let Ok(metadata) = get_metadata(&discoverer, request.path.clone()) {
                    let _ = result_tx.send(MetadataResult {
                        path: request.path,
                        data: metadata,
                    });
                }
            }
        });

        let pb = gstreamer::ElementFactory::make("playbin")
            .build()
            .expect("Could not create playbin");

        Self {
            songs: Songs::new(&std::path::PathBuf::from("./playlists/")),
            row_height: None,
            col1_width: None,
            col2_width: None,
            playbin: pb,
            position_ms: 0,
            duration_ms: 0,
            last_update: std::time::Instant::now(),

            error_show: false,
            error_value: "No error message".to_owned(),

            volume: 1.0,

            folders: get_folders("./playlists/").unwrap_or_default(),
            playlists: get_playlists("./playlists/").unwrap_or_default(),

            currently_selected_playlist: None,
            currently_selected_playlist_path: Some(std::path::PathBuf::from("")),

            now_playing: None,

            now_playing_song: Some(SongCardData {
                title: "".to_owned(),
                artist: "none".to_owned(),  // todo: metadata
                length: "--:--".to_owned(), // todo: parse
                album: "none".to_owned(),
                cover_path: "".to_owned(), //todo: metadata
                path: std::path::PathBuf::from(""),
                texture: None,
                playing: false,
                metadata_loaded: false,
                display: true,
            }),

            metadata_receiver: result_rx,
            metadata_sender: req_tx,

            title_header_width: 250.0,
            total_header_width: 0.0,

            //popup demo
            align4: egui::RectAlign::default(),
            gap: 4.0,
            close_behavior: egui::PopupCloseBehavior::CloseOnClick,
            popup_open: false,
            checked: true,
            color: egui::Color32::RED,
        }
    }
}

impl TemplateApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        cc.egui_ctx
            .style_mut(|s| s.interaction.selectable_labels = false);
        let mut app: Self = if let Some(storage) = cc.storage {
            eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default()
        } else {
            Default::default()
        };

        app.initialize_gstreamer();

        if let Some(path) = app.currently_selected_playlist_path.as_ref() {
            if path.exists() {
                // Check bc user may have deleted folder
                app.songs = Songs::new(path);
            } else {
                app.currently_selected_playlist = Some("Playlist not found".to_owned());
            }
        }

        app
    }

    fn apply_options<'a>(&self, popup: egui::Popup<'a>) -> egui::Popup<'a> {
        popup
            .align(self.align4)
            .gap(self.gap)
            .close_behavior(self.close_behavior)
    }

    fn nested_menus(&mut self, ui: &mut egui::Ui, song_string: &str, song_data: SongCardData) {
        ui.set_max_width(200.0); // To make sure we wrap long text

        ui.menu_button("Add to playlist", |ui| {
            for playlist in &self.playlists {
                let playlist_name = path_to_string_name(playlist);
                let playlist_path = path_to_string(&playlist.to_path_buf());
                if ui.button(&playlist_name).clicked() {
                    let _ = add_to_playlist(&playlist_path, song_string);
                    if Some(playlist_name.clone()) == self.currently_selected_playlist {
                        self.songs.articles.extend([song_data.clone()]);
                    }
                }
            }
            let _ = ui.button("todo - New Playlist & Playlist Folders");
        });
        if ui.button("Remove from playlist").clicked() {
            let playlist_path = path_to_string(
                &self
                    .currently_selected_playlist_path
                    .as_ref()
                    .unwrap()
                    .to_path_buf(),
            );
            let _ = remove_from_playlist(&playlist_path, song_string);
            if let Some(index) = self.songs.articles.iter().position(|x| x == &song_data) {
                self.songs.articles[index].display = false;
            }
        }
    }

    fn initialize_gstreamer(&mut self) {
        gstreamer::init().expect("Failed to init GStreamer");
        self.playbin = gstreamer::ElementFactory::make("playbin")
            .build()
            .expect("Could not create playbin");
    }
}

struct Songs {
    articles: Vec<SongCardData>,
}

#[derive(Clone, serde::Deserialize, serde::Serialize, PartialEq)] // This is so serde knows wat 2 do. Using serde here to store the last playing song
struct SongCardData {
    title: String,
    artist: String,
    album: String,
    length: String,
    cover_path: String,
    path: std::path::PathBuf,
    #[serde(skip)]
    texture: Option<egui::TextureHandle>,
    playing: bool,
    metadata_loaded: bool,
    display: bool,
}

#[derive(Deserialize)]
struct SongManifest {
    song_locations: Vec<PathBuf>,
}

impl Songs {
    pub fn new(json_path: &PathBuf) -> Songs {
        let file = match File::open(json_path) {
            Ok(f) => f,
            Err(_) => {
                //eprintln!("Warning: Could not open playlist file.");
                return Songs { articles: vec![] };
            }
        };

        let reader = BufReader::new(file);

        let manifest: SongManifest = match serde_json::from_reader(reader) {
            Ok(m) => m,
            Err(_) => {
                //eprintln!("Warning: Playlist JSON is malformed.");
                return Songs { articles: vec![] };
            }
        };

        let iter = manifest.song_locations.into_iter().map(|path| {
            let file_name = path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "Unknown Track".to_string());

            SongCardData {
                title: file_name,
                artist: "Unknown Artist(1)".to_owned(),
                album: "Unknown Album(1)".to_owned(),
                length: "--:--".to_owned(),
                cover_path: "".to_owned(),
                path,
                texture: None,
                playing: false,
                metadata_loaded: false,
                display: true,
            }
        });

        Songs {
            articles: Vec::from_iter(iter),
        }
    }
    pub fn new_from_folder(folder_path: &Path) -> Songs {
        let audio_extensions = ["mp3", "wav", "ogg", "flac", "m4a"];

        let iter = fs::read_dir(folder_path)
            .into_iter() // Handle potential errors reading the folder
            .flatten()
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.path())
            .filter(|path| {
                path.is_file()
                    && path
                        .extension()
                        .and_then(|ext| ext.to_str())
                        .map(|ext| audio_extensions.contains(&ext.to_lowercase().as_str()))
                        .unwrap_or(false)
            })
            .map(|path| {
                let file_name = path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| "Unknown Track".to_string()); // unwrap or else probably not needed here, every file has a name right?

                SongCardData {
                    title: file_name,
                    artist: "Unknown Artist".to_owned(), // todo: metadata
                    album: "Unknown Album".to_owned(),
                    length: "--:--".to_owned(), // todo: parse
                    cover_path: "".to_owned(),  //todo: metadata
                    path: path.clone(),         // at most adds 20kb of memory use
                    texture: None,
                    playing: false,
                    metadata_loaded: false,
                    display: true,
                }
            });
        Songs {
            articles: Vec::from_iter(iter),
        }
    }
}
impl SongCardData {
    //i must be for real this section is written by ai. im Sorry. but im fuck at rust,, this should be checked later, though.
    fn load_texture_if_needed(&mut self, ctx: &egui::Context) {
        if self.texture.is_none() {
            if let Ok(image) = image::open(&self.cover_path) {
                let image = image.to_rgba8();
                let size = [image.width() as usize, image.height() as usize];
                let texture = ctx.load_texture(
                    self.cover_path.clone(),
                    egui::ColorImage::from_rgba_unmultiplied(size, &image),
                    Default::default(),
                );
                self.texture = Some(texture);
            }
        }
    }
}

/*fn uri_to_path(uri: &str) -> Result<PathBuf, String> {
    Url::parse(uri)
        .map_err(|e| e.to_string())?
        .to_file_path()
        .map_err(|_| "Invalid URI".into())
}*/

fn to_base62(mut n: usize, width: usize) -> String {
    let charset = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";
    let mut result = Vec::new();

    if n == 0 {
        result.push(charset[0]);
    } else {
        while n > 0 {
            result.push(charset[n % 62]);
            n /= 62;
        }
    }
    
    while result.len() < width {
        result.push(charset[0]);
    }

    result.reverse();
    String::from_utf8(result).unwrap_or_else(|_| "0000".to_string())
}

fn path_to_string(path: &PathBuf) -> String {
    let stringpath = path.as_path().to_string_lossy().to_string();
    stringpath
}

fn path_to_string_name(path: &PathBuf) -> String {
    let file_name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "Unknown".to_string());
    file_name
}

fn path_to_uri(path: std::path::PathBuf) -> String {
    let abs_path = path.canonicalize().unwrap_or(path.clone());
    let path_str = abs_path.to_string_lossy().to_string();

    let cleaned_path = path_str // this will probably need to be changed for android. God how the hell do you builkd for Android. Rafgh.
        .replace("\\\\?\\", "")
        .replace("\\", "/");

    let uri = format!("file:///{}", cleaned_path);
    uri
}

fn show_error(app: &mut TemplateApp, error: String) {
    app.error_value = error;
    app.error_show = true;
}

fn play_song(app: &mut TemplateApp, path: std::path::PathBuf) {
    let _ = app.playbin.set_state(gstreamer::State::Null);

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
        let _ = app.playbin.set_state(gstreamer::State::Null);
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

struct Metadata {
    title: String,
    artist: String,
    album: String,
    cover_path: String,
}

fn get_metadata(
    discoverer: &gstreamer_pbutils::Discoverer,
    path: std::path::PathBuf,
) -> Result<Metadata, anyhow::Error> {
    let uri = path_to_uri(path);
    let info = discoverer.discover_uri(&uri)?;

    let tags = info.tags();

    let title = tags
        .as_ref()
        .and_then(|t| t.get::<tags::Title>())
        .map(|t| t.get().to_string())
        .unwrap_or_else(|| "".to_string());

    let artist = tags
        .as_ref()
        .and_then(|t| t.get::<tags::Artist>())
        .map(|a| a.get().to_string())
        .unwrap_or_else(|| "Unknown Artist".to_string());

    let album = tags
        .as_ref()
        .and_then(|t| t.get::<tags::Album>())
        .map(|al| al.get().to_string())
        .unwrap_or_else(|| "Unknown Album".to_string());

    let cover_data = tags
        .as_ref()
        .and_then(|t| {
            t.get::<tags::Image>()
                .or_else(|| t.get::<tags::Attachment>())
        })
        .and_then(|sample_obj| {
            let sample = sample_obj.get();
            let buffer = sample.buffer()?;
            let map = buffer.map_readable().ok()?;
            Some(map.as_slice().to_vec())
        });

    let mut hasher = DefaultHasher::new();
    if album != "Unknown Album" && artist != "Unknown Artist" {
        format!("{}{}", album, artist).hash(&mut hasher); // this is like this so that we don't cache multiple of the same cover
    } else {
        uri.hash(&mut hasher);
    }
    let unique_id = hasher.finish();
    let output_path_str = format!("cache/cover_{}.jpg", unique_id); // todo: figure out how to get a unique id for each song, this is not the right way to do this. should be based off file name and location + metadata
    let output_path = PathBuf::from(output_path_str.clone());

    if let Some(parent) = output_path.parent() {
        // checking if cache folder exists
        std::fs::create_dir_all(parent).unwrap_or_default();
    }

    if let Some(data) = cover_data {
        if !output_path.exists() {
            match image::load_from_memory(&data) {
                Ok(img) => {
                    let resized = img.resize(96, 96, FilterType::Lanczos3);
                    if let Err(_) = resized.save(output_path) {
                        return Ok(Metadata {
                            title: "⚠ Album cover save error".to_owned(),
                            artist: " ".to_owned(),
                            album,
                            cover_path: "assets/icon-256.png".to_owned(),
                        });
                    }
                }
                Err(e) => eprintln!("error: {}", e),
            }
        }
        return Ok(Metadata {
            title,
            artist,
            album,
            cover_path: output_path_str,
        });
    }

    Ok(Metadata {
        title,
        artist,
        album,
        cover_path: "assets/icon-256.png".to_owned(),
    })
}

struct MetadataRequest {
    path: std::path::PathBuf,
}

struct MetadataResult {
    path: std::path::PathBuf,
    data: Metadata,
}

fn load_metadata_if_needed(
    song: &mut SongCardData,
    sender: std::sync::mpsc::Sender<MetadataRequest>,
) {
    if !song.metadata_loaded {
        let _ = sender.send(MetadataRequest {
            path: song.path.clone(),
        });
        song.metadata_loaded = true;
    }
}

//（︶^︶）（︶^︶）//
//    UI STUFF    //

impl eframe::App for TemplateApp {
    /// Called by the framework to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    } //todo: deciper how the example app does this stuff; how do you add something to be saved on reboot?

    /// Called each time the UI needs repainting, which may be many times per second.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Inside update(&mut self, ctx, frame)
        let bus = self.playbin.bus().unwrap();
        for msg in bus.iter_timed(gstreamer::ClockTime::ZERO) {
            match msg.view() {
                gstreamer::MessageView::Eos(..) => {
                    let _ = self.playbin.set_state(gstreamer::State::Ready);
                    self.now_playing = None; // todo: set this to first song playlist
                }
                gstreamer::MessageView::Error(err) => {
                    show_error(self, format!("GStreamer Error: {}", err.error().to_owned()));
                }
                _ => {}
            }
        }

        // Put your widgets into a `SidePanel`, `TopBottomPanel`, `CentralPanel`, `Window` or `Area`.
        // For inspiration and more examples, go to https://emilk.github.io/egui
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            // The top panel is often a good place for a menu bar:

            egui::MenuBar::new().ui(ui, |ui| {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
                    ui.menu_button("File", |ui| {
                        if ui.button("Quit").clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                        if (ui.button("Meoooww")).clicked() {
                            show_error(self, "Meow Button Pressed".to_owned());
                        }
                    });
                    ui.add_space(16.0);
                });

                egui::widgets::global_theme_preference_buttons(ui); // where ddid they go...
            });
        });

        //--\(￣︶￣*\))---\(￣︶￣*\))---\(￣︶￣*\))---\(￣︶￣*\))--//
        //    Bottom bar to display track info and track controls    //

        egui::TopBottomPanel::bottom("status")
            .resizable(true)
            .min_height(50.0)
            .show(ctx, |ui| {
                ui.set_min_height(ui.available_height());
                ui.horizontal(|ui| {
                    ui.group(|ui| {
                        // song info
                        ui.set_width(150.0);
                        ui.horizontal(|ui| {
                            ui.label("meow");
                            ui.vertical(|ui| {
                                ui.label(self.now_playing_song.as_ref().unwrap().title.clone());
                                ui.label(self.now_playing_song.as_ref().unwrap().artist.clone());
                            });
                        });
                    });

                    ui.add_space(ui.available_width() / 2.0 - 192.0); // there is probably a better way to center things, since this requires some arbitrary numbers.

                    ui.group(|ui| {
                        // main controls
                        /////
                        // Play/pause button
                        ui.set_width(200.0);
                        ui.vertical_centered(|ui| {
                            if ui.button("Play/Pause").clicked() {
                                let (_success, current, _pending) =
                                    self.playbin.state(gstreamer::ClockTime::NONE); // todo: i think i'm checking this a few times per loop. should make this check once and set a variable
                                if current == gstreamer::State::Playing {
                                    if let Err(err) =
                                        self.playbin.set_state(gstreamer::State::Paused)
                                    {
                                        show_error(self, err.to_string());
                                    }
                                } else if current == gstreamer::State::Paused {
                                    if let Err(err) =
                                        self.playbin.set_state(gstreamer::State::Playing)
                                    {
                                        show_error(self, err.to_string());
                                    }
                                } else {
                                    let song_path =
                                        self.now_playing_song.as_ref().map(|s| s.path.clone());
                                    if let Some(path) = song_path {
                                        play_song(self, path);
                                    } else {
                                        show_error(self, "No song ready to play".to_owned()); // This should be removed in the future. Expected behaviour would be to disable the play/pause button.
                                    }
                                }
                            }

                            /////
                            // Seeking
                            let (success, state, _pending) =
                                self.playbin.state(gstreamer::ClockTime::from_mseconds(0));
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if success.is_ok()
                                        && (state == gstreamer::State::Playing
                                            || state == gstreamer::State::Paused)
                                    {
                                        let duration = self.duration_ms.max(1) as f32;
                                        let mut pos = self.position_ms as f32;
                                        let response = ui.add(
                                            egui::Slider::new(&mut pos, 0.0..=duration)
                                                .trailing_fill(true)
                                                .custom_formatter(|n, _| {
                                                    let total_seconds = (n / 1000.0) as i64;
                                                    let minutes = total_seconds / 60;
                                                    let seconds = total_seconds % 60;
                                                    format!("{:02}:{:02}", minutes, seconds)
                                                }),
                                        );
                                        if response.changed() {
                                            let seek_to =
                                                gstreamer::ClockTime::from_mseconds(pos as u64);

                                            self.playbin
                                                .seek_simple(
                                                    gstreamer::SeekFlags::FLUSH
                                                        | gstreamer::SeekFlags::KEY_UNIT,
                                                    seek_to,
                                                ) // Wow! Gstream just has that!
                                                .expect("Seek failed");
                                        }
                                    } else {
                                        ui.add_enabled(
                                            false,
                                            egui::Slider::new(&mut 0.0, 0.0..=1.0),
                                        );
                                    }
                                    if self.last_update.elapsed().as_millis() > 100 {
                                        // Set position
                                        if let Some(pos) =
                                            self.playbin.query_position::<gstreamer::ClockTime>()
                                        {
                                            self.position_ms = pos.mseconds();
                                        }
                                        self.last_update = std::time::Instant::now();
                                    }
                                },
                            );
                        });
                    });

                    /////
                    // Volume slider
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.group(|ui| {
                            // secondary controls
                            ui.set_width(150.0);

                            let response_volume = ui.add(
                                egui::Slider::new(&mut self.volume, 0.0..=1.0)
                                    .text("🔊") // egui speaker emoji looks kind of bad so replace this with something else eventually
                                    .trailing_fill(true) // 1.0 here is the max volume
                                    .show_value(false),
                            );
                            if response_volume.changed() {
                                let cubic_volume = (self.volume * self.volume * self.volume) as f64; // cubic slider & gstreamer needs f64
                                self.playbin.set_property("volume", cubic_volume);
                            }
                        });
                    });
                });

                if self.error_show {
                    Modal::new(Id::new("IO Error")).show(ui.ctx(), |ui| {
                        ui.set_width(200.0);
                        ui.heading("Error");
                        ui.label(&self.error_value);

                        ui.add_space(32.0);

                        egui::Sides::new().show(
                            ui,
                            |_ui| {},
                            |ui| {
                                if ui.button("aw dang").clicked() {
                                    self.error_show = false;
                                }

                                if ui.button("im sorry").clicked() {
                                    self.error_show = false;
                                }
                            },
                        );
                    });
                }
            });

        //--(*￣3￣)╭----(*￣3￣)╭---(*￣3￣)╭----(*￣3￣)╭--//
        // Side panel to display playlists and app controls //
        fn draw_drop_bar(ui: &mut egui::Ui, start: egui::Pos2, end: egui::Pos2) {
            let stroke = egui::Stroke::new(2.0, ui.visuals().widgets.active.bg_fill);
            ui.painter().line_segment([start, end], stroke);
            // Add little circles at the ends for a "polished" look
            ui.painter().circle_filled(start, 3.0, stroke.color);
            ui.painter().circle_filled(end, 3.0, stroke.color);
        }

        egui::SidePanel::left("playlists")
            .resizable(true)
            .min_width(30.0)
            .show(ctx, |ui| {
                ui.heading("miniQuartz");
                let fps = 1.0 / ctx.input(|i| i.stable_dt.max(0.0001)); // fps counter for extra awesome
                ui.label(format!("FPS: {:.1}", fps));
                ScrollArea::vertical().show(ui, |ui| {
                    ui.set_min_width(ui.available_width()); // this makes smooth resizing possible. feels kinda jank but whatever.
                    let list_id = ui.make_persistent_id("reorder_list_bars");
                    let dragging_index = ui.data(|d| d.get_temp::<usize>(list_id));
                    let mut drop_target_index = None;

                    if ui.selectable_label(false, "📁 Local Files").clicked() {
                        let local_path = std::path::PathBuf::from("playlists/playlist-1"); // todo: make user configurable
                        self.songs = Songs::new_from_folder(&local_path);
                        self.currently_selected_playlist = Some("Local Files".to_string());
                        self.currently_selected_playlist_path = Some(local_path);
                    }
                    for i in 0..self.playlists.len() {
                        let playlist_name = self.playlists[i]
                            .file_name()
                            .map(|n| n.to_string_lossy().to_string())
                            .unwrap_or_else(|| "Unknown".to_string()); // unwrap_or_else might be unnecessary here, since a playlist should *never* not have a name; if it didn't, it wouldn't exist.
                        let playlist_name = &playlist_name[4..];
                        let response = ui
                            .selectable_label(false, format!("📃  {}", &playlist_name))
                            .interact(egui::Sense::drag());
                        if response.drag_started() {
                            ui.data_mut(|d| d.insert_temp(list_id, i));
                        }

                        if response.dragged() {
                            ui.ctx().set_cursor_icon(egui::CursorIcon::Move);
                            egui::Tooltip::always_open(
                                ui.ctx().clone(), // is clone right here? feels wrong dunno why
                                ui.layer_id(),
                                egui::Id::new("playlist_drag_tooltip"),
                                egui::Pos2::ZERO,
                            )
                            .at_pointer()
                            .show(|ui| {
                                ui.label(playlist_name);
                            });
                        }

                        if response.clicked() {
                            self.songs = Songs::new(&self.playlists[i]);
                            self.currently_selected_playlist = Some(playlist_name.to_string());
                            self.currently_selected_playlist_path =
                                Some(self.playlists[i].to_path_buf());
                        }

                        let rect = response.rect;
                        if let (Some(pos), Some(_)) =
                            (ui.input(|i| i.pointer.interact_pos()), dragging_index)
                        {
                            if rect.contains(pos) {
                                drop_target_index =
                                    Some(if pos.y < rect.center().y { i } else { i + 1 });
                            }
                        }

                        if let Some(target) = drop_target_index {
                            // drop indicator
                            if target == i {
                                draw_drop_bar(ui, rect.left_top(), rect.right_top());
                            } else if target == self.playlists.len()
                                && i == self.playlists.len() - 1
                            {
                                draw_drop_bar(ui, rect.left_bottom(), rect.right_bottom());
                            }
                        }
                    }

                    if ui.selectable_label(false, "+").clicked() {
                        show_error(
                            self,
                            "todo: add playlist button & right click menu for playlists"
                                .to_string(),
                        );
                    }

                    if ui.input(|i| i.pointer.any_released()) {
                        if let (Some(from), Some(to)) = (dragging_index, drop_target_index) {
                            if from != to && to <= self.playlists.len() {
                                let item = self.playlists.remove(from);
                                let insert_at = if to > from { to - 1 } else { to };
                                self.playlists.insert(insert_at, item);

                                let mut count = 0;
                                for mut playlist in self.playlists.clone() {
                                    let path_string = path_to_string(&playlist);
                                    let file_name = path_to_string_name(&playlist);
                                    let clean_name: String = file_name.chars().skip(4).collect(); // todo: when program more refined, check if you need it like this or if you can just do [4..]
                                    // ^^ this is done in case a json file is ever put into folder that has less than 4 chars. shouldn't happen, but just in case.
                                    let count62 = to_base62(count, 4);
                                    playlist.set_file_name(format!("{:04}{}", count62, clean_name));
                                    let to_string = path_to_string(&playlist);
                                    self.playlists[count] = playlist; // this should probably be on a different thread, since a huge amount of playlists will cause a freeze bc disk operations

                                    if let Err(error) = fs::rename(path_string.clone(), &to_string)
                                    {
                                        show_error(
                                            self,
                                            format!(
                                                "err: {} | from: {} | to: {}",
                                                error.to_string(),
                                                path_string,
                                                to_string
                                            ),
                                        ); // this error cant be closed. dunno why!
                                    }
                                    count += 1;
                                }
                            }
                        }
                        ui.data_mut(|d| d.remove::<usize>(list_id));
                    }
                });
            });

        //--◑﹏◐---◑﹏◐---◑﹏◐---◑﹏◐---◑﹏◐---◑﹏◐---◑﹏◐---◑﹏◐---◑﹏◐---◑﹏◐-//
        //   Central panel to display: Playlist contents, album contents, artist pages  //

        egui::CentralPanel::default().show(ctx, |ui| {
            // central panel has to be rendered after other panels
            egui::TopBottomPanel::top("Header")
                .resizable(false)
                .min_height(50.0)
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        let playlist_name = self
                            .currently_selected_playlist
                            .as_deref()
                            .unwrap_or("No playlist selected");
                        ui.label(egui::RichText::new(playlist_name).size(32.0).strong());
                    });
                });
            egui::TopBottomPanel::top("Resizables")
                .resizable(false)
                //.min_height(50.0)
                .show(ctx, |ui| {
                    self.total_header_width = ui.available_width();
                    ui.horizontal(|ui| {
                        ui.label("#");
                        ui.separator();
                        ui.spacing_mut().item_spacing.x = 0.0;
                        egui::Resize::default()
                            .id_salt("Title")
                            .default_width(100.0)
                            .max_width(self.total_header_width - 110.0)
                            .min_height(ui.available_height())
                            .max_height(ui.available_height())
                            .with_stroke(false)
                            .show(ui, |ui| {
                                ui.label("Title");
                                self.title_header_width = ui.available_width();
                            });

                        ui.separator();

                        ui.label("Album");

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.add_space(17.0);
                            ui.label("🕒");
                            ui.separator();
                        });
                    });
                });
            egui::CentralPanel::default().show(ctx, |ui| {
                // Nested panels don't usually behave well but it works so idk lol.
                ScrollArea::vertical()
                    //.max_width(available_width-5.0)
                    .show(ui, |ui| {
                        // render buffer stuff
                        let row_height = self.row_height.unwrap_or(30.0); // proper row height: it feels wrong to be setting this every frame.
                        let total_rows = self.songs.articles.len(); // it feels wrong to be setting this every frame. this only really needs to be set if the shown list changes.

                        let clip_rect = ui.clip_rect();
                        let top = clip_rect.top();
                        let bottom = clip_rect.bottom();

                        let mut start = ((top - ui.min_rect().top()) / row_height).floor() as usize;
                        let mut end = ((bottom - ui.min_rect().top()) / row_height).ceil() as usize;

                        let render_buffer_size = 8; // If fast scrolling causes metadata not to load, increase this.

                        start = start.saturating_sub(render_buffer_size);
                        end = (end + render_buffer_size).min(total_rows);

                        let above_px = start as f32 * row_height;
                        ui.add_space(above_px); // makes scroll bar look big (1/2)
                        let mut clicked_song_index = None;

                        for result in self.metadata_receiver.try_iter().take(20) {
                            for song in self
                                .songs
                                .articles
                                .iter_mut()
                                .filter(|s| s.path == result.path)
                            {
                                song.album = result.data.album.clone();
                                song.artist = result.data.artist.clone();
                                if !result.data.title.is_empty() {
                                    song.title = result.data.title.clone();
                                }
                                song.cover_path = result.data.cover_path.clone();
                            }
                        }
                        for i in start..end {
                            let song = &mut self.songs.articles[i];
                            if song.display {
                                load_metadata_if_needed(song, self.metadata_sender.clone());
                                song.load_texture_if_needed(ctx);
                                ui.spacing_mut().item_spacing.y = 0.0;
                                let response = ui
                                    .scope_builder(
                                        egui::UiBuilder::new()
                                            .id_salt("song_card")
                                            .sense(egui::Sense::click()),
                                        |ui| {
                                            let response = ui.response();
                                            let visuals = ui.style().interact(&response);
                                            let fill_color =
                                                if response.hovered() || response.has_focus() {
                                                    visuals.bg_fill.gamma_multiply(0.3)
                                                } else {
                                                    egui::Color32::TRANSPARENT
                                                };
                                            egui::Frame::new()
                                                .fill(fill_color)
                                                //.stroke(visuals.bg_stroke)
                                                .inner_margin(ui.spacing().menu_margin)
                                                .show(ui, |ui| {
                                                    ui.horizontal(|ui| {
                                                        ui.label((i + 1).to_string());
                                                        ui.scope(|ui| {
                                                            ui.set_width(
                                                                self.title_header_width + 25.0,
                                                            );
                                                            if let Some(tex) = &song.texture {
                                                                ui.add(
                                                                    egui::Image::new(tex)
                                                                        .max_width(30.0)
                                                                        .corner_radius(3), // todo: this should be user configurable. some people haaate corner radius on album art
                                                                );
                                                            } else {
                                                                ui.add(
                                                                    egui::Spinner::new()
                                                                        .size(30.0)
                                                                        .color(egui::Color32::BLUE),
                                                                );
                                                            }
                                                            ui.vertical(|ui| {
                                                                // song & artist names
                                                                let color = if self.now_playing // todo: this check should be based on file *and* playlist!
                                                                == Some(song.path.clone())
                                                                {
                                                                    Color32::from_rgb(255, 128, 0) // make this configurable later
                                                                } else {
                                                                    ui.visuals().strong_text_color()
                                                                };
                                                                ui.add(
                                                                    egui::Label::new(
                                                                        egui::RichText::new(
                                                                            &song.title,
                                                                        )
                                                                        .color(color),
                                                                    )
                                                                    .truncate(),
                                                                );
                                                                ui.add(
                                                                    egui::Label::new(&song.artist)
                                                                        .truncate(),
                                                                );
                                                            });
                                                        });
                                                        let remaining_width =
                                                            ui.available_width() - 60.0;
                                                        ui.allocate_ui_with_layout(
                                                            egui::vec2(
                                                                remaining_width,
                                                                ui.available_height(),
                                                            ),
                                                            egui::Layout::left_to_right(
                                                                egui::Align::Center,
                                                            ),
                                                            |ui| {
                                                                ui.add(
                                                                    egui::Label::new(&song.album)
                                                                        .truncate(),
                                                                );
                                                            },
                                                        );
                                                        ui.with_layout(
                                                            egui::Layout::right_to_left(
                                                                egui::Align::TOP,
                                                            ),
                                                            |ui| {
                                                                ui.add_space(10.0);
                                                                ui.label(format!(
                                                                    "{}",
                                                                    &song.length
                                                                ));
                                                            },
                                                        );
                                                    });
                                                });
                                        },
                                    )
                                    .response;
                                if response.double_clicked() {
                                    clicked_song_index = Some(i);
                                }

                                if self.row_height.is_none() {
                                    self.row_height = Some(response.rect.height()); // todo: this is in the for loop and is probably fuck for performance \(￣︶￣*\))
                                } // this really only needs to be done on startup

                                if self.now_playing == Some(song.path.clone()) {
                                    // todo: this check should be based on file *and* playlist!
                                    ui.painter().rect_filled(
                                        response.rect,
                                        4.0,
                                        egui::Color32::from_white_alpha(10),
                                    );
                                }
                                let path_string = path_to_string(&song.path);
                                let song_send = song.clone();
                                self.apply_options(
                                    egui::Popup::context_menu(&response)
                                        .id(Id::new(format!("context_menu{}", i))),
                                )
                                .show(|ui| self.nested_menus(ui, &path_string, song_send));
                            }
                        }

                        if let Some(idx) = clicked_song_index {
                            let song = &self.songs.articles[idx];
                            let path = song.path.clone();

                            self.now_playing = Some(path.clone());
                            self.now_playing_song = Some(song.clone());

                            play_song(self, path);
                        }

                        let remaining_px = (total_rows - end) as f32 * row_height; //      <- part of render buffer
                        ui.add_space(remaining_px); // makes scroll bar look big (2/2)  <- part of render buffer
                    });
            });

            ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                egui::warn_if_debug_build(ui); // this was in the example thing and idk if its needed or if theres a benefit to removing it
            });
        });
        ctx.request_repaint_after(std::time::Duration::from_millis(300)); // Updates UI every 300ms, so that the duration bar moves smoothly when tabbed out
    }
}
