//
// player.rs
// Copyright (C) 2019 gmg137 <gmg137@live.com>
// Distributed under terms of the GPLv3 license.
//
use crate::app::Action;
use crate::clone;
use crate::data::MusicData;
use crate::musicapi::model::SongInfo;
use crate::utils::*;
use chrono::NaiveTime;
use crossbeam_channel::Sender;
use dbus::arg::RefArg;
use fragile::Fragile;
use glib::{SignalHandlerId, WeakRef};
use gst::ClockTime;
use gtk::prelude::*;
use gtk::{ActionBar, Builder, Button, Image, Label, RadioButton, Scale};
use mpris_player::{Metadata, MprisPlayer, OrgMprisMediaPlayer2Player, PlaybackStatus};
use std::cell::RefCell;
use std::ops::Deref;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone)]
struct PlayerControls {
    play: Button,
    pause: Button,
    backward: Button,
    forward: Button,
    like: Button,
}

#[derive(Debug, Clone)]
struct PlayerInfo {
    song: Label,
    singer: Label,
    cover: Image,
    mpris: Arc<MprisPlayer>,
    episode_id: RefCell<Option<i32>>,
}

impl PlayerInfo {
    fn init(&self, song_info: &SongInfo) {
        self.song.set_text(&song_info.name);
        self.song.set_tooltip_text(Some(&song_info.name[..]));
        self.singer.set_text(&song_info.singer);
        let image_path = format!("{}/{}.jpg", crate::CACHED_PATH.to_owned(), &song_info.id);
        download_img(&song_info.pic_url, &image_path, 38, 38);
        self.cover.set_from_file(&image_path);
        self.cover.set_tooltip_text(Some(&song_info.name[..]));

        let mut metadata = Metadata::new();
        metadata.artist = Some(vec![song_info.singer.clone()]);
        metadata.title = Some(song_info.name.clone());
        metadata.art_url = Some(song_info.pic_url.clone());
        metadata.track_number = Some(song_info.id as i32);

        self.mpris.set_metadata(metadata);
        self.mpris.set_can_play(true);
    }
}

#[derive(Debug, Clone)]
struct PlayerTimes {
    progressed: Label,
    duration: Label,
    slider: Scale,
    slider_update: Rc<SignalHandlerId>,
}

#[derive(Debug, Clone, Copy)]
struct Duration(ClockTime);

impl Deref for Duration {
    type Target = ClockTime;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

fn format_duration(seconds: u32) -> String {
    let time = NaiveTime::from_num_seconds_from_midnight(seconds, 0);

    if seconds >= 3600 {
        time.format("%T").to_string()
    } else {
        time.format("%M∶%S").to_string()
    }
}

#[derive(Debug, Clone, Copy)]
struct Position(ClockTime);

impl Deref for Position {
    type Target = ClockTime;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl PlayerTimes {
    pub(crate) fn on_duration_changed(&self, duration: Duration) {
        let seconds = duration.seconds().map(|v| v as f64).unwrap_or(0.0);

        self.slider.block_signal(&self.slider_update);
        self.slider.set_range(0.0, seconds);
        self.slider.unblock_signal(&self.slider_update);

        self.duration.set_text(&format_duration(seconds as u32));
    }

    pub(crate) fn on_position_updated(&self, position: Position) {
        let seconds = position.seconds().map(|v| v as f64).unwrap_or(0.0);

        self.slider.block_signal(&self.slider_update);
        self.slider.set_value(seconds);
        self.slider.unblock_signal(&self.slider_update);

        self.progressed.set_text(&format_duration(seconds as u32));
    }
}

#[derive(Debug, Clone)]
struct PlayerLoops {
    shuffle: RadioButton,
    playlist: RadioButton,
    none: RadioButton,
    consecutive: RadioButton,
    image: Image,
}

#[derive(Debug, Clone)]
enum LoopsState {
    SHUFFLE,
    PLAYLIST,
    NONE,
    CONSECUTIVE,
}

#[derive(Clone)]
pub(crate) struct PlayerWidget {
    pub(crate) action_bar: ActionBar,
    player: gst_player::Player,
    controls: PlayerControls,
    timer: PlayerTimes,
    info: PlayerInfo,
    loops: PlayerLoops,
    loops_state: Rc<RefCell<LoopsState>>,
    player_types: Rc<RefCell<PlayerTypes>>,
    data: Arc<Mutex<MusicData>>,
    sender: Sender<Action>,
}

impl PlayerWidget {
    fn new(builder: &Builder, data: Arc<Mutex<MusicData>>, sender: Sender<Action>) -> Self {
        let dispatcher = gst_player::PlayerGMainContextSignalDispatcher::new(None);
        let player = gst_player::Player::new(
            None,
            // Use the gtk main thread
            Some(&dispatcher.upcast::<gst_player::PlayerSignalDispatcher>()),
        );

        let mpris = MprisPlayer::new(
            "NeteaseCloudMusic".to_string(),
            "Netease Cloud Music".to_string(),
            "com.github.gmg137.netease-cloud-music-gtk.desktop".to_string(),
        );
        mpris.set_can_raise(true);
        mpris.set_can_control(true);
        mpris.set_can_play(false);
        mpris.set_can_seek(false);
        mpris.set_can_set_fullscreen(false);

        let mut config = player.get_config();
        config.set_user_agent(
            "User-Agent: Mozilla/5.0 (X11; Linux x86_64; rv:65.0) Gecko/20100101 Firefox/65.0",
        );
        config.set_position_update_interval(250);
        player.set_config(config).unwrap();

        let play: Button = builder.get_object("play_button").unwrap();
        let pause: Button = builder.get_object("pause_button").unwrap();
        let forward: Button = builder.get_object("forward_button").unwrap();
        let backward: Button = builder.get_object("backward_button").unwrap();
        let like: Button = builder.get_object("like_button").unwrap();

        let controls = PlayerControls {
            play,
            pause,
            forward,
            backward,
            like,
        };

        let progressed: Label = builder.get_object("progress_time_label").unwrap();
        let duration: Label = builder.get_object("total_duration_label").unwrap();
        let slider: Scale = builder.get_object("seek").unwrap();
        slider.set_range(0.0, 1.0);
        let player_weak = player.downgrade();
        let slider_update = Rc::new(Self::connect_update_slider(&slider, player_weak));
        let timer = PlayerTimes {
            progressed,
            duration,
            slider,
            slider_update,
        };

        let song: Label = builder.get_object("song_label").unwrap();
        let singer: Label = builder.get_object("singer_label").unwrap();
        let cover: Image = builder.get_object("song_cover").unwrap();
        let info = PlayerInfo {
            mpris,
            song,
            singer,
            cover,
            episode_id: RefCell::new(None),
        };

        let shuffle: RadioButton = builder.get_object("shuffle_radio").unwrap();
        let playlist: RadioButton = builder.get_object("playlist_radio").unwrap();
        let none: RadioButton = builder.get_object("none_radio").unwrap();
        let consecutive: RadioButton = builder.get_object("consecutive_radio").unwrap();
        let image: Image = builder.get_object("loops_image").unwrap();
        let loops = PlayerLoops {
            shuffle,
            playlist,
            none,
            consecutive,
            image,
        };

        let action_bar: ActionBar = builder.get_object("play_action_bar").unwrap();
        PlayerWidget {
            player,
            action_bar,
            controls,
            timer,
            info,
            loops,
            loops_state: Rc::new(RefCell::new(LoopsState::CONSECUTIVE)),
            player_types: Rc::new(RefCell::new(PlayerTypes::Song)),
            data,
            sender,
        }
    }

    fn reveal(&self) {
        self.action_bar.show();
    }

    pub(crate) fn initialize_player(&self, song_info: SongInfo, player_types: PlayerTypes) {
        *self.player_types.borrow_mut() = player_types;
        let sender = self.sender.clone();
        let data = self.data.clone();
        std::thread::spawn(move || {
            let mut data = data.lock().unwrap();
            if let Some(v) = data.songs_url(&[song_info.id], 320) {
                sender
                    .send(Action::Player(song_info, v[0].url.to_owned()))
                    .unwrap();
            } else {
                sender
                    .send(Action::ShowNotice("播放失败!".to_owned()))
                    .unwrap();
            }
        });
    }

    pub(crate) fn player(&self, song_info: SongInfo, song_url: String) {
        match *self.player_types.borrow() {
            PlayerTypes::Fm => {
                self.sender
                    .send(Action::RefreshMineFm(song_info.to_owned()))
                    .unwrap();
            }
            _ => (),
        }
        self.sender
            .send(Action::ShowNotice(song_info.name.to_owned()))
            .unwrap();
        self.info.init(&song_info);
        self.player.set_uri(&song_url);
        self.play();
    }

    fn connect_update_slider(
        slider: &Scale,
        player: WeakRef<gst_player::Player>,
    ) -> SignalHandlerId {
        slider.connect_value_changed(move |slider| {
            let player = match player.upgrade() {
                Some(p) => p,
                None => return,
            };

            let value = slider.get_value() as u64;
            player.seek(ClockTime::from_seconds(value));
        })
    }

    fn play(&self) {
        self.reveal();

        self.controls.pause.show();
        self.controls.play.hide();

        self.player.play();
        self.info.mpris.set_playback_status(PlaybackStatus::Playing);
    }

    fn pause(&self) {
        self.controls.pause.hide();
        self.controls.play.show();

        self.player.pause();
        self.info.mpris.set_playback_status(PlaybackStatus::Paused);
    }

    fn stop(&self) {
        self.controls.pause.hide();
        self.controls.play.show();

        self.player.stop();
        self.info.mpris.set_playback_status(PlaybackStatus::Stopped);
        self.forward();
    }

    pub(crate) fn forward(&self) {
        match *self.player_types.borrow() {
            PlayerTypes::Fm => {
                if let Some(si) = get_player_list_song(PD::FORWARD, false, false) {
                    self.sender
                        .send(Action::Player(si.to_owned(), si.song_url.to_owned()))
                        .unwrap();
                } else {
                    self.sender.send(Action::RefreshMineFmPlayerList).unwrap();
                }
                return;
            }
            _ => (),
        }
        let state = match *self.loops_state.borrow() {
            LoopsState::SHUFFLE => true,
            LoopsState::PLAYLIST => {
                if let Some(si) = get_player_list_song(PD::FORWARD, false, false) {
                    self.sender
                        .send(Action::Player(si.to_owned(), si.song_url.to_owned()))
                        .unwrap();
                } else {
                    if let Some(si) = get_player_list_song(PD::FORWARD, false, true) {
                        self.sender
                            .send(Action::Player(si.to_owned(), si.song_url.to_owned()))
                            .unwrap();
                    }
                }
                return;
            }
            LoopsState::NONE => {
                self.play();
                return;
            }
            LoopsState::CONSECUTIVE => false,
        };
        if let Some(si) = get_player_list_song(PD::FORWARD, state, false) {
            self.sender
                .send(Action::Player(si.to_owned(), si.song_url.to_owned()))
                .unwrap();
        }
    }

    fn backward(&self) {
        let state = match *self.loops_state.borrow() {
            LoopsState::SHUFFLE => true,
            LoopsState::PLAYLIST => {
                if let Some(si) = get_player_list_song(PD::BACKWARD, false, false) {
                    self.sender
                        .send(Action::Player(si.to_owned(), si.song_url.to_owned()))
                        .unwrap();
                } else {
                    if let Some(si) = get_player_list_song(PD::BACKWARD, false, true) {
                        self.sender
                            .send(Action::Player(si.to_owned(), si.song_url.to_owned()))
                            .unwrap();
                    }
                }
                return;
            }
            LoopsState::NONE => {
                self.stop();
                self.play();
                return;
            }
            LoopsState::CONSECUTIVE => false,
        };
        if let Some(si) = get_player_list_song(PD::BACKWARD, state, false) {
            self.sender
                .send(Action::Player(si.to_owned(), si.song_url.to_owned()))
                .unwrap();
        }
    }

    fn like(&self) {
        let sender = self.sender.clone();
        if let Ok(metadata) = self.info.mpris.get_metadata() {
            if let Some(value) = metadata.get("xesam:trackNumber") {
                if let Some(id) = value.as_i64() {
                    let data = self.data.clone();
                    std::thread::spawn(move || {
                        let mut data = data.lock().unwrap();
                        if data.like(true, id as u32) {
                            sender
                                .send(Action::ShowNotice("已添加到喜欢!".to_owned()))
                                .unwrap();
                        } else {
                            sender
                                .send(Action::ShowNotice("收藏失败!".to_owned()))
                                .unwrap();
                        }
                    });
                    return;
                }
            }
        }
        self.sender
            .send(Action::ShowNotice("收藏失败!".to_owned()))
            .unwrap();
    }
}

#[derive(Clone)]
pub(crate) struct PlayerWrapper(pub Rc<PlayerWidget>);

impl Deref for PlayerWrapper {
    type Target = Rc<PlayerWidget>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl PlayerWrapper {
    pub(crate) fn new(
        builder: &Builder,
        sender: &Sender<Action>,
        data: Arc<Mutex<MusicData>>,
    ) -> Self {
        let w = PlayerWrapper(Rc::new(PlayerWidget::new(
            builder,
            data.clone(),
            sender.clone(),
        )));
        w.init(sender);
        w
    }

    fn init(&self, sender: &Sender<Action>) {
        self.connect_control_buttons();
        self.connect_loops_buttons();
        self.connect_mpris_buttons();
        self.connect_gst_signals(sender);
    }

    /// Connect the `PlayerControls` buttons to the `PlayerExt` methods.
    fn connect_control_buttons(&self) {
        let weak = Rc::downgrade(self);

        // Connect the play button to the gst Player.
        self.controls.play.connect_clicked(clone!(weak => move |_| {
             weak.upgrade().map(|p| p.play());
        }));

        // Connect the pause button to the gst Player.
        self.controls
            .pause
            .connect_clicked(clone!(weak => move |_| {
                weak.upgrade().map(|p| p.pause());
            }));

        self.controls
            .forward
            .connect_clicked(clone!(weak => move |_| {
                weak.upgrade().map(|p| p.forward());
            }));

        self.controls
            .backward
            .connect_clicked(clone!(weak => move |_| {
                weak.upgrade().map(|p| p.backward());
            }));

        self.controls.like.connect_clicked(clone!(weak => move |_| {
            weak.upgrade().map(|p| p.like());
        }));
    }

    #[cfg_attr(rustfmt, rustfmt_skip)]
    fn connect_gst_signals(&self, sender: &Sender<Action>) {
        // Log gst warnings.
        self.player.connect_warning(move |_, warn| warn!("gst warning: {}", warn));

        // Log gst errors.
        self.player.connect_error(clone!(sender => move |_, error| {
            sender
                .send(Action::ShowNotice(format!("播放错误: {}", error)))
                .unwrap();
            // 刷新播放列表
            update_player_list(sender.clone());
        }));

        // The following callbacks require `Send` but are handled by the gtk main loop
        let weak = Fragile::new(Rc::downgrade(self));

        // Update the duration label and the slider
        self.player.connect_duration_changed(clone!(weak => move |_, clock| {
            weak.get()
                .upgrade()
                .map(|p| p.timer.on_duration_changed(Duration(clock)));
        }));

        // Update the position label and the slider
        self.player.connect_position_updated(clone!(weak => move |_, clock| {
            weak.get()
                .upgrade()
                .map(|p| p.timer.on_position_updated(Position(clock)));
        }));

        // Reset the slider to 0 and show a play button
        self.player.connect_end_of_stream(clone!(weak => move |_| {
             weak.get()
                 .upgrade()
                 .map(|p| p.stop());
        }));
    }

    fn connect_loops_buttons(&self) {
        let weak = Rc::downgrade(self);

        self.loops.shuffle.connect_toggled(clone!(weak => move |_| {
        weak.upgrade().map(|p| *p.loops_state.borrow_mut() = LoopsState::SHUFFLE);
        weak.upgrade().map(|p| p.loops.image.set_from_icon_name("media-playlist-shuffle-symbolic",gtk::IconSize::Menu));
        }));

        self.loops
            .playlist
            .connect_toggled(clone!(weak => move |_| {
        weak.upgrade().map(|p| *p.loops_state.borrow_mut() = LoopsState::PLAYLIST);
        weak.upgrade().map(|p| p.loops.image.set_from_icon_name("media-playlist-repeat-symbolic",gtk::IconSize::Menu));
            }));

        self.loops.none.connect_toggled(clone!(weak => move |_| {
        weak.upgrade().map(|p| *p.loops_state.borrow_mut() = LoopsState::NONE);
        weak.upgrade().map(|p| p.loops.image.set_from_icon_name("media-playlist-repeat-song-symbolic",gtk::IconSize::Menu));
        }));
        self.loops.consecutive.connect_toggled(clone!(weak => move |_| {
        weak.upgrade().map(|p| *p.loops_state.borrow_mut() = LoopsState::CONSECUTIVE);
        weak.upgrade().map(|p| p.loops.image.set_from_icon_name("media-playlist-consecutive-symbolic",gtk::IconSize::Menu));
        }));
    }

    fn connect_mpris_buttons(&self) {
        let weak = Rc::downgrade(self);

        let mpris = self.info.mpris.clone();
        self.info.mpris.connect_play_pause(clone!(weak => move || {
            let player = match weak.upgrade() {
                Some(s) => s,
                None => return
            };

            if let Ok(status) = mpris.get_playback_status() {
                match status.as_ref() {
                    "Paused" => player.play(),
                    "Stopped" => player.play(),
                    _ => player.pause(),
                };
            }
        }));

        self.info.mpris.connect_play(clone!(weak => move || {
            let player = match weak.upgrade() {
                Some(s) => s,
                None => return
            };

            player.play();
        }));

        self.info.mpris.connect_pause(clone!(weak => move || {
            let player = match weak.upgrade() {
                Some(s) => s,
                None => return
            };

            player.pause();
        }));

        self.info.mpris.connect_next(clone!(weak => move || {
        weak.upgrade().map(|p| p.forward());
        }));

        self.info.mpris.connect_previous(clone!(weak => move || {
        weak.upgrade().map(|p| p.backward());
        }));
    }
}
