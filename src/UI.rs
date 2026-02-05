
use crate::utilities::{path_to_string,path_to_string_name};
use crate::playlist::{add_to_playlist,remove_from_playlist,SongCardData};
use crate::TemplateApp;

        /// UI ///
        /// Drawing functions

pub fn right_click_song_card(app: &mut TemplateApp, ui: &mut egui::Ui, song_data: SongCardData, index: usize) {
        ui.set_max_width(200.0); // To make sure we wrap long text

        ui.menu_button("Add to playlist", |ui| {
            for playlist in &app.playlists {
                let playlist_name = &path_to_string_name(playlist)[4..];
                let playlist_path = path_to_string(&playlist.to_path_buf());
                if ui.button(playlist_name).clicked() {
                    let _ = add_to_playlist(&playlist_path, &song_data);
                    if (Some(playlist_path) == app.currently_selected_playlist) {
                        app.songs.articles.push(song_data.clone()); // wish this could be in the add_to_playlist function but couldn't get it to play nice. skill issue
                    }
                    if Some(playlist_name) == app.currently_selected_playlist.as_deref() {
                        app.songs.articles.extend([song_data.clone()]);
                    }
                }
            }
            let _ = ui.button("todo - New Playlist & Playlist Folders");
        });
        if ui.button("Remove from playlist").clicked() {
            let playlist_path = path_to_string(
                &app
                    .currently_selected_playlist_path
                    .as_ref()
                    .unwrap()
                    .to_path_buf(),
            );
            let _ = remove_from_playlist(&playlist_path, index);
            if let Some(index) = app.songs.articles.iter().position(|x| x == &song_data) {
                app.songs.articles[index].display = false;
            }
        }
    }