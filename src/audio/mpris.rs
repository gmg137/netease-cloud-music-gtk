//
// mpris.rs
// Copyright (C) 2022 gmg137 <gmg137 AT live.com>
// Distributed under terms of the GPL-3.0-or-later license.
//

use adw::prelude::GtkWindowExt;
use glib::clone;
use gtk::glib;
use mpris_server::{zbus::Result, Time, TrackId, *};

use ncm_api::SongInfo;
use std::rc::Rc;

use crate::gui::PlayerControls;
use crate::window::NeteaseCloudMusicGtk4Window;

use super::LoopsState;

unsafe impl Send for MprisController {}
unsafe impl Sync for MprisController {}

#[derive(Debug, Clone)]
pub struct MprisController {
    mpris_player: Rc<Player>,
}

impl MprisController {
    pub async fn new() -> Result<Self> {
        let mpris_player = Rc::new(
            Player::builder(crate::MPRIS_NAME)
                .identity(gettextrs::gettext(crate::APP_NAME))
                .desktop_entry(crate::APP_ID)
                .can_raise(true)
                .can_control(true)
                .can_quit(true)
                .can_play(true)
                .can_pause(true)
                .can_go_next(true)
                .can_go_previous(true)
                .can_quit(true)
                .can_seek(false)
                .build()
                .await?,
        );
        let mpris_player_clone = mpris_player.clone();
        crate::MAINCONTEXT.spawn_local_with_priority(glib::source::Priority::LOW, async move {
            mpris_player_clone.run().await
        });
        let mut metadata = Metadata::new();
        metadata.set_trackid(Some(TrackId::NO_TRACK));
        mpris_player.set_metadata(metadata).await.ok();

        Ok(Self { mpris_player })
    }

    pub async fn update_metadata(&self, si: &SongInfo) -> Result<()> {
        let mut metadata = Metadata::new();
        metadata.set_artist(Some(vec![si.singer.clone()]));
        metadata.set_title(Some(si.name.clone()));
        metadata.set_album(Some(si.album.clone()));
        metadata.set_length(Some(Time::from_micros(si.duration as i64 * 1000)));
        metadata.set_trackid(
            TrackId::try_from(format!("/com/gitee/gmg137/NeteaseCloudMusicGtk4/{}", si.id)).ok(),
        );
        // 取消从缓存获取专辑封面
        //let mut path_cover = crate::path::CACHE.clone();
        //path_cover.push(format!("{}-songlist.jpg", si.album_id));
        //if path_cover.exists() {
        //metadata.set_art_url(Some(format!("file://{}", path_cover.to_string_lossy())));
        //} else {
        //metadata.set_art_url(Some(si.pic_url.to_owned()));
        //}
        metadata.set_art_url(Some(si.pic_url.to_owned()));
        self.mpris_player.set_metadata(metadata).await?;
        Ok(())
    }

    pub async fn set_volume(&self, volume: f64) -> Result<()> {
        if (self.mpris_player.volume() * 100.0).round() as i64 != (volume * 100.0).round() as i64 {
            self.mpris_player.set_volume(volume).await?;
        }
        Ok(())
    }

    pub async fn set_playback_status(&self, state: PlaybackStatus) -> Result<()> {
        if self.mpris_player.playback_status() != state {
            self.mpris_player.set_playback_status(state).await?;
        }
        Ok(())
    }

    pub fn get_loop_status(&self) -> Result<LoopsState> {
        Ok(match self.mpris_player.loop_status().as_str() {
            "None" => LoopsState::None,
            "Track" => LoopsState::Track,
            "Playlist" => LoopsState::Playlist,
            _ => LoopsState::None,
        })
    }

    pub async fn set_loop_status(&self, status: LoopsState) -> Result<()> {
        async fn set_mpris_shuffle(s: &MprisController, val: bool) -> Result<()> {
            if s.mpris_player.shuffle() != val {
                s.mpris_player.set_shuffle(val).await?;
            }
            Ok(())
        }
        async fn set_loop_status(s: &MprisController, val: LoopStatus) -> Result<()> {
            if s.mpris_player.loop_status() != val {
                s.mpris_player.set_loop_status(val).await?;
            }
            Ok(())
        }
        match status {
            LoopsState::Shuffle => set_loop_status(self, LoopStatus::Playlist).await?,
            LoopsState::Playlist => set_loop_status(self, LoopStatus::Playlist).await?,
            LoopsState::Track => set_loop_status(self, LoopStatus::Track).await?,
            LoopsState::None => set_loop_status(self, LoopStatus::None).await?,
        };
        match status {
            LoopsState::Shuffle => set_mpris_shuffle(self, true).await?,
            _ => set_mpris_shuffle(self, false).await?,
        }
        Ok(())
    }

    pub fn set_position(&self, value: i64) {
        self.mpris_player.set_position(Time::from_micros(value));
    }

    pub async fn seeked(&self, value: i64) -> Result<()> {
        self.mpris_player.seeked(Time::from_micros(value)).await
    }

    pub fn setup_signals(&self, player_controls: &PlayerControls) {
        // mpris raise
        self.mpris_player.connect_raise(move |_| {
            let window = NeteaseCloudMusicGtk4Window::default();
            window.present();
        });

        // mpris quit
        self.mpris_player.connect_quit(move |_| {
            let window = NeteaseCloudMusicGtk4Window::default();
            window.destroy();
        });

        // mpris play / pause
        self.mpris_player.connect_play_pause(clone!(
            #[weak(rename_to = mpris)]
            self.mpris_player,
            #[weak]
            player_controls,
            move |_| {
                match mpris.playback_status() {
                    PlaybackStatus::Paused => player_controls.switch_play(),
                    PlaybackStatus::Stopped => player_controls.switch_play(),
                    _ => player_controls.switch_pause(),
                };
            }
        ));

        // mpris play
        self.mpris_player.connect_play(clone!(
            #[weak]
            player_controls,
            move |_| {
                player_controls.switch_play();
            }
        ));

        // mpris pause
        self.mpris_player.connect_pause(clone!(
            #[weak]
            player_controls,
            move |_| {
                player_controls.switch_pause();
            }
        ));

        // mpris stop
        self.mpris_player.connect_stop(clone!(
            #[weak]
            player_controls,
            move |_| {
                player_controls.switch_stop();
            }
        ));

        // mpris next
        self.mpris_player.connect_next(clone!(
            #[weak]
            player_controls,
            move |_| {
                player_controls.next_song();
            }
        ));

        // mpris prev
        self.mpris_player.connect_previous(clone!(
            #[weak]
            player_controls,
            move |_| {
                player_controls.prev_song();
            }
        ));

        // mpris loop
        self.mpris_player.connect_set_loop_status(clone!(
            #[weak]
            player_controls,
            move |_, status| {
                player_controls.set_loops(LoopsState::from(status));
            }
        ));

        // mpris shuffle
        self.mpris_player.connect_set_shuffle(clone!(
            #[weak]
            player_controls,
            move |_, status| {
                player_controls.set_shuffle(status);
            }
        ));

        // mpris volume
        self.mpris_player.connect_set_volume(clone!(
            #[weak]
            player_controls,
            move |_, value| {
                player_controls.set_volume(value);
            }
        ));
    }
}
