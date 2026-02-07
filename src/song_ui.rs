use std::path::PathBuf;

use egui::{Context, Ui, Id};

use crate::TemplateApp;
use crate::playlist::{SongCardData, add_to_playlist, move_m3u_track, remove_from_playlist};
use crate::utilities::{path_to_string, path_to_string_name, show_error};
use crate::app::load_metadata_if_needed;
use crate::playlist::{get_playlists,reset_playlist_ids};

/// UI ///
/// Drawing functions
/// 

pub fn draw_drop_bar(ui: &mut egui::Ui, start: egui::Pos2, end: egui::Pos2) {
    // This should be in a different UI file, since this UI file is meant to be juist for songs.
            let color = ui.visuals().selection.bg_fill;
            let stroke = egui::Stroke::new(2.0, color);
            ui.painter().line_segment([start, end], stroke);
            ui.painter().circle_filled(start, 3.0, stroke.color);
            ui.painter().circle_filled(end, 3.0, stroke.color);
        }

pub fn right_click_song_card(
    app: &mut TemplateApp,
    ui: &mut egui::Ui,
    song_data: SongCardData,
    index: usize,
) {
    ui.set_max_width(200.0);

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
            &app.currently_selected_playlist_path.to_path_buf(),
        );
        let _ = remove_from_playlist(&playlist_path, index);
        if let Some(index) = app.songs.articles.iter().position(|x| x == &song_data) {
            app.songs.articles[index].display = false;
        }
    }
    if ui.button("Move up").clicked(){
        if let Err(e) = move_m3u_track(app, &path_to_string(&app.currently_selected_playlist_path), index, index-1){
            show_error(app, e.to_string());
        }
    }
}

pub fn right_click_playlist(
    app: &mut TemplateApp,
    ui: &mut egui::Ui,
    playlist_index: usize,
) {
    ui.set_max_width(200.0);
    let playlist = &app.playlists[playlist_index];
    if ui.button("Rename playlist").clicked() {
        app.rename_playlist_show = true;
        app.playlist_to_rename = Some(playlist.to_path_buf());
        let name = &path_to_string_name(playlist)[4..];
        app.rename_to = name[..name.len()-4].to_string();
    }
    if ui.button("Delete playlist").clicked(){
        app.warning_show = true;
        app.playlist_to_delete = Some(playlist.to_path_buf());
    }
}

pub fn delete_playlist_warning(app: &mut TemplateApp, ui: &mut egui::Ui){
    egui::Modal::new(Id::new("Deletion warning")).show(ui.ctx(), |ui| {
        ui.set_width(200.0);
        ui.heading("Delete playlist?");

        ui.add_space(32.0);

        egui::Sides::new().show(
            ui,
            |_ui| {},
            |ui| {
                if ui.button("Yes").clicked() {
                    app.warning_show = false;
                    if app.playlist_to_delete.is_some(){
                        if let Err(e) = std::fs::remove_file(app.playlist_to_delete.as_ref().unwrap()){
                            show_error(app, format!("Failed to delete file: {}",e.to_string()));
                        }
                        app.playlists = get_playlists("./playlists/").unwrap_or_default(); // get because one is now deleted & reset_playlist_ids wont like that
                        reset_playlist_ids(app);
                        app.playlists = get_playlists("./playlists/").unwrap_or_default(); // get again because id's are now changed
                        /* doing this twice? bleh.
                        should just be removing the single removed playlist from ram or skipping it in get_playlists*/
                    }
                }

                if ui.button("Cancel").clicked() {
                    app.warning_show = false;
                    app.playlist_to_delete = None;
                }
            },
        );
    });
}

pub fn rename_playlist(app: &mut TemplateApp, ui: &mut egui::Ui){
    egui::Modal::new(Id::new("Playlist options")).show(ui.ctx(), |ui| {
        ui.set_width(200.0);
        ui.heading("Rename playlist");
        let mut text = app.rename_to.clone();
        ui.text_edit_singleline(&mut app.rename_to);

        ui.add_space(32.0);

        egui::Sides::new().show(
            ui,
            |_ui| {},
            |ui| {
                if ui.button("Save").clicked() {
                    let idx = &path_to_string_name(&app.playlist_to_rename.as_ref().unwrap())[..4];
                    let mut set_current = false;
                    if &app.currently_selected_playlist_path == app.playlist_to_rename.as_ref().unwrap_or(&app.currently_selected_playlist_path){
                        set_current = true;
                    }
                    text = format!("{}{}.m3u",idx,text);
                    if let Some(old_path) = &app.playlist_to_rename{
                        if let Some(parent) = old_path.parent(){
                            let new_path = parent.join(&text);
                            if &new_path != old_path{
                                if new_path.try_exists().unwrap_or(false){
                                    show_error(app,format!("Playlist already exists! If you're seeing this, something went very wrong (✿uwu)\nold path: {} \nnew path: {}",path_to_string(old_path),path_to_string(&new_path)));
                                }else{
                                    if let Err(error) = std::fs::rename(app.playlist_to_rename.as_ref().unwrap(), &new_path) {
                                        show_error(
                                            app,
                                            format!(
                                                "rename playlist err: {} | from: {} | to: {}",
                                                error.to_string(),
                                                path_to_string(&app.playlist_to_rename.as_ref().unwrap()),
                                                text,
                                            ),
                                        );
                                    }
                                    app.playlists = get_playlists("./playlists/").unwrap_or_default();
                                    if set_current{
                                        app.currently_selected_playlist_path = new_path;
                                        app.currently_selected_playlist_name = Some(text[4..].to_string());
                                    }
                                }
                            }
                        }
                    }
                    app.rename_playlist_show = false;
                    app.playlist_to_rename = None;
                }
            },
        );
    });
}

pub fn draw_song_card(app: &mut TemplateApp, ctx: &Context, ui: &mut Ui, i: usize) -> (bool, Option<usize>){
    let song = &mut app.songs.articles[i];
    let mut clicked = false;
    let mut move_to = None;
    // let title = song.title.clone();
    if !song.display { return (false, None)}

    load_metadata_if_needed(song, app.metadata_sender.clone());
    song.load_texture_if_needed(ctx);

    ui.spacing_mut().item_spacing.y = 0.0;

    let response = ui
        .scope_builder(
            egui::UiBuilder::new()
                .id_salt(i)
                .sense(egui::Sense::click()),
            |ui| {
                let response = ui.response();

            let is_upper_half = ui.input(|i| i.pointer.hover_pos()).map_or(true, |pos| pos.y < response.rect.center().y);

            if ui.input(|i| i.pointer.primary_released()) {
                app.drag_origin = None;
                app.dragging_song = None;
            }
            if response.is_pointer_button_down_on() && ui.input(|i| i.pointer.primary_down()){
                app.dragging_song = Some(i);
                app.drag_origin = ui.input(|i| i.pointer.press_origin());
            }
            if app.dragging_song == Some(i) {
                /* this needs the check to see if we are dragging the right song because without
                it every card gets set to being dragged because dragging_song is global.
                hovered() is broken here, because song cards are in a scroll area & that steals focus.
                so, the song card to drag should be set upon response.is_pointer_button_down_on()
                that way, only the clicked song cards index gets set to i.
                so much for just a drag buffer! */
                let delta = ui.input(|i| i.pointer.latest_pos()).unwrap_or(egui::Pos2::new(0.0,0.0)).distance(app.drag_origin.unwrap_or(egui::Pos2::new(0.0,0.0)));
                app.test_thing = Some(delta);
                if delta > 2.0 {
                    app.dragged_song_index = Some(i);
                }
            }
            
            if let Some(from_idx) = app.dragged_song_index {
                if response.contains_pointer() && from_idx != i {
                    let rect = response.rect;
                    let mut start = rect.left_bottom();
                    let mut end = rect.right_bottom();
                    if is_upper_half {
                        move_to = Some(i);
                        start = rect.left_top();
                        end = rect.right_top();
                    } else{
                        move_to = Some(i+1);
                    }
                    draw_drop_bar(ui, start, end);
                }
            }

            let visuals = ui.style().interact(&response);
            let fill_color =
                if response.contains_pointer() || response.has_focus() {
                    visuals.bg_fill.gamma_multiply(0.3)
                } else {
                    egui::Color32::TRANSPARENT
                };
            egui::Frame::new()
                .fill(fill_color)
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
                                    ui.visuals().selection.stroke.color // make this configurable later
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

    (clicked, move_to)
}