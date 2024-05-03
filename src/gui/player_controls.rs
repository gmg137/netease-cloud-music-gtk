//
// player_controls.rs
// Copyright (C) 2022 gmg137 <gmg137 AT live.com>
// Distributed under terms of the GPL-3.0-or-later license.
//
use async_channel::Sender;
use gettextrs::gettext;
use gio::Settings;
use glib::{
    clone, source::Priority, ParamSpec, ParamSpecBoolean, ParamSpecDouble, ParamSpecEnum,
    ParamSpecUInt, ParamSpecUInt64, Value,
};
use gst::{prelude::ObjectExt, ClockTime};
use gstreamer_play::{prelude::ElementExt, *};
use gtk::{glib, prelude::*, subclass::prelude::*, CompositeTemplate, GestureClick, *};
use log::*;
use mpris_server::PlaybackStatus;
use ncm_api::{SongInfo, SongList};
use once_cell::sync::*;

use crate::{application::Action, audio::*, model::ImageDownloadImpl, path::CACHE};
use std::{
    cell::Cell,
    fs, path,
    rc::Rc,
    sync::{Arc, Mutex},
    thread,
};

glib::wrapper! {
    pub struct PlayerControls(ObjectSubclass<imp::PlayerControls>)
        @extends gtk::Widget, gtk::Box,
        @implements gtk::Accessible, gtk::Buildable,gtk::ConstraintTarget, gtk::Orientable;
}

impl PlayerControls {
    pub fn new() -> Self {
        let player_controls: PlayerControls = glib::Object::new();
        player_controls
    }

    pub fn set_sender(&self, sender: Sender<Action>) {
        self.imp().sender.set(sender).unwrap();
        self.connect_gst_signals();
        self.bind_click();
        self.setup_mpris();
    }

    fn setup_settings(&self) {
        let settings = Settings::new(crate::APP_ID);

        self.imp()
            .settings
            .set(settings)
            .expect("Could not set `Settings`.");
    }

    fn settings(&self) -> &Settings {
        self.imp().settings.get().expect("Could not get settings.")
    }

    fn load_settings(&self) {
        let settings = self.settings();
        let loop_state = settings.string("repeat-variant");
        let loop_state = LoopsState::from_str(loop_state.as_str());

        self.set_loops(loop_state);
        self.set_volume(if settings.boolean("mute-start") {
            0.0
        } else {
            settings.double("volume")
        });

        settings.bind("music-rate", self, "music-rate").build();
    }

    pub fn setup_mpris(&self) {
        let imp = self.imp();
        let sender = imp.sender.get().unwrap().clone();
        crate::MAINCONTEXT.spawn_local_with_priority(Priority::LOW, async move {
            if let Ok(mc) = MprisController::new().await {
                sender.send(Action::InitMpris(mc)).await.unwrap();
            }
        });
    }

    pub fn init_mpris(&self, mpris: MprisController) {
        let imp = self.imp();
        imp.mpris.set(Rc::new(mpris)).unwrap();

        if let Some(mpris) = self.imp().mpris.get() {
            mpris.setup_signals(self);
        }
    }

    pub fn setup_player(&self) {
        let imp = self.imp();
        let player = Play::new(None::<PlayVideoRenderer>);
        let player_signal = PlaySignalAdapter::new(&player);
        let mut config = player.config();
        config.set_user_agent(
            "User-Agent: Mozilla/5.0 (X11; Linux x86_64; rv:100.0) Gecko/20100101 Firefox/100.0",
        );
        config.set_position_update_interval(250);
        config.set_seek_accurate(true);
        player.set_config(config).unwrap();
        player.set_volume(0.0);

        let pipeline = player.pipeline();

        let flags = pipeline.property_value("flags");
        let flags_class = glib::FlagsClass::with_type(flags.type_()).unwrap();
        let flags = flags_class
            .builder_with_value(flags)
            .unwrap()
            .set_by_nick("download")
            .build()
            .unwrap();
        pipeline.set_property_from_value("flags", &flags);

        imp.player.set(player).unwrap();
        imp.player_signal.set(player_signal).unwrap();
    }

    pub fn play(&self, song_info: SongInfo) {
        let imp = self.imp();

        let sender = imp.sender.get().unwrap();
        sender
            .send_blocking(Action::AddToast(gettext!(
                "Start playback [{}] ...",
                song_info.name
            )))
            .unwrap();

        let player = imp.player.get().unwrap();
        player.stop();
        player.set_uri(Some(&song_info.song_url));
        player.set_volume(self.property("volume"));
        player.play();

        self.init_play_info(song_info);
    }

    pub fn next_song(&self) {
        let next_button = self.imp().next_button.get();
        next_button.emit_clicked();
    }

    pub fn prev_song(&self) {
        let prev_button = self.imp().prev_button.get();
        prev_button.emit_clicked();
    }

    pub fn init_play_info(&self, song_info: SongInfo) {
        let imp = self.imp();
        let cover_image = imp.cover_image.get();
        let mut path_cover = CACHE.clone();
        path_cover.push(format!("{}-songlist.jpg", song_info.album_id));
        if path_cover.exists() {
            cover_image.set_from_file(Some(&path_cover));
        } else {
            cover_image.set_from_icon_name(Some("image-missing-symbolic"));
            let sender = imp.sender.get().unwrap().clone();
            cover_image.set_from_net(
                song_info.pic_url.to_owned(),
                path_cover.to_owned(),
                (140, 140),
                &sender,
            );
        }

        let title_label = imp.title_label.get();
        title_label.set_label(&song_info.name);
        title_label.set_tooltip_text(Some(&song_info.name));

        let artist_label = imp.artist_label.get();
        artist_label.set_label(&song_info.singer);

        let volume = self.property("volume");
        if let Some(mpris) = imp.mpris.get() {
            crate::MAINCONTEXT.spawn_local_with_priority(
                Priority::LOW,
                clone!(@weak mpris => async move {
                    if let Err(err) = mpris.update_metadata(&song_info).await {
                        warn!("设置 MPRIS metadata 失败: {err:?}");
                    }
                    if let Err(err) = mpris.set_playback_status(PlaybackStatus::Playing).await {
                        warn!("设置 MPRIS 播放状态失败: {err:?}");
                    }
                    if let Err(err) = mpris.set_volume(volume).await {
                        warn!("设置 MPRIS 音量失败: {err:?}");
                    }
                    mpris.set_position(0);
                    mpris.seeked(0).await.ok();
                }),
            );
        }
    }

    pub fn connect_gst_signals(&self) {
        let imp = self.imp();
        let sender_ = imp.sender.get().unwrap().clone();
        let player = imp.player.get().unwrap();
        let player_sig = imp.player_signal.get().unwrap();

        let sender = sender_.clone();
        // need gstplay's playbin bus
        let bus = player.pipeline().bus().unwrap();
        bus.connect_message(Some("element"), move |_, msg| {
            use gst::MessageView;
            if let MessageView::Element(ele) = msg.view() {
                if let Some(stu) = ele.structure() {
                    if "GstCacheDownloadComplete" == stu.name() {
                        if let Ok(loc) = stu.get::<String>("location") {
                            sender
                                .send_blocking(Action::GstCacheDownloadComplete(loc))
                                .unwrap();
                        }
                    }
                }
            }
        });

        let sender = sender_.clone();
        let old_msec: Cell<u64> = Cell::new(0);
        player_sig.connect_position_updated(move |_, clock| {
            if let Some(clock) = clock {
                // mseconds -> milliseconds
                // useconds -> microseconds
                let msec = clock.mseconds();
                if old_msec.get() / 500 != msec / 500 {
                    sender
                        .send_blocking(Action::ScaleSeekUpdate(clock.useconds()))
                        .unwrap();
                    old_msec.replace(msec);
                }
            }
        });

        let sender = sender_.clone();
        player_sig.connect_duration_changed(move |_, clock| {
            if let Some(clock) = clock {
                sender
                    .send_blocking(Action::GstDurationChanged(clock.useconds()))
                    .unwrap();
            }
        });

        let sender = sender_.clone();
        player_sig.connect_end_of_stream(move |_| {
            sender.send_blocking(Action::PlayNextSong).unwrap();
        });

        let sender = sender_.clone();
        player_sig.connect_error(move |_, e, _| {
            sender
                .send_blocking(Action::AddToast(gettext!(
                    "Playback error:{}",
                    e.to_string()
                )))
                .unwrap();
            sender.send_blocking(Action::PlayNextSong).unwrap();
        });

        let sender = sender_.clone();
        player_sig.connect_state_changed(move |_, state| {
            sender
                .send_blocking(Action::GstStateChanged(state))
                .unwrap();
        });

        let sender = sender_.clone();
        player_sig.connect_volume_changed(move |_, volume| {
            sender
                .send_blocking(Action::GstVolumeChanged(volume))
                .unwrap();
        });

        // let sender = sender_.clone();
        // player_sig.connect_buffering(move |_, percent| {});
    }

    // msec -> microseconds
    pub fn scale_seek_update(&self, msec: u64) {
        let imp = self.imp();

        let seek_scale = imp.seek_scale.get();
        seek_scale.set_value(msec as f64);

        let sec = msec / 10u64.pow(6);
        let duration = format!("{:0>2}:{:0>2}", sec / 60, sec % 60);
        imp.progress_time_label.get().set_label(&duration);

        if let Some(mpris) = self.imp().mpris.get() {
            mpris.set_position(msec as i64);
            crate::MAINCONTEXT.spawn_local_with_priority(
                Priority::LOW,
                clone!(@weak mpris => async move {
                    mpris.seeked(msec as i64).await.ok();
                }),
            );
        }
    }

    pub fn scale_value_update(&self) {
        let value: f64 = self.property("scale-value");
        self.gst_position_update(value as u64);

        if let Some(mpris) = self.imp().mpris.get() {
            mpris.set_position(value as i64);
            crate::MAINCONTEXT.spawn_local_with_priority(
                Priority::LOW,
                clone!(@weak mpris => async move {
                    mpris.seeked(value as i64).await.ok();
                }),
            );
        }
    }

    // msec -> microseconds
    pub fn gst_position_update(&self, msec: u64) {
        let imp = self.imp();
        let player = imp.player.get().unwrap();
        player.seek(ClockTime::from_useconds(msec));
    }

    pub fn gst_duration_changed(&self, msec: u64) {
        let imp = self.imp();
        let sec = msec / 10u64.pow(6);

        let duration = format!("{:0>2}:{:0>2}", sec / 60, sec % 60);

        imp.seek_scale.set_range(0.0, msec as f64);
        imp.duration_label.get().set_label(&duration);

        self.set_property("duration", sec);

        if let Some(mpris) = imp.mpris.get() {
            if let Some(mut si) = self.get_current_song() {
                si.duration = msec / 1000;
                crate::MAINCONTEXT.spawn_local_with_priority(
                    Priority::LOW,
                    clone!(@weak mpris => async move {
                        mpris.update_metadata(&si).await.ok();
                    }),
                );
            }
        }
    }

    pub fn gst_state_changed(&self, state: PlayState) {
        let imp = self.imp();
        let play_button = imp.play_button.get();
        match state {
            PlayState::Stopped => play_button.set_icon_name("media-playback-start-symbolic"),
            PlayState::Paused => play_button.set_icon_name("media-playback-start-symbolic"),
            PlayState::Playing => play_button.set_icon_name("media-playback-pause-symbolic"),
            _ => (),
        }
    }

    pub fn gst_volume_changed(&self, volume: f64) {
        self.set_property("volume", volume);
    }

    pub fn gst_cache_download_complete(&self, loc: String) {
        let duration: u64 = self.property("duration");
        // 不缓存小于 30 秒时长的乐曲(vip试听)
        if duration > 30 {
            if let Some(si) = self.get_current_song() {
                let rate = self.property::<u32>("music-rate");
                let src = path::PathBuf::from(loc);
                let dst = crate::path::get_music_cache_path(si.id, rate);
                thread::spawn(|| {
                    if let Err(err) = fs::copy(src, dst) {
                        log::error!("{:?}", err);
                    }
                });
            }
        }
    }

    pub fn bind_shortcut(&self) {
        // 播放按钮
        let play_button = self.imp().play_button.get();
        let controller = ShortcutController::new();
        let trigger = ShortcutTrigger::parse_string("<primary>space").unwrap();
        let action = ActivateAction::get();
        let shortcut = Shortcut::new(Some(trigger), Some(action));
        controller.add_shortcut(shortcut);
        controller.set_scope(ShortcutScope::Global);
        play_button.add_controller(controller);

        // 上一曲按钮
        let prev_button = self.imp().prev_button.get();
        let controller = ShortcutController::new();
        let trigger = ShortcutTrigger::parse_string("<primary>b").unwrap();
        let action = ActivateAction::get();
        let shortcut = Shortcut::new(Some(trigger), Some(action));
        controller.add_shortcut(shortcut);
        controller.set_scope(ShortcutScope::Global);
        prev_button.add_controller(controller);

        // 下一曲按钮
        let next_button = self.imp().next_button.get();
        let controller = ShortcutController::new();
        let trigger = ShortcutTrigger::parse_string("<primary>n").unwrap();
        let action = ActivateAction::get();
        let shortcut = Shortcut::new(Some(trigger), Some(action));
        controller.add_shortcut(shortcut);
        controller.set_scope(ShortcutScope::Global);
        next_button.add_controller(controller);
    }

    // 绑定拖动播放进度条结束时的事件
    pub fn bind_click(&self) {
        let mut gesture = GestureClick::new();
        let seek_scale = self.imp().seek_scale.get();
        seek_scale
            .observe_controllers()
            .into_iter()
            .for_each(|collection| {
                if let Ok(event) = collection {
                    if event.type_() == GestureClick::static_type() {
                        gesture = event.downcast::<GestureClick>().unwrap();
                    }
                }
            });
        let sender = self.imp().sender.get().unwrap().clone();
        gesture.connect_released(move |_, _, _, _| {
            sender.send_blocking(Action::ScaleValueUpdate).unwrap();
        });
    }

    pub fn add_song(&self, song: SongInfo) {
        if let Ok(mut playlist) = self.imp().playlist.lock() {
            playlist.add_song(song);
        }
    }

    pub fn add_list(&self, list: Vec<SongInfo>) {
        let settings = self.settings();
        let not_ignore_grey = settings.get("not-ignore-grey");
        let list: Vec<SongInfo> = if not_ignore_grey {
            list
        } else {
            list.into_iter()
                .filter(|si| si.copyright.playable())
                .collect()
        };

        if let Ok(mut playlist) = self.imp().playlist.lock() {
            playlist.add_list(list);
        }
    }

    pub fn get_next_song(&self) -> Option<SongInfo> {
        if let Ok(mut playlist) = self.imp().playlist.lock() {
            return playlist.next_song().map(|s| s.to_owned());
        }
        None
    }

    pub fn get_prev_song(&self) -> Option<SongInfo> {
        if let Ok(mut playlist) = self.imp().playlist.lock() {
            return playlist.prev_song().map(|s| s.to_owned());
        }
        None
    }

    pub fn get_current_song(&self) -> Option<SongInfo> {
        if let Ok(playlist) = self.imp().playlist.lock() {
            return playlist.current_song().map(|s| s.to_owned());
        }
        None
    }

    pub fn switch_play(&self) {
        let imp = self.imp();
        let player = imp.player.get().unwrap();
        player.play();

        if let Some(mpris) = imp.mpris.get() {
            crate::MAINCONTEXT.spawn_local_with_priority(
                Priority::LOW,
                clone!(@weak mpris => async move {
                    if let Err(err) = mpris.set_playback_status(PlaybackStatus::Playing).await {
                        warn!("设置 MPRIS 播放状态失败: {err:?}");
                    }
                }),
            );
        }
    }

    pub fn switch_pause(&self) {
        let imp = self.imp();
        let player = imp.player.get().unwrap();
        player.pause();

        if let Some(mpris) = imp.mpris.get() {
            crate::MAINCONTEXT.spawn_local_with_priority(
                Priority::LOW,
                clone!(@weak mpris => async move {
                    if let Err(err) = mpris.set_playback_status(PlaybackStatus::Paused).await {
                        warn!("设置 MPRIS 播放状态失败: {err:?}");
                    }
                }),
            );
        }
    }

    pub fn switch_stop(&self) {
        let imp = self.imp();
        let player = imp.player.get().unwrap();
        player.stop();

        if let Some(mpris) = imp.mpris.get() {
            crate::MAINCONTEXT.spawn_local_with_priority(
                Priority::LOW,
                clone!(@weak mpris => async move {
                    if let Err(err) = mpris.set_playback_status(PlaybackStatus::Stopped).await {
                        warn!("设置 MPRIS 播放状态失败: {err:?}");
                    }
                }),
            );
        }
    }

    // these set funcs will be called from mpris
    pub fn set_loops(&self, state: LoopsState) {
        if self.property::<LoopsState>("loops") != state {
            self.set_property("loops", state);
        }
    }

    pub fn set_shuffle(&self, shuffle: bool) {
        let imp = self.imp();
        match shuffle {
            true => self.set_loops(LoopsState::Shuffle),
            false => {
                if let Ok(status) = imp.mpris.get().unwrap().get_loop_status() {
                    self.set_loops(status);
                };
            }
        }
    }

    pub fn set_volume(&self, value: f64) {
        let old: f64 = self.property("volume");
        if (old * 100.0).round() as i64 != (value * 100.0).round() as i64 {
            self.set_property("volume", value);
            let player = self.imp().player.get().unwrap();
            player.set_volume(value);
        }
    }

    pub fn setup_notify_connect(&self) {
        self.connect_notify(None, move |s, p| {
            s.property_changed(p.name(), p);
        });
    }

    fn property_changed(&self, name: &str, _: &ParamSpec) {
        let imp = self.imp();
        match name {
            "volume" => {
                let value = self.property::<f64>("volume");
                self.imp().volume_button.get().set_value(value);
                if let Some(mpris) = imp.mpris.get() {
                    crate::MAINCONTEXT.spawn_local_with_priority(
                        Priority::LOW,
                        clone!(@weak mpris => async move {
                            if let Err(err) = mpris.set_volume(value).await {
                                warn!("设置 MPRIS 音量失败: {err:?}");
                            }
                        }),
                    );
                }
            }
            "loops" => {
                let value = self.property::<LoopsState>("loops");
                let switch: gtk::CheckButton = match value {
                    LoopsState::Shuffle => imp.shuffle_button.get(),
                    LoopsState::None => imp.none_button.get(),
                    LoopsState::Track => imp.one_button.get(),
                    LoopsState::Playlist => imp.loop_button.get(),
                };
                if !switch.is_active() {
                    switch.set_active(true);
                }

                if let Some(mpris) = imp.mpris.get() {
                    crate::MAINCONTEXT.spawn_local_with_priority(
                        Priority::LOW,
                        clone!(@weak mpris => async move {
                            if let Err(err) = mpris.set_loop_status(value).await {
                                warn!("设置 MPRIS 循环状态失败: {err:?}");
                            }
                        }),
                    );
                }

                if let Ok(mut playlist) = imp.playlist.lock() {
                    playlist.set_loops(value);
                }

                self.settings()
                    .set_string("repeat-variant", value.to_string().as_str())
                    .unwrap();
            }
            _ => (),
        }
    }
}

impl Default for PlayerControls {
    fn default() -> Self {
        Self::new()
    }
}

#[gtk::template_callbacks]
impl PlayerControls {
    #[template_callback]
    fn volume_cb(&self, adj: Adjustment) {
        self.set_volume(adj.value());
    }

    #[template_callback]
    fn cover_clicked_cb(&self) {
        let sender = self.imp().sender.get().unwrap().clone();
        if let Some(songinfo) = self.get_current_song() {
            if songinfo.album_id != 0 {
                let songlist = SongList {
                    id: songinfo.album_id,
                    name: songinfo.album,
                    cover_img_url: songinfo.pic_url,
                    author: String::new(),
                };
                sender.send_blocking(Action::ToAlbumPage(songlist)).unwrap();
            } else {
                sender
                    .send_blocking(Action::AddToast(gettext("Album not found!")))
                    .unwrap();
            }
        }
    }

    #[template_callback]
    fn title_clicked_cb(&self) {
        if let Some(songinfo) = self.get_current_song() {
            let sender = self.imp().sender.get().unwrap().clone();
            let clipboard = self.clipboard();
            let share = gettext!(
                "https://music.163.com/song?id={}\nsong:{}\nsinger:{}",
                songinfo.id,
                songinfo.name,
                songinfo.singer
            );
            clipboard.set_text(&share);
            sender
                .send_blocking(Action::AddToast(gettext(
                    "Copied song information to the clipboard!",
                )))
                .unwrap();
        }
    }
}

mod imp {

    use gst::glib::Propagation;

    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/gitee/gmg137/NeteaseCloudMusicGtk4/gtk/player-controls.ui")]
    pub struct PlayerControls {
        #[template_child]
        pub prev_button: TemplateChild<Button>,
        #[template_child]
        pub play_button: TemplateChild<Button>,
        #[template_child]
        pub next_button: TemplateChild<Button>,
        #[template_child]
        pub cover_image: TemplateChild<Image>,
        #[template_child]
        pub title_label: TemplateChild<Label>,
        #[template_child]
        pub artist_label: TemplateChild<Label>,
        #[template_child]
        pub seek_scale: TemplateChild<Scale>,
        #[template_child]
        pub progress_time_label: TemplateChild<Label>,
        #[template_child]
        pub duration_label: TemplateChild<Label>,
        #[template_child]
        pub volume_button: TemplateChild<VolumeButton>,

        #[template_child]
        pub repeat_menu_button: TemplateChild<MenuButton>,
        #[template_child]
        pub repeat_image: TemplateChild<Image>,
        #[template_child(id = "none")]
        pub none_button: TemplateChild<CheckButton>,
        #[template_child(id = "one")]
        pub one_button: TemplateChild<CheckButton>,
        #[template_child(id = "loop")]
        pub loop_button: TemplateChild<CheckButton>,
        #[template_child(id = "shuffle")]
        pub shuffle_button: TemplateChild<CheckButton>,
        #[template_child(id = "like_button")]
        pub like_button: TemplateChild<Button>,

        pub settings: OnceCell<Settings>,
        pub sender: OnceCell<Sender<Action>>,
        pub player: OnceCell<gstreamer_play::Play>,
        pub player_signal: OnceCell<gstreamer_play::PlaySignalAdapter>,
        pub playlist: Arc<Mutex<PlayList>>,
        pub mpris: OnceCell<Rc<MprisController>>,

        volume: Cell<f64>,
        loops: Cell<LoopsState>,
        music_rate: Cell<u32>,
        duration: Cell<u64>,

        like: Cell<bool>,

        // 播放条拖动结束时的值
        scale_value: Cell<f64>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for PlayerControls {
        const NAME: &'static str = "PlayerControls";
        type Type = super::PlayerControls;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_callbacks();
            klass.bind_template_instance_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[gtk::template_callbacks]
    impl PlayerControls {
        #[template_callback]
        fn prev_button_clicked_cb(&self) {
            let sender = self.sender.get().unwrap().clone();
            if let Ok(mut playlist) = self.playlist.lock() {
                if let Some(song_info) = playlist.prev_song() {
                    let song_info = song_info.to_owned();
                    sender
                        .send_blocking(Action::Play(song_info.to_owned()))
                        .unwrap();
                    sender
                        .send_blocking(Action::UpdatePlayListStatus(
                            playlist.get_position(),
                            song_info,
                        ))
                        .unwrap();
                    return;
                }
            }
            sender
                .send_blocking(Action::AddToast(gettext("No more songs！")))
                .unwrap();
        }

        #[template_callback]
        fn play_button_clicked_cb(&self, button: Button) {
            let player = self.player.get().unwrap();
            if button
                .icon_name()
                .unwrap()
                .eq("media-playback-start-symbolic")
            {
                player.play();
                button.set_icon_name("media-playback-pause-symbolic");
                if let Some(mpris) = self.mpris.get() {
                    crate::MAINCONTEXT.spawn_local_with_priority(
                        Priority::LOW,
                        clone!(@weak mpris => async move {
                            if let Err(err) = mpris.set_playback_status(PlaybackStatus::Playing).await {
                                warn!("设置 MPRIS 播放状态失败: {err:?}");
                            }
                        }),
                    );
                }
            } else {
                player.pause();
                button.set_icon_name("media-playback-start-symbolic");
                if let Some(mpris) = self.mpris.get() {
                    crate::MAINCONTEXT.spawn_local_with_priority(
                        Priority::LOW,
                        clone!(@weak mpris => async move {
                            if let Err(err) = mpris.set_playback_status(PlaybackStatus::Paused).await {
                                warn!("设置 MPRIS 播放状态失败: {err:?}");
                            }
                        }),
                    );
                }
            }
        }

        #[template_callback]
        fn next_button_clicked_cb(&self) {
            let sender = self.sender.get().unwrap().clone();
            if let Ok(mut playlist) = self.playlist.lock() {
                if let Some(song_info) = playlist.next_song() {
                    let song_info = song_info.to_owned();
                    sender
                        .send_blocking(Action::Play(song_info.to_owned()))
                        .unwrap();
                    sender
                        .send_blocking(Action::UpdatePlayListStatus(
                            playlist.get_position(),
                            song_info,
                        ))
                        .unwrap();
                    return;
                }
            }
            sender
                .send_blocking(Action::AddToast(gettext("No more songs！")))
                .unwrap();
        }

        #[template_callback]
        fn seek_scale_cb(&self, _: ScrollType, value: f64) -> Propagation {
            let sec = value as u64 / 10u64.pow(6);
            let duration = format!("{:0>2}:{:0>2}", sec / 60, sec % 60);
            self.progress_time_label.get().set_label(&duration);

            self.scale_value.set(value);

            Propagation::Proceed
        }

        #[template_callback]
        fn like_button_cb(&self) {
            let sender = self.sender.get().unwrap().clone();
            if let Ok(playlist) = self.playlist.lock() {
                if let Some(song_info) = playlist.current_song() {
                    sender
                        .send_blocking(Action::LikeSong(song_info.id, !self.like.get(), None))
                        .unwrap();
                    return;
                }
            }
            sender
                .send_blocking(Action::AddToast(gettext("Collection failure！")))
                .unwrap();
        }

        #[template_callback]
        fn repeat_none_cb(&self) {
            self.repeat_image
                .set_icon_name(Some("media-playlist-consecutive-symbolic"));

            self.obj().set_loops(LoopsState::None);
        }

        #[template_callback]
        fn repeat_one_cb(&self) {
            self.repeat_image
                .set_icon_name(Some("media-playlist-repeat-song-symbolic"));

            self.obj().set_loops(LoopsState::Track);
        }

        #[template_callback]
        fn repeat_loop_cb(&self) {
            self.repeat_image
                .set_icon_name(Some("media-playlist-repeat-symbolic"));

            self.obj().set_loops(LoopsState::Playlist);
        }

        #[template_callback]
        fn repeat_shuffle_cb(&self) {
            self.repeat_image
                .set_icon_name(Some("media-playlist-shuffle-symbolic"));

            self.obj().set_loops(LoopsState::Shuffle);
        }

        #[template_callback]
        fn playlist_lyrics_cb(&self) {
            if let Ok(playlist) = self.playlist.lock() {
                let current_song = playlist
                    .current_song()
                    .unwrap_or(&SongInfo {
                        id: 0,
                        name: String::new(),
                        singer: String::new(),
                        album: String::new(),
                        album_id: 0,
                        pic_url: String::new(),
                        duration: 0,
                        song_url: String::new(),
                        copyright: ncm_api::SongCopyright::Unknown,
                    })
                    .to_owned();
                let sender = self.sender.get().unwrap().clone();
                sender
                    .send_blocking(Action::ToPlayListLyricsPage(
                        playlist.get_list(),
                        current_song,
                    ))
                    .unwrap();
            }
        }
    }

    impl ObjectImpl for PlayerControls {
        fn constructed(&self) {
            let obj = self.obj();
            self.parent_constructed();
            *self.playlist.lock().unwrap() = PlayList::new();

            obj.setup_player();
            obj.setup_settings();

            obj.setup_notify_connect();

            obj.load_settings();
            obj.bind_shortcut();

            obj.bind_property("like", &self.like_button.get(), "icon_name")
                .transform_to(|_, v: bool| {
                    Some(
                        (if v {
                            "starred-symbolic"
                        } else {
                            "non-starred-symbolic"
                        })
                        .to_string(),
                    )
                })
                .build();
        }

        fn properties() -> &'static [ParamSpec] {
            static PROPERTIES: Lazy<Vec<ParamSpec>> = Lazy::new(|| {
                vec![
                    ParamSpecDouble::builder("volume").build(),
                    ParamSpecEnum::builder::<LoopsState>("loops").build(),
                    ParamSpecUInt::builder("music-rate").build(),
                    ParamSpecUInt64::builder("duration").build(),
                    ParamSpecBoolean::builder("like").readwrite().build(),
                    ParamSpecDouble::builder("scale-value").readwrite().build(),
                ]
            });
            PROPERTIES.as_ref()
        }

        fn set_property(&self, _id: usize, value: &Value, pspec: &ParamSpec) {
            match pspec.name() {
                "volume" => {
                    let input_number = value.get().expect("The value needs to be of type `f64`.");
                    self.volume.replace(input_number);
                }
                "loops" => {
                    let val = value.get().unwrap();
                    self.loops.replace(val);
                }
                "music-rate" => {
                    let val = value.get().unwrap();
                    self.music_rate.replace(val);
                }
                "duration" => {
                    let val = value.get().unwrap();
                    self.duration.replace(val);
                }
                "like" => {
                    let like = value.get().expect("The value needs to be of type `bool`.");
                    self.like.replace(like);
                }
                "scale-value" => {
                    let scale_value = value.get().expect("The value needs to be of type `bool`.");
                    self.scale_value.replace(scale_value);
                }
                n => unimplemented!("{}", n),
            }
        }

        fn property(&self, _id: usize, pspec: &ParamSpec) -> Value {
            match pspec.name() {
                "volume" => self.volume.get().to_value(),
                "loops" => self.loops.get().to_value(),
                "music-rate" => self.music_rate.get().to_value(),
                "duration" => self.duration.get().to_value(),
                "like" => self.like.get().to_value(),
                "scale-value" => self.scale_value.get().to_value(),
                n => unimplemented!("{}", n),
            }
        }

        fn dispose(&self) {
            let obj = self.obj();
            obj.settings()
                .set_double("volume", obj.property("volume"))
                .unwrap();
        }
    }
    impl WidgetImpl for PlayerControls {}
    impl BoxImpl for PlayerControls {}
}
