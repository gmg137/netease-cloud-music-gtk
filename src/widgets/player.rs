//
// player.rs
// Copyright (C) 2019 gmg137 <gmg137@live.com>
// Distributed under terms of the GPLv3 license.
//
use crate::{app::Action, data::MusicData, model::NCM_CACHE, musicapi::model::SongInfo, task::Task, utils::*};
use async_std::{sync, task};
use chrono::NaiveTime;
use fragile::Fragile;
use futures::{channel::mpsc, sink::SinkExt};
use gdk_pixbuf::{InterpType, Pixbuf};
use glib::{clone, Sender, SignalHandlerId, WeakRef};
use gst::ClockTime;
use gtk::{
    prelude::*, AccelGroup, ActionBar, Builder, Button, CellRendererText, Image, Label, ListStore, MenuButton, Popover,
    RadioButton, Scale, TextView, TreeView, TreeViewColumn, VolumeButton,
};
use mpris_player::{LoopStatus, Metadata, MprisPlayer, OrgMprisMediaPlayer2Player, PlaybackStatus};
use pango::EllipsizeMode;
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
    more: MenuButton,
    more_popover: Popover,
    tree: TreeView,
    store: ListStore,
    lyrics_text: TextView,
}

#[derive(Debug, Clone)]
struct PlayerInfo {
    song: Label,
    singer: Label,
    cover: Image,
    mpris: Arc<MprisPlayer>,
    song_id: RefCell<Option<u64>>,
}

impl PlayerInfo {
    fn init(&self, song_info: &SongInfo) {
        self.song.set_text(&song_info.name);
        self.song.set_tooltip_text(Some(&song_info.name[..]));
        self.singer.set_text(&song_info.singer);
        let image_path = format!("{}{}.jpg", NCM_CACHE.to_string_lossy(), &song_info.id);
        if let Ok(image) = Pixbuf::from_file(&image_path) {
            let image = image.scale_simple(38, 38, InterpType::Bilinear);
            self.cover.set_from_pixbuf(image.as_ref());
        };
        self.cover.set_tooltip_text(Some(&song_info.name[..]));
        *self.song_id.borrow_mut() = Some(song_info.id);

        let mut metadata = Metadata::new();
        metadata.artist = Some(vec![song_info.singer.clone()]);
        metadata.title = Some(song_info.name.clone());
        let img_uri = format!("file:///{}{}.jpg", NCM_CACHE.to_string_lossy(), &song_info.id);
        if Path::new(&img_uri).exists() {
            metadata.art_url = Some(img_uri);
        } else {
            metadata.art_url = Some(song_info.pic_url.to_owned());
        }

        self.mpris.set_metadata(metadata);
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
        let seconds = duration.seconds();
        let useconds = duration.useconds();

        self.slider.block_signal(&self.slider_update);
        self.slider.set_range(0.0, useconds as f64);
        self.slider.unblock_signal(&self.slider_update);

        self.duration.set_text(&format_duration(seconds as u32));
    }

    pub(crate) fn on_position_updated(&self, position: Position) {
        let seconds = position.seconds();
        let useconds = position.useconds();

        self.slider.block_signal(&self.slider_update);
        self.slider.set_value(useconds as f64);
        self.slider.unblock_signal(&self.slider_update);

        self.progressed.set_text(&format_duration(seconds as u32));
    }
}

#[derive(Debug, Clone)]
struct PlayerLoops {
    shuffle: RadioButton,
    playlist: RadioButton,
    track: RadioButton,
    none: RadioButton,
    image: Image,
}

// 播放列表循环模式
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub(crate) enum LoopsState {
    // 随机
    SHUFFLE,
    // 列表循环
    PLAYLIST,
    // 单曲循环
    TRACK,
    // 不循环
    NONE,
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
    music_data: sync::Arc<sync::Mutex<MusicData>>,
}

impl PlayerWidget {
    fn new(
        builder: &Builder,
        sender: Sender<Action>,
        sender_task: mpsc::Sender<Task>,
        music_data: sync::Arc<sync::Mutex<MusicData>>,
    ) -> Self {
        let dispatcher = gst_player::PlayerGMainContextSignalDispatcher::new(None);
        let player = gst_player::Player::new(
            None,
            // Use the gtk main thread
            Some(&dispatcher.upcast::<gst_player::PlayerSignalDispatcher>()),
        );

        let mpris = MprisPlayer::new(
            "NeteaseCloudMusic".to_string(),
            "Netease Cloud Music".to_string(),
            "netease-cloud-music-gtk.desktop".to_string(),
        );
        mpris.set_can_raise(true);
        mpris.set_can_control(true);
        mpris.set_can_quit(true);
        mpris.set_can_seek(true);

        let mut config = player.config();
        config.set_user_agent("User-Agent: Mozilla/5.0 (X11; Linux x86_64; rv:65.0) Gecko/20100101 Firefox/65.0");
        config.set_position_update_interval(250);
        player.set_config(config).unwrap();

        let play: Button = builder.object("play_button").unwrap();
        let pause: Button = builder.object("pause_button").unwrap();
        let forward: Button = builder.object("forward_button").unwrap();
        let backward: Button = builder.object("backward_button").unwrap();
        let like: Button = builder.object("like_button").unwrap();
        let volume: VolumeButton = builder.object("volume_button").unwrap();

        let (volume_value, loop_state) = match task::block_on(get_config()) {
            Ok(config) => (config.volume, config.loops),
            _ => (0.30, LoopsState::NONE),
        };
        volume.set_value(volume_value);
        player.set_volume(volume_value);
        mpris.set_volume(volume_value).ok();
        let more: MenuButton = builder.object("more_button").unwrap();
        let more_popover: Popover = builder.object("more_popover").unwrap();
        let lyrics_text: TextView = builder.object("lyrics_text_view").unwrap();
        let tree: TreeView = builder
            .object("playlist_tree_view")
            .expect("无法获取 playlist_tree_view .");
        let store: ListStore = ListStore::new(&[
            glib::Type::U64,
            String::static_type(),
            String::static_type(),
            String::static_type(),
            String::static_type(),
        ]);

        let controls = PlayerControls {
            play,
            pause,
            forward,
            backward,
            like,
            volume,
            more,
            more_popover,
            tree,
            store,
            lyrics_text,
        };

        let progressed: Label = builder.object("progress_time_label").unwrap();
        let duration: Label = builder.object("total_duration_label").unwrap();
        let slider: Scale = builder.object("seek").unwrap();
        slider.set_range(0.0, 1.0);
        let player_weak = player.downgrade();
        let slider_update = Rc::new(Self::connect_update_slider(&slider, player_weak));
        let timer = PlayerTimes {
            progressed,
            duration,
            slider,
            slider_update,
        };

        let song: Label = builder.object("song_label").unwrap();
        let singer: Label = builder.object("singer_label").unwrap();
        let cover: Image = builder.object("song_cover").unwrap();

        let shuffle: RadioButton = builder.object("shuffle_radio").unwrap();
        let playlist: RadioButton = builder.object("playlist_radio").unwrap();
        let track: RadioButton = builder.object("track_radio").unwrap();
        let none: RadioButton = builder.object("none_radio").unwrap();
        let image: Image = builder.object("loops_image").unwrap();
        let loops = PlayerLoops {
            shuffle,
            playlist,
            track,
            none,
            image,
        };

        let action_bar: ActionBar = builder.object("play_action_bar").unwrap();
        match loop_state {
            LoopsState::NONE => {
                loops
                    .image
                    .set_from_icon_name(Some("media-playlist-consecutive-symbolic"), gtk::IconSize::Menu);
                loops.none.set_active(true);
                mpris.set_loop_status(LoopStatus::None);
            },
            LoopsState::TRACK => {
                loops
                    .image
                    .set_from_icon_name(Some("media-playlist-repeat-song-symbolic"), gtk::IconSize::Menu);
                loops.track.set_active(true);
                mpris.set_loop_status(LoopStatus::Track);
            },
            LoopsState::PLAYLIST => {
                loops
                    .image
                    .set_from_icon_name(Some("media-playlist-repeat-symbolic"), gtk::IconSize::Menu);
                loops.playlist.set_active(true);
                mpris.set_loop_status(LoopStatus::Playlist);
            },
            LoopsState::SHUFFLE => {
                loops
                    .image
                    .set_from_icon_name(Some("media-playlist-shuffle-symbolic"), gtk::IconSize::Menu);
                loops.shuffle.set_active(true);
                mpris.property_changed("Shuffle".to_string(), true);
            },
        }
        let info = PlayerInfo {
            mpris,
            song,
            singer,
            cover,
            song_id: RefCell::new(None),
        };

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
            music_data,
        }
    }

    fn reveal(&self) {
        self.action_bar.show();
    }

    pub(crate) fn initialize_player(&self, song_info: SongInfo, player_types: PlayerTypes, lyrics: bool) {
        if let PlayerTypes::Fm = player_types {
            if song_info.id == self.info.song_id.borrow().unwrap_or(0) {
                self.play();
                return;
            }
            task::spawn(async {
                clear_playlist().await.unwrap_or(());
            });
        }
        *self.player_types.borrow_mut() = player_types;
        let sender = self.sender.clone();
        let mut sender_task = self.sender_task.clone();
        let data = self.music_data.clone();
        task::spawn(async move {
            let mut data = data.lock().await;
            // 下载歌词
            if lyrics {
                download_lyrics(&mut data, &song_info).await.ok();
            }
            let path = song_info.get_song_cache_path();
            if path.exists() {
                sender.send(Action::Player(song_info)).unwrap();
            } else if let Ok(v) = data.songs_url(&[song_info.id]).await {
                if !v.is_empty() {
                    let mut song_info = song_info;
                    song_info.song_url = v[0].url.to_string();
                    sender.send(Action::Player(song_info.clone())).unwrap();
                    // 缓存音乐和图片
                    let image_path = format!("{}{}.jpg", NCM_CACHE.to_string_lossy(), &song_info.id);
                    sender_task
                        .send(Task::DownloadPlayerImg {
                            url: song_info.pic_url.to_owned(),
                            path: image_path.to_owned(),
                            width: 140,
                            high: 140,
                            timeout: 1000,
                            fm: false,
                        })
                        .await
                        .ok();
                    sender_task
                        .send(Task::DownloadMusic {
                            song_info: song_info.clone(),
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
        });
    }

    pub(crate) fn ready_player(&self, song_info: SongInfo, lyrics: bool) {
        let sender = self.sender.clone();
        let mut song_info = song_info;
        let mut sender_task = self.sender_task.clone();
        // 是否刷新 FM 专辑图片
        let mut fm = false;
        let width = 140;
        let high = 140;
        if let PlayerTypes::Fm = *self.player_types.borrow() {
            fm = true;
        }
        let data = self.music_data.clone();
        task::spawn(async move {
            let mut data = data.lock().await;
            // 下载歌词
            if lyrics {
                download_lyrics(&mut data, &song_info).await.ok();
            }
            // 缓存音乐图片路径
            let image_path = format!("{}{}.jpg", NCM_CACHE.to_string_lossy(), song_info.id);
            // 缓存音乐路径
            let path = song_info.get_song_cache_path();
            // 检查是否已经缓存音乐
            if !path.exists() {
                if let Ok(v) = data.songs_url(&[song_info.id]).await {
                    if !v.is_empty() {
                        song_info.song_url = v[0].url.to_string();
                        // 缓存音乐
                        sender_task
                            .send(Task::DownloadMusic {
                                song_info: song_info.clone(),
                                path: path.clone(),
                                timeout: 3000,
                            })
                            .await
                            .ok();
                    } else {
                        sender.send(Action::ShowNotice("获取歌曲URL错误!".to_owned())).unwrap();
                    }
                } else {
                    sender.send(Action::ShowNotice("获取歌曲URL错误!".to_owned())).unwrap();
                }
            }
            // 播放音乐
            sender.send(Action::Player(song_info.clone())).unwrap();
            // 缓存封面
            sender_task
                .send(Task::DownloadPlayerImg {
                    url: song_info.pic_url.to_owned(),
                    path: image_path.to_owned(),
                    width,
                    high,
                    timeout: 1000,
                    fm,
                })
                .await
                .ok();
        });
    }

    pub(crate) fn player(&self, song_info: SongInfo) {
        info!("准备播放音乐: {:?}", song_info);
        if let PlayerTypes::Fm = *self.player_types.borrow() {
            self.sender.send(Action::RefreshMineFm(song_info.to_owned())).unwrap();
        }
        self.sender.send(Action::ShowNotice(song_info.name.to_owned())).unwrap();
        self.info.init(&song_info);
        let song_uri = song_info.get_song_cache_path();
        if song_uri.exists() {
            info!("播放音乐缓存: {}", song_uri.to_string_lossy());
            self.player
                .set_uri(Some(&format!("file:///{}", song_uri.to_string_lossy())));
        } else {
            let music_url = song_info.song_url.replace("https:", "http:");
            info!("播放在线音乐: {}", music_url);
            self.player.set_uri(Some(&music_url));
        }
        self.play();
        // 如果播放列表已打开则刷新播放列表
        if self.controls.more_popover.is_visible() {
            self.get_lyrics_text();
            self.get_playlist();
        }
    }

    fn connect_update_slider(slider: &Scale, player: WeakRef<gst_player::Player>) -> SignalHandlerId {
        slider.connect_value_changed(move |slider| {
            let player = match player.upgrade() {
                Some(p) => p,
                None => return,
            };

            let value = slider.value() as u64;
            player.seek(ClockTime::from_useconds(value));
        })
    }

    fn play(&self) {
        // 更新 FM 播放按钮
        match *self.player_types.borrow() {
            PlayerTypes::Fm => {
                self.sender.send(Action::RefreshMineFmPause).unwrap();
            },
            _ => self.sender.send(Action::RefreshMineFmPlay).unwrap(),
        }
        self.reveal();

        self.controls.pause.show();
        self.controls.play.hide();

        self.player.play();
        self.info.mpris.set_position(0);
        self.info.mpris.set_playback_status(PlaybackStatus::Playing);
    }

    pub(crate) fn pause(&self) {
        // 更新 FM 播放按钮
        if let PlayerTypes::Fm = *self.player_types.borrow() {
            self.sender.send(Action::RefreshMineFmPlay).unwrap();
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
        if let PlayerTypes::Fm = *self.player_types.borrow() {
            if let Ok(si) = task::block_on(get_player_list_song(PD::FORWARD, false, false)) {
                self.sender.send(Action::ReadyPlayer(si)).unwrap();
            } else {
                self.sender.send(Action::RefreshMineFmPlayerList).unwrap();
            }
            return;
        }
        let (shuffle, loops) = match *self.loops_state.borrow() {
            LoopsState::SHUFFLE => (true, false),
            LoopsState::PLAYLIST => (false, true),
            LoopsState::NONE => (false, false),
            LoopsState::TRACK => {
                self.play();
                return;
            },
        };
        if let Ok(si) = task::block_on(get_player_list_song(PD::FORWARD, shuffle, loops)) {
            self.sender.send(Action::ReadyPlayer(si)).unwrap();
        }
    }

    pub(crate) fn play_one(&self) {
        let (shuffle, loops) = match *self.loops_state.borrow() {
            LoopsState::SHUFFLE => (true, false),
            LoopsState::PLAYLIST => (false, true),
            LoopsState::NONE => (false, false),
            LoopsState::TRACK => (false, false),
        };
        if let Ok(si) = task::block_on(get_player_list_song(PD::FORWARD, shuffle, loops)) {
            self.sender.send(Action::ReadyPlayer(si)).unwrap();
        }
    }

    fn backward(&self) {
        let state = match *self.loops_state.borrow() {
            LoopsState::SHUFFLE => true,
            LoopsState::PLAYLIST => {
                if let Ok(si) = task::block_on(get_player_list_song(PD::BACKWARD, false, false)) {
                    self.sender.send(Action::ReadyPlayer(si)).unwrap();
                } else if let Ok(si) = task::block_on(get_player_list_song(PD::BACKWARD, false, true)) {
                    self.sender.send(Action::ReadyPlayer(si)).unwrap();
                }
                return;
            },
            LoopsState::TRACK => {
                self.stop();
                self.play();
                return;
            },
            LoopsState::NONE => false,
        };
        if let Ok(si) = task::block_on(get_player_list_song(PD::BACKWARD, state, false)) {
            self.sender.send(Action::ReadyPlayer(si)).unwrap();
        }
    }

    fn like(&self) {
        let song_id = *self.info.song_id.borrow();
        if let Some(id) = song_id {
            self.sender.send(Action::LikeSong(id)).unwrap();
            return;
        }
        self.sender.send(Action::ShowNotice("收藏失败!".to_owned())).unwrap();
    }

    fn set_volume(&self, value: f64, mpris: bool) {
        task::block_on(async move {
            if let Ok(mut config) = get_config().await {
                config.volume = value;
                save_config(&config).await.ok();
            }
        });
        self.player.set_volume(value);
        if mpris {
            self.controls.volume.set_value(value);
        }
    }

    fn get_lyrics_text(&self) {
        let sender = self.sender.clone();
        let song_id = *self.info.song_id.borrow();
        let data = self.music_data.clone();
        task::spawn(async move {
            let mut data = data.lock().await;
            if let Some(id) = song_id {
                let lrc = get_lyrics(&mut data, id)
                    .await
                    .unwrap_or_else(|_| "没有找到歌词!".to_owned());
                sender.send(Action::RefreshLyricsText(lrc)).unwrap();
            }
        });
    }

    fn get_playlist(&self) {
        let sender = self.sender.clone();
        task::spawn(async move {
            // 获取播放列表
            if let Ok(playlist) = get_playlist().await {
                sender.send(Action::RefreshPlaylist(playlist)).unwrap();
            }
        });
    }

    // 从 Mpris2 设置播放循环
    fn set_loops(&self, loops_status: LoopStatus) {
        match loops_status {
            LoopStatus::None => {
                self.loops.none.set_active(true);
            },
            LoopStatus::Track => {
                self.loops.track.set_active(true);
            },
            LoopStatus::Playlist => {
                self.loops.playlist.set_active(true);
            },
        }
    }

    // 从 Mpris2 设置混淆播放
    fn set_shuffle(&self, shuffle: bool) {
        if shuffle {
            self.loops.shuffle.set_active(true);
        } else if let Ok(status) = self.info.mpris.get_loop_status() {
            if status.eq("None") {
                self.set_loops(LoopStatus::None);
            } else if status.eq("Track") {
                self.set_loops(LoopStatus::Track);
            } else if status.eq("Playlist") {
                self.set_loops(LoopStatus::Playlist);
            } else {
                self.set_loops(LoopStatus::None);
            }
        }
    }

    pub(crate) fn update_lyrics_text(&self, lrc: String) {
        if let Some(buffer) = self.controls.lyrics_text.buffer() {
            buffer.set_text(&lrc);
            self.controls.lyrics_text.set_buffer(Some(&buffer));
        }
    }

    pub(crate) fn update_playlist(&self, pl: PlayerListData) {
        self.controls.store.clear();
        for c in self.controls.tree.columns().iter() {
            self.controls.tree.remove_column(c);
        }
        self.controls.tree.set_model(Some(&self.controls.store));

        let column = TreeViewColumn::new();
        column.set_visible(false);
        column.set_sizing(gtk::TreeViewColumnSizing::Fixed);
        let id = CellRendererText::new();
        column.pack_start(&id, true);
        column.add_attribute(&id, "text", 0);
        self.controls.tree.append_column(&column);

        let column = TreeViewColumn::new();
        column.set_sizing(gtk::TreeViewColumnSizing::Fixed);
        let play = CellRendererText::new();
        play.set_xpad(18);
        play.set_xalign(0.0);
        play.set_yalign(0.5);
        play.set_height(37);
        column.pack_start(&play, true);
        column.add_attribute(&play, "text", 1);
        self.controls.tree.append_column(&column);

        let column = TreeViewColumn::new();
        column.set_sizing(gtk::TreeViewColumnSizing::Fixed);
        let title = CellRendererText::new();
        play.set_xpad(18);
        play.set_xalign(0.0);
        title.set_ellipsize(EllipsizeMode::End);
        column.pack_start(&title, true);
        column.add_attribute(&title, "text", 2);

        let duration = CellRendererText::new();
        duration.set_xpad(32);
        duration.set_xalign(0.0);
        column.pack_start(&duration, true);
        column.add_attribute(&duration, "text", 3);

        let singer = CellRendererText::new();
        singer.set_xpad(22);
        singer.set_xalign(0.0);
        singer.set_ellipsize(EllipsizeMode::End);
        column.pack_start(&singer, true);
        column.add_attribute(&singer, "text", 4);
        self.controls.tree.append_column(&column);

        let song_id = *self.info.song_id.borrow();
        pl.player_list.iter().for_each(|(song, _)| {
            let play_icon = if Some(song.id).eq(&song_id) { "▶" } else { "" };
            self.controls.store.insert_with_values(
                None,
                &[
                    (0, &song.id),
                    (1, &play_icon),
                    (2, &song.name),
                    (3, &song.duration),
                    (4, &song.singer),
                ],
            );
        });
    }

    pub(crate) fn set_player_typers(&self, player_types: PlayerTypes) {
        *self.player_types.borrow_mut() = player_types;
    }

    pub(crate) fn playlist_song(&self, index: i32) {
        if task::block_on(get_playlist_song_by_index(index, self.sender.clone())).is_err() {
            self.sender.send(Action::ShowNotice("播放错误!".to_owned())).unwrap();
        }
    }

    pub(crate) fn set_cover_image(&self, image_path: String) {
        if let Ok(image) = Pixbuf::from_file(&image_path) {
            let image = image.scale_simple(38, 38, InterpType::Bilinear);
            self.info.cover.set_from_pixbuf(image.as_ref());
        };
    }

    // 添加快捷键
    pub(crate) fn play_add_accel(&self, ag: &AccelGroup) {
        self.controls
            .play
            .add_accelerator("clicked", ag, 32, gdk::ModifierType::empty(), gtk::AccelFlags::VISIBLE);
        self.controls
            .pause
            .add_accelerator("clicked", ag, 32, gdk::ModifierType::empty(), gtk::AccelFlags::VISIBLE);
    }

    // 删除快捷键
    pub(crate) fn play_remove_accel(&self, ag: &AccelGroup) {
        self.controls
            .play
            .remove_accelerator(ag, 32, gdk::ModifierType::empty());
        self.controls
            .pause
            .remove_accelerator(ag, 32, gdk::ModifierType::empty());
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
        sender_task: &mpsc::Sender<Task>,
        music_data: sync::Arc<sync::Mutex<MusicData>>,
    ) -> Self {
        let w = PlayerWrapper(Rc::new(PlayerWidget::new(
            builder,
            sender.clone(),
            sender_task.clone(),
            music_data,
        )));
        w.init(sender);
        w
    }

    fn init(&self, sender: &Sender<Action>) {
        self.connect_control_tree();
        self.connect_control_buttons();
        self.connect_loops_buttons();
        self.connect_mpris_buttons();
        self.connect_gst_signals(sender);
    }

    fn connect_control_tree(&self) {
        let sender = self.sender.clone();
        self.controls.tree.connect_button_press_event(move |tree, event| {
            if event.event_type() == gdk::EventType::DoubleButtonPress {
                if let Some(path) = tree.selection().selected_rows().0.get(0) {
                    let index = path.indices()[0];
                    sender.send(Action::PlaylistSong(index)).unwrap_or(());
                }
            }
            Inhibit(false)
        });
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
                weak.set_volume(value,false);
            }));

        self.controls.more.connect_clicked(clone!(@weak weak => move |_| {
            weak.get_lyrics_text();
            weak.get_playlist();
        }));
    }

    fn connect_gst_signals(&self, sender: &Sender<Action>) {
        // Log gst warnings.
        self.player
            .connect_warning(move |_, warn| warn!("gst warning: {}", warn));

        let sender_clone = sender.clone();
        // Log gst errors.
        let weak = Fragile::new(Rc::clone(self));
        self.player.connect_error(move |_, _| {
            sender_clone
                .send(Action::ShowNotice("播放格式错误!".to_owned()))
                .unwrap();
            weak.get().forward();
        });

        // The following callbacks require `Send` but are handled by the gtk main loop
        let weak = Fragile::new(Rc::clone(self));
        // Update the duration label and the slider
        self.player.connect_duration_changed(move |_, clock| {
            match clock {
                Some(c) => weak.get().timer.on_duration_changed(Duration(c)),
                _ => {},
            };
        });

        let weak = Fragile::new(Rc::clone(self));
        // Update the position label and the slider
        self.player.connect_position_updated(move |_, clock| {
            // 实时更新播放进度
            //if let Some(t) = clock.useconds() {
            //weak.get().info.mpris.set_position(t as i64);
            //}
            match clock {
                Some(c) => weak.get().timer.on_position_updated(Position(c)),
                _ => {},
            };
        });

        let weak = Fragile::new(Rc::clone(self));
        // Reset the slider to 0 and show a play button
        self.player.connect_end_of_stream(move |_| {
            weak.get().stop();
        });

        let weak = Fragile::new(Rc::clone(self));
        // 连接音量变化
        self.player.connect_volume_changed(move |p| {
            weak.get().controls.volume.set_value(p.volume());
        });

        let weak = Fragile::new(Rc::clone(self));
        // 连接进度条变化
        self.player.connect_seek_done(move |_, time| {
            let t = time.useconds();
            let weak = weak.get();
            weak.info.mpris.set_position(t as i64);
            weak.info.mpris.seek(t as i64).unwrap_or(());
        });
    }

    fn connect_loops_buttons(&self) {
        let weak = Rc::clone(self);

        self.loops.shuffle.connect_toggled(clone!(@weak weak => move |_| {
            *weak.loops_state.borrow_mut() = LoopsState::SHUFFLE;
            weak.loops.image.set_from_icon_name(Some("media-playlist-shuffle-symbolic"),gtk::IconSize::Menu);
            task::block_on(async {
                if let Ok(mut conf) = get_config().await {
                    conf.loops = LoopsState::SHUFFLE;
                    save_config(&conf).await.ok();
                }
            });
        }));

        self.loops.playlist.connect_toggled(clone!(@weak weak => move |_| {
            *weak.loops_state.borrow_mut() = LoopsState::PLAYLIST;
            weak.loops.image.set_from_icon_name(Some("media-playlist-repeat-symbolic"),gtk::IconSize::Menu);
            task::block_on(async {
                if let Ok(mut conf) = get_config().await {
                    conf.loops = LoopsState::PLAYLIST;
                    save_config(&conf).await.ok();
                }
            });
        }));

        self.loops.track.connect_toggled(clone!(@weak weak => move |_| {
            *weak.loops_state.borrow_mut() = LoopsState::TRACK;
            weak.loops.image.set_from_icon_name(Some("media-playlist-repeat-song-symbolic"),gtk::IconSize::Menu);
            task::block_on(async {
                if let Ok(mut conf) = get_config().await {
                    conf.loops = LoopsState::TRACK;
                    save_config(&conf).await.ok();
                }
            });
        }));
        self.loops.none.connect_toggled(clone!(@weak weak => move |_| {
            *weak.loops_state.borrow_mut() = LoopsState::NONE;
            weak.loops.image.set_from_icon_name(Some("media-playlist-consecutive-symbolic"),gtk::IconSize::Menu);
            task::block_on(async {
                if let Ok(mut conf) = get_config().await {
                    if conf.loops != LoopsState::NONE{
                        conf.loops = LoopsState::NONE;
                        save_config(&conf).await.ok();
                    }
                }
            });
        }));
    }

    fn connect_mpris_buttons(&self) {
        let weak = Rc::clone(self);

        self.info.mpris.connect_quit(clone!(@weak weak => move || {
            weak.sender.send(Action::QuitMain).unwrap();
        }));

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

        self.info.mpris.connect_raise(clone!(@weak weak => move || {
            weak.sender.send(Action::ActivateApp).unwrap();
        }));

        self.info.mpris.connect_volume(clone!(@weak weak => move |volume| {
            weak.set_volume(volume, true);
        }));

        self.info.mpris.connect_shuffle(clone!(@weak weak => move |status| {
            weak.set_shuffle(status);
        }));

        self.info.mpris.connect_loop_status(clone!(@weak weak => move |status| {
            weak.set_loops(status);
        }));
    }
}
