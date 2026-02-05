use egui::{Context, Ui, Color32, Id};

use crate::TemplateApp;
use crate::playlist::{SongCardData, add_to_playlist, remove_from_playlist};
use crate::utilities::{path_to_string, path_to_string_name};
use crate::app::load_metadata_if_needed;
/// UI ///
/// Drawing functions

pub fn right_click_song_card(
    app: &mut TemplateApp,
    ui: &mut egui::Ui,
    song_data: SongCardData,
    index: usize,
) {
    ui.set_max_width(200.0); // To make sure we wrap long text

    ui.menu_button("Add to playlist", |ui| {
        for playlist in &app.playlists {
            let playlist_name = &path_to_string_name(playlist)[4..];
            let playlist_path = path_to_string(&playlist.to_path_buf());
            if ui.button(playlist_name).clicked() {
                let _ = add_to_playlist(&playlist_path, &song_data);
                if Some(playlist_path) == app.currently_selected_playlist_name {
                    app.songs.articles.push(song_data.clone()); // wish this could be in the add_to_playlist function but couldn't get it to play nice. skill issue
                }
                if Some(playlist_name) == app.currently_selected_playlist_name.as_deref() {
                    app.songs.articles.extend([song_data.clone()]);
                }
            }
        }
        let _ = ui.button("todo - New Playlist & Playlist Folders");
    });
    if ui.button("Remove from playlist").clicked() {
        let playlist_path = path_to_string(
            &app.currently_selected_playlist_path
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


pub fn display_song_card(app: &mut TemplateApp, ctx: &Context, ui: &mut Ui, i: usize) -> bool{
    let song = &mut app.songs.articles[i];
    let mut clicked = false;
    if song.display {
        load_metadata_if_needed(song, app.metadata_sender.clone());
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
                                        app.title_header_width + 25.0,
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
                                                /* for some reason the spinner is slightly larger than the image, despite being 30.0?
                                                    it might have some sort of padding, but im not sure how to change that. */
                                        );
                                    }
                                    ui.vertical(|ui| {
                                        // song & artist names
                                        let color = if app.now_playing
                                        /* todo: this should be based off of the ID in the list, and the currently selected playlist.
                                        Will need to also add logic in the song reordering area to change the currently selected ID */
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
                                        //ui.label(path_to_string(&song.path));
                                    },
                                );
                            });
                        });
                },
            )
            .response;
        if response.double_clicked() {
            clicked = true;
        }

        if app.row_height.is_none() {
            app.row_height = Some(response.rect.height()); // todo: this is in the for loop and is probably fuck for performance \(￣︶￣*\))
        } // this really only needs to be done on startup

        if app.now_playing == Some(song.path.clone()) {
            // todo: this check should be based on file *and* playlist!
            ui.painter().rect_filled(
                response.rect,
                4.0,
                egui::Color32::from_white_alpha(10),
            );
        }
        let song_send = song.clone();
        app.apply_options(
            egui::Popup::context_menu(&response)
                .id(Id::new(format!("context_menu{}", i))),
        )
        .show(|ui| right_click_song_card(app, ui, song_send, i));
    }
    clicked
}