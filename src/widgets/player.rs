//
// player.rs
// Copyright (C) 2019 gmg137 <gmg137@live.com>
// Distributed under terms of the GPLv3 license.
//
use crate::{app::Action, data::MusicData, model::NCM_CACHE, musicapi::model::SongInfo, task::Task, utils::*};
use async_std::task;
use chrono::NaiveTime;
use crossbeam_channel::Sender;
use fragile::Fragile;
use futures::{channel::mpsc, sink::SinkExt};
use gdk_pixbuf::{InterpType, Pixbuf};
use glib::{clone, SignalHandlerId, WeakRef};
use gst::ClockTime;
use gtk::{
    prelude::*, ActionBar, Builder, Button, Image, Label, MenuButton, RadioButton, Scale, TextView, VolumeButton,
};
use mpris_player::{Metadata, MprisPlayer, OrgMprisMediaPlayer2Player, PlaybackStatus};
use serde::{Deserialize, Serialize};
use std::{cell::RefCell, ops::Deref, path::Path, rc::Rc, sync::Arc};

#[derive(Debug, Clone)]
struct PlayerControls {
    play: Button,
    pause: Button,
    backward: Button,
    forward: Button,
    like: Button,
    volume: VolumeButton,
    lyrics: MenuButton,
    lyrics_text: TextView,
}

#[derive(Debug, Clone)]
struct PlayerInfo {
    song: Label,
    singer: Label,
    cover: Image,
    mpris: Arc<MprisPlayer>,
    song_id: RefCell<Option<u32>>,
}

impl PlayerInfo {
    fn init(&self, song_info: &SongInfo) {
        self.song.set_text(&song_info.name);
        self.song.set_tooltip_text(Some(&song_info.name[..]));
        self.singer.set_text(&song_info.singer);
        let image_path = format!("{}{}.jpg", NCM_CACHE.to_string_lossy(), &song_info.id);
        if let Ok(image) = Pixbuf::new_from_file(&image_path) {
            let image = image.scale_simple(38, 38, InterpType::Bilinear);
            self.cover.set_from_pixbuf(image.as_ref());
        };
        self.cover.set_tooltip_text(Some(&song_info.name[..]));
        *self.song_id.borrow_mut() = Some(song_info.id);

        let mut metadata = Metadata::new();
        metadata.artist = Some(vec![song_info.singer.clone()]);
        metadata.title = Some(song_info.name.clone());
        metadata.art_url = Some(format!("file://{}{}.jpg", NCM_CACHE.to_string_lossy(), song_info.id));

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

// 播放列表循环模式
#[derive(Debug, Clone, Deserialize, Serialize)]
pub(crate) enum LoopsState {
    // 随机
    SHUFFLE,
    // 列表循环
    PLAYLIST,
    // 单曲循环
    NONE,
    // 不循环
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
    sender: Sender<Action>,
    sender_task: mpsc::Sender<Task>,
}

impl PlayerWidget {
    fn new(builder: &Builder, sender: Sender<Action>, sender_task: mpsc::Sender<Task>) -> Self {
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
        config.set_user_agent("User-Agent: Mozilla/5.0 (X11; Linux x86_64; rv:65.0) Gecko/20100101 Firefox/65.0");
        config.set_position_update_interval(250);
        player.set_config(config).unwrap();

        let play: Button = builder.get_object("play_button").unwrap();
        let pause: Button = builder.get_object("pause_button").unwrap();
        let forward: Button = builder.get_object("forward_button").unwrap();
        let backward: Button = builder.get_object("backward_button").unwrap();
        let like: Button = builder.get_object("like_button").unwrap();
        let volume: VolumeButton = builder.get_object("volume_button").unwrap();
        let volume_value = mpris.get_volume().unwrap_or(0.0);
        volume.set_value(volume_value);
        player.set_volume(volume_value);
        let lyrics: MenuButton = builder.get_object("lyrics_button").unwrap();
        let lyrics_text: TextView = builder.get_object("lyrics_text_view").unwrap();

        let controls = PlayerControls {
            play,
            pause,
            forward,
            backward,
            like,
            volume,
            lyrics,
            lyrics_text,
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
            song_id: RefCell::new(None),
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
        let loop_state = task::block_on(async {
            if let Ok(conf) = get_config().await {
                conf.loops
            } else {
                LoopsState::CONSECUTIVE
            }
        });
        match loop_state {
            LoopsState::CONSECUTIVE => {
                loops
                    .image
                    .set_from_icon_name(Some("media-playlist-consecutive-symbolic"), gtk::IconSize::Menu);
                loops.consecutive.set_active(true);
            }
            LoopsState::NONE => {
                loops
                    .image
                    .set_from_icon_name(Some("media-playlist-repeat-song-symbolic"), gtk::IconSize::Menu);
                loops.none.set_active(true);
            }
            LoopsState::PLAYLIST => {
                loops
                    .image
                    .set_from_icon_name(Some("media-playlist-repeat-symbolic"), gtk::IconSize::Menu);
                loops.playlist.set_active(true);
            }
            LoopsState::SHUFFLE => {
                loops
                    .image
                    .set_from_icon_name(Some("media-playlist-shuffle-symbolic"), gtk::IconSize::Menu);
                loops.shuffle.set_active(true);
            }
        }
        PlayerWidget {
            player,
            action_bar,
            controls,
            timer,
            info,
            loops,
            loops_state: Rc::new(RefCell::new(loop_state)),
            player_types: Rc::new(RefCell::new(PlayerTypes::Song)),
            sender,
            sender_task,
        }
    }

    fn reveal(&self) {
        self.action_bar.show();
    }

    pub(crate) fn initialize_player(&self, song_info: SongInfo, player_types: PlayerTypes, lyrics: bool) {
        match player_types {
            PlayerTypes::Fm => {
                if song_info.id == self.info.song_id.borrow().unwrap_or(0) {
                    self.play();
                    return;
                }
            }
            _ => (),
        }
        *self.player_types.borrow_mut() = player_types;
        let sender = self.sender.clone();
        let mut sender_task = self.sender_task.clone();
        task::spawn(async move {
            if let Ok(mut data) = MusicData::new().await {
                // 下载歌词
                if lyrics {
                    download_lyrics(&mut data, &song_info.name, &song_info).await.ok();
                }
                let path = format!("{}{}.mp3", NCM_CACHE.to_string_lossy(), song_info.id);
                if std::path::Path::new(&path).exists() {
                    sender.send(Action::Player(song_info)).unwrap();
                } else {
                    if let Ok(v) = data.songs_url(&[song_info.id], 320).await {
                        if v.len() > 0 {
                            let mut song_info = song_info;
                            song_info.song_url = v[0].url.to_string();
                            sender.send(Action::Player(song_info.clone())).unwrap();
                            // 缓存音乐和图片
                            let image_path = format!("{}{}.jpg", NCM_CACHE.to_string_lossy(), &song_info.id);
                            sender_task
                                .send(Task::DownloadPlayerImg {
                                    url: song_info.pic_url.to_owned(),
                                    path: image_path.to_owned(),
                                    width: 34,
                                    high: 34,
                                    timeout: 1000,
                                })
                                .await
                                .ok();
                            sender_task
                                .send(Task::DownloadMusic {
                                    url: song_info.song_url.to_owned(),
                                    path: path.to_owned(),
                                    timeout: 3000,
                                })
                                .await
                                .ok();
                        } else {
                            warn!(
                                "未能获取 {}[id:{}] 的播放链接!(版权或VIP限制)",
                                song_info.name, song_info.id
                            );
                            sender.send(Action::ShowNotice("播放失败!".to_owned())).unwrap();
                        }
                    } else {
                        warn!("解析 {}[id:{}] 的播放链接失败!", song_info.name, song_info.id);
                        sender.send(Action::ShowNotice("播放失败!".to_owned())).unwrap();
                    }
                }
            } else {
                sender.send(Action::ShowNotice("接口请求异常!".to_owned())).unwrap();
            }
        });
    }

    pub(crate) fn ready_player(&self, song_info: SongInfo, lyrics: bool) {
        let sender = self.sender.clone();
        let mut sender_task = self.sender_task.clone();
        task::spawn(async move {
            // 下载歌词
            if lyrics {
                if let Ok(mut data) = MusicData::new().await {
                    download_lyrics(&mut data, &song_info.name, &song_info).await.ok();
                }
            }
            sender.send(Action::Player(song_info.clone())).unwrap();
            // 缓存音乐图片路径
            let image_path = format!("{}{}.jpg", NCM_CACHE.to_string_lossy(), song_info.id);
            // 缓存音乐路径
            let path = format!("{}{}.mp3", NCM_CACHE.to_string_lossy(), song_info.id);
            sender_task
                .send(Task::DownloadPlayerImg {
                    url: song_info.pic_url.to_owned(),
                    path: image_path.to_owned(),
                    width: 34,
                    high: 34,
                    timeout: 1000,
                })
                .await
                .ok();
            sender_task
                .send(Task::DownloadMusic {
                    url: song_info.song_url.to_owned(),
                    path: path.to_owned(),
                    timeout: 3000,
                })
                .await
                .ok();
        });
    }

    pub(crate) fn player(&self, song_info: SongInfo) {
        info!("准备播放音乐: {:?}", song_info);
        match *self.player_types.borrow() {
            PlayerTypes::Fm => {
                self.sender.send(Action::RefreshMineFm(song_info.to_owned())).unwrap();
            }
            _ => (),
        }
        self.sender.send(Action::ShowNotice(song_info.name.to_owned())).unwrap();
        self.info.init(&song_info);
        let song_uri = format!("{}{}.mp3", NCM_CACHE.to_string_lossy(), song_info.id);
        if Path::new(&song_uri).exists() {
            info!("播放音乐缓存: {}", song_uri);
            self.player.set_uri(&format!("file://{}", song_uri));
        } else {
            let music_url = song_info.song_url.replace("https:", "http:");
            info!("播放在线音乐: {}", music_url);
            self.player.set_uri(&music_url);
        }
        self.play();
    }

    fn connect_update_slider(slider: &Scale, player: WeakRef<gst_player::Player>) -> SignalHandlerId {
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
        // 更新 FM 播放按钮
        match *self.player_types.borrow() {
            PlayerTypes::Fm => {
                self.sender.send(Action::RefreshMineFmPause).unwrap();
            }
            _ => self.sender.send(Action::RefreshMineFmPlay).unwrap(),
        }
        self.reveal();

        self.controls.pause.show();
        self.controls.play.hide();

        self.player.play();
        self.info.mpris.set_playback_status(PlaybackStatus::Playing);
    }

    pub(crate) fn pause(&self) {
        // 更新 FM 播放按钮
        match *self.player_types.borrow() {
            PlayerTypes::Fm => {
                self.sender.send(Action::RefreshMineFmPlay).unwrap();
            }
            _ => (),
        }
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
                if let Ok(si) = task::block_on(get_player_list_song(PD::FORWARD, false, false)) {
                    self.sender.send(Action::ReadyPlayer(si.to_owned())).unwrap();
                } else {
                    self.sender.send(Action::RefreshMineFmPlayerList).unwrap();
                }
                return;
            }
            _ => (),
        }
        let (shuffle, loops) = match *self.loops_state.borrow() {
            LoopsState::SHUFFLE => (true, false),
            LoopsState::PLAYLIST => (false, true),
            LoopsState::CONSECUTIVE => (false, false),
            LoopsState::NONE => {
                self.play();
                return;
            }
        };
        if let Ok(si) = task::block_on(get_player_list_song(PD::FORWARD, shuffle, loops)) {
            self.sender.send(Action::ReadyPlayer(si.to_owned())).unwrap();
        }
    }

    fn backward(&self) {
        let state = match *self.loops_state.borrow() {
            LoopsState::SHUFFLE => true,
            LoopsState::PLAYLIST => {
                if let Ok(si) = task::block_on(get_player_list_song(PD::BACKWARD, false, false)) {
                    self.sender.send(Action::ReadyPlayer(si.to_owned())).unwrap();
                } else {
                    if let Ok(si) = task::block_on(get_player_list_song(PD::BACKWARD, false, true)) {
                        self.sender.send(Action::ReadyPlayer(si.to_owned())).unwrap();
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
        if let Ok(si) = task::block_on(get_player_list_song(PD::BACKWARD, state, false)) {
            self.sender.send(Action::ReadyPlayer(si.to_owned())).unwrap();
        }
    }

    fn like(&self) {
        let sender = self.sender.clone();
        let song_id = *self.info.song_id.borrow();
        if let Some(id) = song_id {
            task::spawn(async move {
                if let Ok(mut data) = MusicData::new().await {
                    if data.like(true, id).await {
                        sender.send(Action::ShowNotice("已添加到喜欢!".to_owned())).unwrap();
                        sender.send(Action::RefreshMineLikeList()).unwrap();
                    } else {
                        sender.send(Action::ShowNotice("收藏失败!".to_owned())).unwrap();
                    }
                } else {
                    sender.send(Action::ShowNotice("接口请求异常!".to_owned())).unwrap();
                }
            });
            return;
        }
        self.sender.send(Action::ShowNotice("收藏失败!".to_owned())).unwrap();
    }

    fn set_volume(&self, value: f64) {
        self.info.mpris.set_volume(value).ok();
        self.player.set_volume(value);
    }

    fn get_lyrics_text(&self) {
        let sender = self.sender.clone();
        let song_id = *self.info.song_id.borrow();
        task::spawn(async move {
            if let Ok(mut data) = MusicData::new().await {
                if let Some(id) = song_id {
                    let lrc = get_lyrics(&mut data, id).await.unwrap_or("没有找到歌词!".to_owned());
                    sender.send(Action::RefreshLyricsText(lrc)).unwrap();
                }
            } else {
                sender.send(Action::ShowNotice("接口请求异常!".to_owned())).unwrap();
            }
        });
    }

    pub(crate) fn update_lyrics_text(&self, lrc: String) {
        if let Some(buffer) = self.controls.lyrics_text.get_buffer() {
            buffer.set_text(&lrc);
            self.controls.lyrics_text.set_buffer(Some(&buffer));
        }
    }

    pub(crate) fn set_player_typers(&self, player_types: PlayerTypes) {
        *self.player_types.borrow_mut() = player_types;
    }

    pub(crate) fn set_cover_image(&self, image_path: String) {
        if let Ok(image) = Pixbuf::new_from_file(&image_path) {
            let image = image.scale_simple(38, 38, InterpType::Bilinear);
            self.info.cover.set_from_pixbuf(image.as_ref());
        };
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
    pub(crate) fn new(builder: &Builder, sender: &Sender<Action>, sender_task: &mpsc::Sender<Task>) -> Self {
        let w = PlayerWrapper(Rc::new(PlayerWidget::new(builder, sender.clone(), sender_task.clone())));
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
        let weak = Rc::clone(self);

        // Connect the play button to the gst Player.
        self.controls.play.connect_clicked(clone!(@weak weak => move |_| {
            weak.play();
        }));

        // Connect the pause button to the gst Player.
        self.controls.pause.connect_clicked(clone!(@weak weak => move |_| {
            weak.pause();
        }));

        self.controls.forward.connect_clicked(clone!(@weak weak => move |_| {
            weak.forward();
        }));

        self.controls.backward.connect_clicked(clone!(@weak weak => move |_| {
            weak.backward();
        }));

        self.controls.like.connect_clicked(clone!(@weak weak => move |_| {
            weak.like();
        }));

        self.controls
            .volume
            .connect_value_changed(clone!(@weak weak => move |_, value| {
                weak.set_volume(value);
            }));

        self.controls.lyrics.connect_clicked(clone!(@weak weak => move |_| {
            weak.get_lyrics_text();
        }));
    }

    #[cfg_attr(rustfmt, rustfmt_skip)]
    fn connect_gst_signals(&self, sender: &Sender<Action>) {
        // Log gst warnings.
        self.player.connect_warning(move |_, warn| warn!("gst warning: {}", warn));

        let sender_clone = sender.clone();
        // Log gst errors.
        self.player.connect_error(move |_, _| {
            sender_clone
                .send(Action::ShowNotice(format!("播放格式错误!")))
                .unwrap();
            let sender_clone = sender_clone.clone();
            // 刷新播放列表
            task::spawn(async move {
                update_player_list(sender_clone).await.ok();
            });
        });

        // The following callbacks require `Send` but are handled by the gtk main loop
        let weak = Fragile::new(Rc::clone(self));
        // Update the duration label and the slider
        self.player.connect_duration_changed(move |_, clock| {
            weak.get().timer.on_duration_changed(Duration(clock));
        });

        let weak = Fragile::new(Rc::clone(self));
        // Update the position label and the slider
        self.player.connect_position_updated(move |_, clock| {
            weak.get().timer.on_position_updated(Position(clock));
        });

        let weak = Fragile::new(Rc::clone(self));
        // Reset the slider to 0 and show a play button
        self.player.connect_end_of_stream(move |_| {
             weak.get().stop();
        });
    }

    fn connect_loops_buttons(&self) {
        let weak = Rc::clone(self);

        self.loops.shuffle.connect_toggled(clone!(@weak weak => move |_| {
            *weak.loops_state.borrow_mut() = LoopsState::SHUFFLE;
            weak.loops.image.set_from_icon_name(Some("media-playlist-shuffle-symbolic"),gtk::IconSize::Menu);
            task::spawn(async {
                if let Ok(mut conf) = get_config().await {
                    conf.loops = LoopsState::SHUFFLE;
                    save_config(&conf).await.ok();
                }
            });
        }));

        self.loops.playlist.connect_toggled(clone!(@weak weak => move |_| {
            *weak.loops_state.borrow_mut() = LoopsState::PLAYLIST;
            weak.loops.image.set_from_icon_name(Some("media-playlist-repeat-symbolic"),gtk::IconSize::Menu);
            task::spawn(async {
                if let Ok(mut conf) = get_config().await {
                    conf.loops = LoopsState::PLAYLIST;
                    save_config(&conf).await.ok();
                }
            });
        }));

        self.loops.none.connect_toggled(clone!(@weak weak => move |_| {
            *weak.loops_state.borrow_mut() = LoopsState::NONE;
            weak.loops.image.set_from_icon_name(Some("media-playlist-repeat-song-symbolic"),gtk::IconSize::Menu);
            task::spawn(async {
                if let Ok(mut conf) = get_config().await {
                    conf.loops = LoopsState::NONE;
                    save_config(&conf).await.ok();
                }
            });
        }));
        self.loops.consecutive.connect_toggled(clone!(@weak weak => move |_| {
            *weak.loops_state.borrow_mut() = LoopsState::CONSECUTIVE;
            weak.loops.image.set_from_icon_name(Some("media-playlist-consecutive-symbolic"),gtk::IconSize::Menu);
            task::spawn(async {
                if let Ok(mut conf) = get_config().await {
                    conf.loops = LoopsState::CONSECUTIVE;
                    save_config(&conf).await.ok();
                }
            });
        }));
    }

    fn connect_mpris_buttons(&self) {
        let weak = Rc::clone(self);

        self.info
            .mpris
            .connect_play_pause(clone!(@weak weak, @weak self.info.mpris as mpris => move || {
                if let Ok(status) = mpris.get_playback_status() {
                    match status.as_ref() {
                        "Paused" => weak.play(),
                        "Stopped" => weak.play(),
                        _ => weak.pause(),
                    };
                }
            }));

        self.info.mpris.connect_play(clone!(@weak weak => move || {
            weak.play();
        }));

        self.info.mpris.connect_pause(clone!(@weak weak => move || {
            weak.pause();
        }));

        self.info.mpris.connect_next(clone!(@weak weak => move || {
            weak.forward();
        }));

        self.info.mpris.connect_previous(clone!(@weak weak => move || {
            weak.backward();
        }));
    }
}
