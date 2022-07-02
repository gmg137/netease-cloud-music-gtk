//
// player_controls.rs
// Copyright (C) 2022 gmg137 <gmg137 AT live.com>
// Distributed under terms of the GPL-3.0-or-later license.
//
use fragile::Fragile;
use gettextrs::gettext;
use gio::Settings;
use glib::{ParamFlags, ParamSpec, ParamSpecDouble, Sender, Value};
use gst::ClockTime;
use gstreamer_player::*;
use gtk::{glib, prelude::*, subclass::prelude::*, CompositeTemplate, *};
use mpris_player::{LoopStatus, PlaybackStatus};
use ncm_api::{SongInfo, SongList};
use once_cell::sync::*;

use crate::{application::Action, audio::*, ncmapi::COOKIE_JAR, path::CACHE};
use std::{
    cell::Cell,
    rc::Rc,
    sync::{Arc, Mutex},
    time::Duration,
};

glib::wrapper! {
    pub struct PlayerControls(ObjectSubclass<imp::PlayerControls>)
        @extends gtk::Widget, gtk::Box,
        @implements gtk::Accessible, gtk::Buildable,gtk::ConstraintTarget, gtk::Orientable;
}

impl PlayerControls {
    pub fn new() -> Self {
        let player_controls: PlayerControls =
            glib::Object::new(&[]).expect("Failed to create PlayerControls");
        player_controls
    }

    pub fn set_sender(&self, sender: Sender<Action>) {
        self.imp().sender.set(sender).unwrap();
        self.connect_gst_signals();
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
        let imp = self.imp();
        let settings = self.settings();
        let loop_state = settings.string("repeat-variant");
        let loop_state = match loop_state.as_str() {
            "none" => {
                imp.none_button.set_active(true);
                LoopsState::NONE
            }
            "one" => {
                imp.one_button.set_active(true);
                LoopsState::ONE
            }
            "loop" => {
                imp.loop_button.set_active(true);
                LoopsState::LOOP
            }
            "shuffle" => {
                imp.shuffle_button.set_active(true);
                LoopsState::SHUFFLE
            }
            _ => {
                imp.none_button.set_active(true);
                LoopsState::NONE
            }
        };
        if let Ok(mut playlist) = imp.playlist.lock() {
            playlist.set_loops(loop_state);
        }
    }

    pub fn setup_mpris(&self) {
        let imp = self.imp();
        imp.mpris.set(MprisController::new()).unwrap();
        let mpris = imp.mpris.get().unwrap();
        mpris.setup_signals(self);
        let settings = self.settings();
        let loop_state = settings.string("repeat-variant");
        match loop_state.as_str() {
            "none" => {
                mpris.set_loop_status(LoopsState::NONE);
            }
            "one" => {
                mpris.set_loop_status(LoopsState::ONE);
            }
            "loop" => {
                mpris.set_loop_status(LoopsState::LOOP);
            }
            "shuffle" => {
                mpris.set_loop_status(LoopsState::SHUFFLE);
            }
            _ => {
                mpris.set_loop_status(LoopsState::NONE);
            }
        };
    }

    pub fn setup_player(&self) {
        let imp = self.imp();
        let dispatcher = PlayerGMainContextSignalDispatcher::new(None);
        let player = Player::new(None, Some(&dispatcher.upcast::<PlayerSignalDispatcher>()));

        let mut config = player.config();
        config.set_user_agent(
            "User-Agent: Mozilla/5.0 (X11; Linux x86_64; rv:100.0) Gecko/20100101 Firefox/100.0",
        );
        config.set_position_update_interval(250);
        config.set_seek_accurate(true);
        player.set_config(config).unwrap();
        player.set_volume(0.0);

        imp.player.set(player).unwrap();
    }

    pub fn play(&self, song_info: SongInfo) {
        let imp = self.imp();

        let sender = imp.sender.get().unwrap();
        sender
            .send(Action::AddToast(gettext!(
                "Start playback [{}] ...",
                song_info.name
            )))
            .unwrap();

        let player = imp.player.get().unwrap();
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
        path_cover.push(format!("{}-cover.jpg", song_info.album_id));
        if path_cover.exists() {
            cover_image.set_from_file(Some(&path_cover));
        } else {
            cover_image.set_icon_name(Some("logo-symbolic"));
        }

        let title_label = imp.title_label.get();
        title_label.set_label(&song_info.name);
        title_label.set_tooltip_text(Some(&song_info.name));

        let artist_label = imp.artist_label.get();
        artist_label.set_label(&song_info.singer);

        let mpris = imp.mpris.get().unwrap();
        mpris.update_metadata(&song_info);
        mpris.set_playback_status(PlaybackStatus::Playing);
        mpris.set_volume(self.property("volume"));
    }

    pub fn connect_gst_signals(&self) {
        let imp = self.imp();
        let player = imp.player.get().unwrap();

        let scale = Rc::new(imp.seek_scale.get());

        let scale_clone = Fragile::new(Rc::clone(&scale));
        let progress_time_label = Fragile::new(Rc::new(imp.progress_time_label.get()));
        player.connect_position_updated(move |_, clock| {
            if let Some(clock) = clock {
                let scale = scale_clone.get();
                let duration = format!("{:0>2}:{:0>2}", clock.seconds() / 60, clock.seconds() % 60);
                scale.set_value(clock.useconds() as f64);
                progress_time_label.get().set_label(&duration);
            }
        });

        let scale_clone = Fragile::new(Rc::clone(&scale));
        let duration_label = Fragile::new(Rc::new(imp.duration_label.get()));
        player.connect_duration_changed(move |_, clock| {
            if let Some(clock) = clock {
                let scale = scale_clone.get();
                let duration = format!("{:0>2}:{:0>2}", clock.seconds() / 60, clock.seconds() % 60);
                scale.set_range(0.0, clock.useconds() as f64);
                duration_label.get().set_label(&duration);
            }
        });

        let sender = imp.sender.get().unwrap().clone();
        player.connect_end_of_stream(move |_| {
            sender.send(Action::PlayNextSong).unwrap();
        });

        let sender = imp.sender.get().unwrap().clone();
        player.connect_error(move |_, e| {
            sender
                .send(Action::AddToast(gettext!(
                    "Playback error:{}",
                    e.to_string()
                )))
                .unwrap();
            sender.send(Action::PlayNextSong).unwrap();
        });

        let play_button = Fragile::new(Rc::new(imp.play_button.get()));
        player.connect_state_changed(move |_, state| {
            let play_button = play_button.get();
            match state {
                PlayerState::Stopped => play_button.set_icon_name("media-playback-start-symbolic"),
                PlayerState::Paused => play_button.set_icon_name("media-playback-start-symbolic"),
                PlayerState::Playing => play_button.set_icon_name("media-playback-pause-symbolic"),
                _ => (),
            }
        });
    }

    pub fn bind_shortcut(&self) {
        // 播放按钮
        let play_button = self.imp().play_button.get();
        let controller = ShortcutController::new();
        let trigger = ShortcutTrigger::parse_string("<primary>space").unwrap();
        let action = ActivateAction::get();
        let shortcut = Shortcut::new(Some(&trigger), Some(&action));
        controller.add_shortcut(&shortcut);
        controller.set_scope(ShortcutScope::Global);
        play_button.add_controller(&controller);

        // 上一曲按钮
        let prev_button = self.imp().prev_button.get();
        let controller = ShortcutController::new();
        let trigger = ShortcutTrigger::parse_string("<primary>b").unwrap();
        let action = ActivateAction::get();
        let shortcut = Shortcut::new(Some(&trigger), Some(&action));
        controller.add_shortcut(&shortcut);
        controller.set_scope(ShortcutScope::Global);
        prev_button.add_controller(&controller);

        // 下一曲按钮
        let next_button = self.imp().next_button.get();
        let controller = ShortcutController::new();
        let trigger = ShortcutTrigger::parse_string("<primary>n").unwrap();
        let action = ActivateAction::get();
        let shortcut = Shortcut::new(Some(&trigger), Some(&action));
        controller.add_shortcut(&shortcut);
        controller.set_scope(ShortcutScope::Global);
        next_button.add_controller(&controller);
    }

    pub fn add_song(&self, song: SongInfo) {
        if let Ok(mut playlist) = self.imp().playlist.lock() {
            playlist.add_song(song);
        }
    }

    pub fn add_list(&self, list: Vec<SongInfo>) {
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
        let mpris = imp.mpris.get().unwrap();

        mpris.set_playback_status(PlaybackStatus::Playing);
        player.play();
    }

    pub fn switch_pause(&self) {
        let imp = self.imp();
        let player = imp.player.get().unwrap();
        let mpris = imp.mpris.get().unwrap();

        mpris.set_playback_status(PlaybackStatus::Paused);
        player.pause();
    }

    pub fn switch_stop(&self) {
        let imp = self.imp();
        let player = imp.player.get().unwrap();
        let mpris = imp.mpris.get().unwrap();

        mpris.set_playback_status(PlaybackStatus::Stopped);
        player.stop();
    }

    // 从 Mpris2 设置播放循环
    pub fn set_loops(&self, loops_status: LoopStatus) {
        let imp = self.imp();
        match loops_status {
            LoopStatus::None => {
                imp.none_button.set_active(true);
            }
            LoopStatus::Track => {
                imp.one_button.set_active(true);
            }
            LoopStatus::Playlist => {
                imp.loop_button.set_active(true);
            }
        }
    }

    // 从 Mpris2 设置混淆播放
    pub fn set_shuffle(&self, shuffle: bool) {
        let imp = self.imp();
        if shuffle {
            imp.shuffle_button.set_active(true);
        } else if let Some(status) = imp.mpris.get().unwrap().get_loop_status() {
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

    pub fn set_volume(&self, value: f64) {
        let player = self.imp().player.get().unwrap();
        player.set_volume(value);

        self.set_property("volume", value);

        let volume_button = self.imp().volume_button.get();
        volume_button.set_value(value);
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
        let player = self.imp().player.get().unwrap();
        let mpris = self.imp().mpris.get().unwrap();
        player.set_volume(adj.value());
        if self.property::<f64>("volume") != adj.value() {
            mpris.set_volume(adj.value());
        }
        self.set_property("volume", adj.value());
    }

    #[template_callback]
    fn cover_clicked_cb(&self) {
        let sender = self.imp().sender.get().unwrap().clone();
        if let Some(songinfo) = self.get_current_song() {
            let mut path = CACHE.clone();
            path.push(format!("{}-songlist.jpg", songinfo.album_id));
            if sender
                .send(Action::DownloadImage(
                    songinfo.pic_url.to_owned(),
                    path.to_owned(),
                    140,
                    140,
                ))
                .is_ok()
            {
                let songlist = SongList {
                    id: songinfo.album_id,
                    name: songinfo.album,
                    cover_img_url: songinfo.pic_url,
                };
                let path = path.to_owned();
                glib::timeout_add_local(Duration::from_millis(100), move || {
                    if path.exists() {
                        sender
                            .send(Action::ToAlbumPage(songlist.to_owned()))
                            .unwrap();
                        return Continue(false);
                    }
                    Continue(true)
                });
            }
        }
    }
}

mod imp {

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

        pub settings: OnceCell<Settings>,
        pub sender: OnceCell<Sender<Action>>,
        pub player: OnceCell<Player>,
        pub playlist: Arc<Mutex<PlayList>>,
        pub mpris: OnceCell<MprisController>,
        pub volume: Cell<f64>,
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
                    sender.send(Action::Play(song_info.to_owned())).unwrap();
                    sender
                        .send(Action::GetLyrics(song_info.to_owned()))
                        .unwrap();
                    sender
                        .send(Action::UpdatePlayListStatus(playlist.get_position()))
                        .unwrap();
                    return;
                }
            }
            sender
                .send(Action::AddToast(gettext("No more songs！")))
                .unwrap();
        }

        #[template_callback]
        fn play_button_clicked_cb(&self, button: Button) {
            let player = self.player.get().unwrap();
            let mpris = self.mpris.get().unwrap();
            if button
                .icon_name()
                .unwrap()
                .eq("media-playback-start-symbolic")
            {
                player.play();
                mpris.set_playback_status(PlaybackStatus::Playing);
                button.set_icon_name("media-playback-pause-symbolic");
            } else {
                player.pause();
                mpris.set_playback_status(PlaybackStatus::Paused);
                button.set_icon_name("media-playback-start-symbolic");
            }
        }

        #[template_callback]
        fn next_button_clicked_cb(&self) {
            let sender = self.sender.get().unwrap().clone();
            if let Ok(mut playlist) = self.playlist.lock() {
                if let Some(song_info) = playlist.next_song() {
                    sender.send(Action::Play(song_info.to_owned())).unwrap();
                    sender
                        .send(Action::GetLyrics(song_info.to_owned()))
                        .unwrap();
                    sender
                        .send(Action::UpdatePlayListStatus(playlist.get_position()))
                        .unwrap();
                    return;
                }
            }
            sender
                .send(Action::AddToast(gettext("No more songs！")))
                .unwrap();
        }

        #[template_callback]
        fn seek_scale_cb(&self, _: ScrollType, value: f64) -> Inhibit {
            let player = self.player.get().unwrap();
            player.seek(ClockTime::from_useconds(value as u64));

            let mpris = self.mpris.get().unwrap();
            mpris.set_position(value as i64);
            mpris.seek(value as i64);
            Inhibit(false)
        }

        #[template_callback]
        fn like_button_cb(&self) {
            let sender = self.sender.get().unwrap().clone();
            if COOKIE_JAR.get().is_none() {
                sender
                    .send(Action::AddToast(gettext("Please login first！")))
                    .unwrap();
                return;
            }
            if let Ok(playlist) = self.playlist.lock() {
                if let Some(song_info) = playlist.current_song() {
                    sender.send(Action::LikeSong(song_info.id)).unwrap();
                    return;
                }
            }
            sender
                .send(Action::AddToast(gettext("Collection failure！")))
                .unwrap();
        }

        #[template_callback]
        fn repeat_none_cb(&self) {
            self.repeat_image
                .set_icon_name(Some("media-playlist-consecutive-symbolic"));
            self.settings
                .get()
                .unwrap()
                .set_string("repeat-variant", "none")
                .unwrap();
            if let Ok(mut playlist) = self.playlist.lock() {
                playlist.set_loops(LoopsState::NONE);
            }
        }

        #[template_callback]
        fn repeat_one_cb(&self) {
            self.repeat_image
                .set_icon_name(Some("media-playlist-repeat-song-symbolic"));
            self.settings
                .get()
                .unwrap()
                .set_string("repeat-variant", "one")
                .unwrap();
            if let Ok(mut playlist) = self.playlist.lock() {
                playlist.set_loops(LoopsState::ONE);
            }
        }

        #[template_callback]
        fn repeat_loop_cb(&self) {
            self.repeat_image
                .set_icon_name(Some("media-playlist-repeat-symbolic"));
            self.settings
                .get()
                .unwrap()
                .set_string("repeat-variant", "loop")
                .unwrap();
            if let Ok(mut playlist) = self.playlist.lock() {
                playlist.set_loops(LoopsState::LOOP);
            }
        }

        #[template_callback]
        fn repeat_shuffle_cb(&self) {
            self.repeat_image
                .set_icon_name(Some("media-playlist-shuffle-symbolic"));
            self.settings
                .get()
                .unwrap()
                .set_string("repeat-variant", "shuffle")
                .unwrap();
            if let Ok(mut playlist) = self.playlist.lock() {
                playlist.set_loops(LoopsState::SHUFFLE);
            }
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
                        duration: String::new(),
                        song_url: String::new(),
                    })
                    .to_owned();
                let sender = self.sender.get().unwrap().clone();
                sender
                    .send(Action::ToPlayListLyricsPage(
                        playlist.get_list(),
                        current_song,
                    ))
                    .unwrap();
            }
        }
    }

    impl ObjectImpl for PlayerControls {
        fn constructed(&self, obj: &Self::Type) {
            self.parent_constructed(obj);
            *self.playlist.lock().unwrap() = PlayList::new();

            obj.setup_settings();
            obj.load_settings();
            obj.setup_player();
            obj.setup_mpris();
            obj.bind_shortcut();
        }

        fn properties() -> &'static [ParamSpec] {
            static PROPERTIES: Lazy<Vec<ParamSpec>> = Lazy::new(|| {
                vec![ParamSpecDouble::new(
                    // Name
                    "volume",
                    // Nickname
                    "volume",
                    // Short description
                    "volume",
                    // Minimum value
                    f64::MIN,
                    // Maximum value
                    f64::MAX,
                    // Default value
                    0.0,
                    // The property can be read and written to
                    ParamFlags::READWRITE,
                )]
            });
            PROPERTIES.as_ref()
        }

        fn set_property(&self, _obj: &Self::Type, _id: usize, value: &Value, pspec: &ParamSpec) {
            match pspec.name() {
                "volume" => {
                    let input_number = value.get().expect("The value needs to be of type `f64`.");
                    self.volume.replace(input_number);
                }
                _ => unimplemented!(),
            }
        }

        fn property(&self, _obj: &Self::Type, _id: usize, pspec: &ParamSpec) -> Value {
            match pspec.name() {
                "volume" => self.volume.get().to_value(),
                _ => unimplemented!(),
            }
        }
    }
    impl WidgetImpl for PlayerControls {}
    impl BoxImpl for PlayerControls {}
}
