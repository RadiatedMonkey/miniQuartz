use std::fs;
use std::fs::File;
use std::io::{BufRead, BufReader, Error, Write};
use std::path::{Path, PathBuf};

use crate::TemplateApp;
use crate::utilities::{path_to_string, path_to_string_name, show_error, to_base62};

/// PLAYLIST ///
/// Song management & organization
pub struct Songs {
    pub articles: Vec<SongCardData>,
}

#[derive(Clone, serde::Deserialize, serde::Serialize, PartialEq)] // This is so serde knows wat 2 do. Using serde here to store the last playing song
pub struct SongCardData {
    pub title: String,
    pub artist: String,
    pub album: String,
    pub length_string: String,
    pub cover_path: String,
    pub path: std::path::PathBuf,
    #[serde(skip)]
    pub texture: Option<egui::TextureHandle>,
    pub playing: bool,
    pub metadata_loaded: bool,
    pub display: bool,
}

impl Songs {
    pub fn new(m3u_path: &PathBuf) -> Songs {
        let playlist_entries = match read_m3u(m3u_path) {
            Ok(entries) => entries,
            Err(_) => return Songs { articles: vec![] },
        };
        let iter = playlist_entries.into_iter().map(|entry| {
            let display_title = if entry.title.is_empty() {
                Path::new(&entry.path)
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| "Unknown Track".to_string())
            } else {
                entry.title
            };
            let path = &entry.path;
            SongCardData {
                title: display_title,
                artist: entry.artist,
                album: entry.album,
                length_string: "--:--".to_owned(),
                cover_path: entry.cover_path,
                path: PathBuf::from(path),
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
                    .unwrap_or_else(|| "Unknown Track".to_string()); // unwrap_or_else probably not needed here, every file has a name right?

                SongCardData {
                    title: file_name,
                    artist: "Unknown Artist(2)".to_owned(),
                    album: "Unknown Album".to_owned(),
                    length_string: "--:--".to_owned(),
                    cover_path: "".to_owned(),
                    path: path.clone(),
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
    pub fn load_texture_if_needed(&mut self, ctx: &egui::Context) {
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

pub fn get_playlists(path: &str) -> std::io::Result<Vec<PathBuf>> {
    let entries = fs::read_dir(path)?;
    let playlist_files = entries
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            let path = entry.path();
            path.is_file() && path.extension().and_then(|ext| ext.to_str()) == Some("m3u")
        })
        .map(|entry| entry.path())
        .collect();
    Ok(playlist_files)
}

pub fn add_to_playlist(
    file_path: &str,
    new_song: &SongCardData,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut playlist = M3uPlaylist::new();

    playlist.add_track(
        &format!("{}", path_to_string(&new_song.path)),
        -1,
        &new_song.title,
        &new_song.artist,
        &format!("{}", new_song.cover_path),
        &new_song.album,
    );

    if let Err(e) = write_m3u(file_path, &playlist, false, true, false) {
        println!("add_to_playlist write_m3u error: {}", e);
    }
    Ok(())
}

pub fn remove_from_playlist(
    file_path: &str,
    index_to_remove: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut playlist = read_m3u(file_path)?;

    if index_to_remove < playlist.entries.len() {
        playlist.entries.remove(index_to_remove);
    } else {
        return Err("Index out of bounds".into());
    }

    write_m3u(file_path, &playlist, true, false, true)?;

    Ok(())
}

#[derive(Clone, PartialEq)]
pub struct PlaylistEntry {
    pub path: String,
    pub duration: i32, // -1 if unknown
    pub title: String,
    pub artist: String,
    pub album: String,
    pub cover_path: String,
}

#[derive(Clone, Default)]
pub struct M3uPlaylist {
    pub entries: Vec<PlaylistEntry>,
    pub texture: Option<egui::TextureHandle>,
}

impl IntoIterator for M3uPlaylist {
    type Item = PlaylistEntry;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.entries.into_iter()
    }
}

impl M3uPlaylist {
    pub fn new() -> Self {
        M3uPlaylist {
            entries: Vec::new(),
            texture: None,
        }
    }

    pub fn add_track(
        &mut self,
        path: &str,
        duration: i32,
        title: &str,
        artist: &str,
        cover_path: &str,
        album: &str,
    ) {
        self.entries.push(PlaylistEntry {
            path: path.to_string(),
            duration,
            title: title.to_string(),
            artist: artist.to_string(),
            cover_path: cover_path.to_string(),
            album: album.to_string(),
        });
    }
}

pub fn read_m3u<P: AsRef<Path>>(path: P) -> std::io::Result<M3uPlaylist> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);

    let mut playlist = M3uPlaylist::new();
    // Temporary storage for metadata read from the previous line
    let mut current_duration = -1;
    let mut current_title = String::new();
    let mut current_artist = String::new();
    let mut current_cover_path = String::new();
    let mut current_album = String::new();

    for line_result in reader.lines() {
        let line = line_result?;
        let trimmed = line.trim();

        if trimmed.is_empty() {
            continue;
        }

        if trimmed.starts_with("#EXTINF:") {
            let content = &trimmed[8..];
            let parts: Vec<&str> = content.split('␟').collect(); // ␟ is the "Unit Separator" symbol, not the country code.
            if parts.len() != 5 {
                println!(
                    "Read m3u error: Malformed playlist file, track missing full #EXTINF. This file may be corrupted or incompatible with MiniQuartz\nlen: {}\nline: {}",
                    parts.len(),
                    line
                ); // would be really nice if the user could see this error!
                break;
            } else {
                current_duration = parts[0].parse().unwrap_or(-1);
                current_title = parts[1].trim().to_string();
                current_artist = parts[2].trim().to_string();
                current_cover_path = parts[3].trim().to_string();
                current_album = parts[4].trim().to_string();
            }
        } else if trimmed.starts_with('#') {
            continue;
        } else {
            playlist.entries.push(PlaylistEntry {
                path: trimmed.to_string(),
                duration: current_duration,
                title: current_title.clone(),
                artist: current_artist.clone(),
                cover_path: current_cover_path.clone(),
                album: current_album.clone(),
            });

            // Reset metadata for next entry : is this necessary?
            current_duration = -1;
            current_title.clear();
            current_artist.clear();
            current_cover_path.clear();
            current_album.clear();
        }
    }

    Ok(playlist)
}

pub fn edit_m3u_track(
    file_path: &str,
    index: usize,
    album: String,
    artist: String,
    cover_path: String,
    title: String,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut playlist = read_m3u(file_path)?;
    if playlist.entries.len() < index {
        println!("{}", "edit_m3u_track: Index out of bounds for m3u file");
    } else {
        let entry = &playlist.entries[index];
        // albums are equaling true
        if entry.album != album {
            println!("ALBUM: entry: {}\nALBUM: actual: {}", entry.album, album)
        }
        if entry.artist != artist {
            println!(
                "ARTIST: entry: {}\nARTIST: actual: {}",
                entry.artist, artist
            )
        }
        if entry.title != title {
            println!("TITLE: entry: {}\nTITLE: actual: {}", entry.title, title);
            println!("{}", if (entry.title == title) { true } else { false });
        }
        if entry.cover_path != cover_path {
            println!(
                "COVER: entry: {}\nCOVER: actual: {}",
                entry.cover_path, cover_path
            )
        }

        if entry.album != album
            || entry.artist != artist
            || entry.cover_path != cover_path
            || entry.title != title
        {
            if index < playlist.entries.len() {
                playlist.entries[index].album = album;
                playlist.entries[index].artist = artist;
                playlist.entries[index].cover_path = cover_path;
                playlist.entries[index].title = title;
                write_m3u(file_path, &playlist, true, false, true)?;
                //println!("{}","Done: Edited m3u track")
                // not setting path bc this already gets the path from the playlist file. they will always be equal.
                // not setting length bc i dont think songs change in length often enough to warrant it
                println!("{}", "Done: Edited m3u track")
            } else {
                return Err("Index out of bounds".into());
            }
        }
    }
    Ok(())
}

pub fn move_m3u_track(file_path: &str, from: usize, to: usize) -> std::io::Result<()> {
    // todo: make this function return an error if it has an error
    let mut playlist = read_m3u(file_path).unwrap(); // todo: check for valid result from read_m3u

    if playlist.entries.len() <= from {
        eprintln!(
            "move_m3u_track index out of bounds error | from: {} | len: {}",
            from,
            playlist.entries.len()
        );
        return Err(Error::new(
            std::io::ErrorKind::Other,
            format!(
                "move_m3u_track index out of bounds error | from: {} | len: {}",
                from,
                playlist.entries.len()
            ),
        ));
    }
    let entry = playlist.entries.remove(from); // i wonder if there is a better way of doing this? .remove() has poor performance at huge playlist sizes.
    let insert_at = if from < to { to - 1 } else { to };

    //if from >= playlist.entries.len() || insert_at >= playlist.entries.len(){
    //    return Err(Error::new(std::io::ErrorKind::Other, format!("playlist::move_m3u_track : Index failure. From:{}, To:{}, Len:{}",from,to,playlist.entries.len())));
    //}
    /* im not really sure whats going on that is causing this check to be freaky? can't explain just check it out and try moving songs to/from the very bottom of a playlist.
    i guess it's not really a big deal cus this shouldn't ever trigger.. but: todo: fix this error check */

    playlist.entries.insert(insert_at, entry);

    if let Err(e) = write_m3u(file_path, &playlist, true, false, true) {
        println!("move_m3u_track write_m3u error: {}", e);
    }

    Ok(())
}

pub fn write_m3u<P: AsRef<Path>>(
    path: P,
    playlist: &M3uPlaylist,
    write_header: bool,
    append: bool,
    overwrite: bool,
) -> std::io::Result<()> {
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .append(append)
        .create(true) // if it doesn't exist it'll make it, this is useful for the New Playlist option in the right click menu on song cards
        .open(&path)?;

    if overwrite {
        file = std::fs::File::create(path)?;
    }

    if write_header {
        writeln!(file, "#EXTM3U")?;
    }

    for entry in &playlist.entries {
        // Only write metadata if requested AND if useful data exists
        if !entry.title.is_empty() || entry.duration != -1 {
            let dur = if entry.duration == -1 {
                0
            } else {
                entry.duration
            };
            let title = if entry.title.is_empty() {
                "Unknown Title"
            } else {
                &entry.title
            };
            let artist = if entry.artist.is_empty() {
                "Unknown Artist(1)"
            } else {
                &entry.artist
            };
            let cover_path = if entry.cover_path.is_empty() {
                "./assets/icon-256.png"
            } else {
                &entry.cover_path
            };
            let album = if entry.album.is_empty() {
                "Unknown Album"
            } else {
                &entry.album
            };

            // Format: #EXTINF:seconds,Title
            writeln!(
                file,
                "#EXTINF:{}␟{}␟{}␟{}␟{}",
                dur, title, artist, cover_path, album
            )?;
        }
        // Write the actual file path
        writeln!(file, "{}", entry.path)?;
    }

    Ok(())
}

pub fn create_empty_m3u<P: AsRef<Path>>(path: P) -> std::io::Result<()> {
    let playlist = M3uPlaylist::new();
    write_m3u(path, &playlist, true, true, true)
}

pub fn get_folders(path: &str) -> std::io::Result<Vec<PathBuf>> {
    let entries = fs::read_dir(path)?; // Read the directory contents
    let folders = entries
        .filter_map(|entry| entry.ok()) // Ignore entries with errors
        .filter(|entry| entry.path().is_dir()) // Keep only directories
        .map(|entry| entry.path()) // Convert DirEntry to PathBuf
        .collect();
    Ok(folders)
}

pub fn reset_playlist_ids(app: &mut TemplateApp) {
    let mut count = 0;
    for mut playlist in app.playlists.clone() {
        let old_path = playlist.clone();
        let selected = &playlist == &app.currently_selected_playlist_path;
        let file_name = path_to_string_name(&playlist);
        let clean_name: String = file_name.chars().skip(4).collect(); // todo: when program more refined, check if you need it like this or if you can just do [4..]
        // ^^ this is done in case a playlist file is ever put into folder that has less than 4 chars. shouldn't happen, but just in case.
        let count62 = to_base62(count, 4); // 14 million playlists gotta be enough.
        playlist.set_file_name(format!("{:04}{}", count62, clean_name));
        app.playlists[count] = playlist.clone(); // this should probably be on a different thread, since a huge amount of playlists will cause a freeze bc disk operations
        if playlist.set_extension("m3utmp") {
            if playlist.file_name()
                != app
                    .currently_selected_playlist_name
                    .as_ref()
                    .map(std::ffi::OsStr::new)
            /*  this check is useless if .file_name returns the extension aswell.
            meant to be a bit of an optimization, so that we do not rename playlists that aren't being rearranged.
            though, i'm not sure if it's working right. i do not think it is, actually! */
            {
                if let Err(error) = fs::rename(&old_path, &playlist) {
                    show_error(
                        app,
                        format!(
                            "err: {} | from: {} | to: {}",
                            error.to_string(),
                            path_to_string(&old_path),
                            path_to_string(&playlist),
                        ),
                    );
                    eprintln!(
                        "reset_playlist_ids: err: {} | from: {} | to: {}",
                        error.to_string(),
                        path_to_string(&old_path),
                        path_to_string(&playlist),
                    );
                }
                if selected {
                    playlist.set_extension("m3u");
                    app.currently_selected_playlist_path = playlist;
                    //show_error(self, "Meow! Selected moved.".to_string());
                }
            }
        } else {
            let err = "reset_playlist_ids set_extension error 1: m3utmp".to_string();
            show_error(app, err.clone());
            eprintln!("{}", err);
        }
        count += 1;
    }
    for mut playlist in app.playlists.clone() {
        let mut old_path = playlist.clone();
        old_path.set_extension("m3utmp");
        if !&playlist.set_extension("m3u") {
            let err = "reset_playlist_ids set_extension error 2: m3u".to_string();
            show_error(app, err.clone());
            eprintln!("{}", err);
        }
        if let Err(error) = fs::rename(&old_path, &playlist) {
            show_error(
                app,
                format!(
                    "err: {} | from: {} | to: {}",
                    error.to_string(),
                    path_to_string(&old_path),
                    path_to_string(&playlist),
                ),
            );
        }
    }
}
