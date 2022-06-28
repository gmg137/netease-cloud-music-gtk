//
// mpris.rs
// Copyright (C) 2022 gmg137 <gmg137 AT live.com>
// Distributed under terms of the GPL-3.0-or-later license.
//

use adw::prelude::GtkWindowExt;
use glib::clone;
use gtk::glib;
use mpris_player::*;
use ncm_api::SongInfo;
use std::sync::Arc;

use crate::gui::PlayerControls;
use crate::path::CACHE;
use crate::window::NeteaseCloudMusicGtk4Window;

use super::LoopsState;

#[derive(Debug)]
pub struct MprisController {
    mpris: Arc<MprisPlayer>,
}

impl MprisController {
    pub fn new() -> Self {
        let mpris = MprisPlayer::new(
            crate::APP_ID.to_string(),
            gettextrs::gettext("Netease Cloud Music"),
            crate::APP_ID.to_string(),
        );
        mpris.set_can_raise(true);
        mpris.set_can_control(true);
        mpris.set_can_quit(true);
        mpris.set_can_seek(false);

        Self { mpris }
    }

    pub fn update_metadata(&self, si: &SongInfo) {
        let mut metadata = Metadata::new();
        metadata.artist = Some(vec![si.singer.clone()]);
        metadata.title = Some(si.name.clone());
        let mut path_cover = CACHE.clone();
        path_cover.push(format!("{}-cover.jpg", si.album_id));
        if path_cover.exists() {
            metadata.art_url = Some(format!("file://{}", path_cover.to_string_lossy()));
        } else {
            metadata.art_url = Some(si.pic_url.to_owned());
        }
        self.mpris.set_metadata(metadata);
    }

    pub fn set_volume(&self, volume: f64) {
        self.mpris.set_volume(volume).unwrap();
    }

    pub fn set_playback_status(&self, state: PlaybackStatus) {
        self.mpris.set_playback_status(state);
    }

    pub fn get_loop_status(&self) -> Option<String> {
        self.mpris.get_loop_status().ok()
    }

    pub fn set_loop_status(&self, status: LoopsState) {
        match status {
            LoopsState::SHUFFLE => self.mpris.set_shuffle(true).unwrap(),
            LoopsState::LOOP => self.mpris.set_loop_status(LoopStatus::Playlist),
            LoopsState::ONE => self.mpris.set_loop_status(LoopStatus::Track),
            LoopsState::NONE => self.mpris.set_loop_status(LoopStatus::None),
        };
    }

    pub fn set_position(&self, value: i64) {
        self.mpris.set_position(value);
    }

    pub fn seek(&self, value: i64) {
        self.mpris.seek(value).ok();
    }

    pub fn setup_signals(&self, player_controls: &PlayerControls) {
        // mpris raise
        self.mpris.connect_raise(move || {
            let window = NeteaseCloudMusicGtk4Window::default();
            window.present();
        });

        // mpris quit
        self.mpris.connect_quit(move || {
            let window = NeteaseCloudMusicGtk4Window::default();
            window.destroy();
        });

        // mpris play / pause
        self.mpris.connect_play_pause(
            clone!(@weak self.mpris as mpris, @weak player_controls => move || {
                match mpris.get_playback_status().unwrap().as_ref() {
                    "Paused" => player_controls.switch_play(),
                    "Stopped" => player_controls.switch_play(),
                    _ => player_controls.switch_pause(),
                };
            }),
        );

        // mpris play
        self.mpris
            .connect_play(clone!(@weak player_controls => move || {
                    player_controls.switch_play();
            }));

        // mpris pause
        self.mpris
            .connect_pause(clone!(@weak player_controls => move || {
                    player_controls.switch_pause();
            }));

        // mpris stop
        self.mpris
            .connect_stop(clone!(@weak player_controls => move || {
                    player_controls.switch_stop();
            }));

        // mpris next
        self.mpris
            .connect_next(clone!(@weak player_controls => move || {
                    player_controls.next_song();
            }));

        // mpris prev
        self.mpris
            .connect_previous(clone!(@weak player_controls => move || {
                    player_controls.prev_song();
            }));

        // mpris loop
        self.mpris
            .connect_loop_status(clone!(@weak player_controls => move |status| {
                    player_controls.set_loops(status);
            }));

        // mpris shuffle
        self.mpris
            .connect_shuffle(clone!(@weak player_controls => move |status| {
                    player_controls.set_shuffle(status);
            }));

        // mpris volume
        self.mpris
            .connect_volume(clone!(@weak player_controls => move |value| {
                    player_controls.set_volume(value);
            }));
    }
}

impl Default for MprisController {
    fn default() -> Self {
        Self::new()
    }
}
