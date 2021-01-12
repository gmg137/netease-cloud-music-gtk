//
// app.rs
// Copyright (C) 2019 gmg137 <gmg137@live.com>
// Distributed under terms of the GPLv3 license.
//

use crate::{
    data::MusicData,
    model::{DATE_DAY, DATE_MONTH, ISO_WEEK},
    musicapi::model::{LoginInfo, Parse, SongInfo, SongList},
    task::{actuator_loop, Task},
    utils::*,
    view::*,
    widgets::{header::*, mark_all_notif, notice::*, player::*},
};
use async_std::{
    sync::{Arc, Mutex},
    task,
};
use futures::channel::mpsc;
use gio::{self, prelude::*};
use glib::{Receiver, Sender};
use gtk::{prelude::*, AccelGroup, ApplicationWindow, Builder, Overlay};
use std::{cell::RefCell, rc::Rc};

pub(crate) enum Action {
    SwitchStackMain,
    SwitchStackSubSpinner,
    SwitchStackMineSpinner,
    SwitchStackFoundSpinner,
    SwitchStackSub((u64, String, String), Parse),
    SwitchHeaderBar(String),
    RefreshHeaderUser,
    RefreshHeaderUserLogin(LoginInfo),
    RefreshHeaderUserLogout,
    RefreshHome,
    RefreshHomeView(Arc<Vec<SongList>>, Arc<Vec<SongList>>),
    RefreshHomeUpImage(i32, i32, SongList),
    RefreshHomeLowImage(i32, i32, SongList),
    RefreshSubUpView(u64, String, String),
    RefreshSubLowView(Vec<SongInfo>),
    ShowSubLike(bool),
    LikeSong(u64),
    LikeSongList,
    DisLikeSongList,
    RefreshFoundViewInit(u8),
    RefreshFoundView(Vec<SongInfo>, String),
    RefreshMine,
    MineShowLogin,
    MineShowNotLogin,
    MineShowFm,
    RefreshMineViewInit(i32),
    RefreshMineCurrentView(),
    RefreshMineLikeList(),
    RefreshMineView(Vec<SongInfo>, String),
    RefreshMineFm(SongInfo),
    RefreshMineRecommendView(Arc<Vec<SongList>>),
    RefreshMineSidebar(Vec<SongList>),
    RefreshMineRecommendImage(i32, i32, SongList),
    PlayerFm,
    PauseFm,
    FmLike,
    FmDislike,
    RefreshMineFmPlayerList,
    RefreshMineFmPlay,
    RefreshMineFmPause,
    RefreshMineFmImage(String),
    CancelCollection,
    Search(String),
    PlayerInit(SongInfo, PlayerTypes),
    PlayerTypes(PlayerTypes),
    ReadyPlayer(SongInfo),
    RefreshLyricsText(String),
    RefreshPlaylist(PlayerListData),
    PlaylistSong(i32),
    Player(SongInfo),
    PlayerOne,
    RefreshPlayerImage(String),
    PlayerSubpages,
    PlayerFound,
    PlayerMine,
    Login(String, String),
    Logout,
    ShowNotice(String),
    DailyTask,
    QuitMain,
    ConfigsSetTray(bool),
    ConfigsSetLyrics(bool),
    ConfigsSetClear(u8),
    ActivateApp,
    PlayAddAccel,
    PlayRemoveAccel,
    BackEvent,
}

pub(crate) struct App {
    window: gtk::ApplicationWindow,
    view: Rc<View>,
    header: Rc<Header>,
    player: PlayerWrapper,
    notice: RefCell<Option<InAppNotification>>,
    overlay: Overlay,
    configs: Rc<RefCell<Configs>>,
    sender: Sender<Action>,
    receiver: RefCell<Option<Receiver<Action>>>,
    music_data: Arc<Mutex<MusicData>>,
    accel_group: AccelGroup,
}

impl App {
    pub(crate) fn new(application: &gtk::Application) -> Rc<Self> {
        let (sender, r) = glib::MainContext::channel(glib::PRIORITY_DEFAULT);
        let receiver = RefCell::new(Some(r));

        let glade_src = include_str!("../ui/window.ui");
        let builder = Builder::from_string(glade_src);

        let window: ApplicationWindow = builder.get_object("applicationwindow").expect("Couldn't get window");
        window.set_application(Some(application));
        window.set_title("网易云音乐");

        let accel_group = AccelGroup::new();
        window.add_accel_group(&accel_group);

        let configs = task::block_on(get_config()).unwrap();

        let (sender_task, receiver_task) = mpsc::channel::<Task>(10);
        let sender_clone = sender.clone();
        task::spawn(async move {
            actuator_loop(receiver_task, sender_clone).await.ok();
        });

        // 捕获鼠标返回键
        let sender_clone = sender.clone();
        window.connect_button_press_event(move |_, event| {
            if event.get_button() == 8 {
                sender_clone.send(Action::BackEvent).unwrap_or(());
            }
            gtk::Inhibit(false)
        });

        let music_data = Arc::new(Mutex::new(task::block_on(MusicData::new())));
        let data = music_data.clone();
        task::block_on(async move {
            let mut data = data.lock().await;
            if data.login {
                data.re_login().await.ok();
            }
        });
        let header = Header::new(&builder, &sender, &configs, Arc::clone(&music_data));
        let view = View::new(&builder, &sender, &sender_task, Arc::clone(&music_data));
        let player = PlayerWrapper::new(&builder, &sender, &sender_task, Arc::clone(&music_data));
        player.play_add_accel(&accel_group);

        window.show_all();

        let weak_app = application.downgrade();
        window.connect_delete_event(move |w, _| {
            let tray = task::block_on(async {
                if let Ok(conf) = get_config().await {
                    conf.tray
                } else {
                    false
                }
            });
            if !tray {
                let app = match weak_app.upgrade() {
                    Some(a) => a,
                    None => return Inhibit(false),
                };

                info!("Application is exiting");
                app.quit();
                Inhibit(false)
            } else {
                w.hide_on_delete();
                Inhibit(true)
            }
        });

        let overlay: Overlay = builder.get_object("overlay").unwrap();

        let notice = RefCell::new(None);

        let app = App {
            window,
            header,
            view,
            player,
            notice,
            overlay,
            configs: Rc::new(RefCell::new(configs)),
            sender,
            receiver,
            music_data,
            accel_group,
        };
        Rc::new(app)
    }

    fn init(app: &Rc<Self>) {
        let app = Rc::clone(app);
        let receiver = app.receiver.borrow_mut().take().unwrap();
        receiver.attach(None, move |action| app.do_action(action));
    }

    fn do_action(&self, action: Action) -> glib::Continue {
        match action {
            Action::SwitchHeaderBar(title) => self.header.switch_header(title),
            Action::RefreshHeaderUser => self.header.update_user_button(),
            Action::RefreshHeaderUserLogin(login_info) => self.header.update_user_login(login_info),
            Action::RefreshHeaderUserLogout => self.header.update_user_logout(),
            Action::RefreshHome => self.view.update_home(),
            Action::RefreshHomeView(tsl, rr) => self.view.update_home_view(tsl, rr),
            Action::RefreshHomeUpImage(left, top, sl) => self.view.set_home_up_image(left, top, sl),
            Action::RefreshHomeLowImage(left, top, sl) => self.view.set_home_low_image(left, top, sl),
            Action::RefreshSubUpView(id, name, image_path) => self.view.update_sub_up_view(id, name, image_path),
            Action::RefreshSubLowView(song_list) => self.view.update_sub_low_view(song_list),
            Action::ShowSubLike(show) => self.view.show_sub_like_button(show),
            Action::SwitchStackMain => self.view.switch_stack_main(),
            Action::SwitchStackSubSpinner => self.view.switch_stack_subpages(),
            Action::SwitchStackMineSpinner => self.view.mine_login_switch_list(),
            Action::SwitchStackFoundSpinner => self.view.found_content_switch_stack_right(),
            Action::SwitchStackSub((id, name, image_path), parse) => {
                self.view.switch_stack_sub(id, name, image_path, parse)
            }
            Action::LikeSong(song_id) => {
                let data = self.music_data.clone();
                let sender = self.sender.clone();
                task::spawn(async move {
                    let mut data = data.lock().await;
                    if data.like(true, song_id).await {
                        sender.send(Action::ShowNotice("已添加到喜欢!".to_owned())).unwrap();
                        sender.send(Action::RefreshMineLikeList()).unwrap();
                    } else {
                        sender.send(Action::ShowNotice("收藏失败!".to_owned())).unwrap();
                    }
                });
            }
            Action::LikeSongList => self.view.sub_like_song_list(),
            Action::DisLikeSongList => self.view.dis_like_song_list(),
            Action::RefreshFoundViewInit(id) => self.view.update_found_view_data(id),
            Action::RefreshFoundView(song_list, title) => self.view.update_found_view(song_list, title),
            Action::RefreshMine => self.view.mine_init(),
            Action::MineShowLogin => self.view.mine_switch_login(),
            Action::MineShowNotLogin => self.view.mine_switch_not_login(),
            Action::MineShowFm => self.view.mine_login_switch_fm(),
            Action::RefreshMineViewInit(id) => self.view.update_mine_view_data(id, false),
            Action::RefreshMineCurrentView() => self.view.update_mine_current_view_data(),
            Action::RefreshMineLikeList() => self.view.update_like_song_list(),
            Action::RefreshMineView(song_list, title) => self.view.update_mine_view(song_list, title),
            Action::RefreshMineFm(si) => self.view.update_mine_fm(si),
            Action::RefreshMineSidebar(vsl) => self.view.update_mine_sidebar(vsl),
            Action::RefreshMineRecommendView(rr) => self.view.update_mine_recommend(rr),
            Action::RefreshMineRecommendImage(l, t, s) => self.view.refresh_mine_recommend_image(l, t, s),
            Action::RefreshMineFmPlayerList => {
                self.view.refresh_fm_player_list();
            }
            Action::RefreshMineFmPlay => self.view.switch_fm_play(),
            Action::RefreshMineFmPause => self.view.switch_fm_pause(),
            Action::RefreshMineFmImage(path) => self.view.set_fm_image(path),
            Action::PlayerFm => self.view.play_fm(),
            Action::PauseFm => self.player.pause(),
            Action::FmLike => self.view.like_fm(),
            Action::FmDislike => {
                self.player.forward();
                self.view.dislike_fm();
            }
            Action::CancelCollection => self.view.cancel_collection(),
            Action::Search(text) => self.view.switch_stack_search(text),
            Action::Login(name, pass) => self.header.login(name, pass),
            Action::Logout => self.header.logout(),
            Action::DailyTask => self.header.daily_task(),
            Action::PlayerInit(info, pt) => self.player.initialize_player(info, pt, self.configs.borrow().lyrics),
            Action::PlayerTypes(pt) => self.player.set_player_typers(pt),
            Action::PlaylistSong(index) => self.player.playlist_song(index),
            Action::Player(info) => self.player.player(info),
            Action::ReadyPlayer(info) => self.player.ready_player(info, self.configs.borrow().lyrics),
            Action::RefreshLyricsText(lrc) => self.player.update_lyrics_text(lrc),
            Action::RefreshPlaylist(pl) => self.player.update_playlist(pl),
            Action::ShowNotice(text) => {
                let notif = mark_all_notif(text);
                let old = self.notice.replace(Some(notif));
                if let Some(i) = old {
                    i.destroy()
                }
                if let Some(i) = self.notice.borrow().as_ref() {
                    i.show(&self.overlay)
                }
            }
            Action::PlayerOne => self.player.play_one(),
            Action::RefreshPlayerImage(path) => self.player.set_cover_image(path),
            Action::PlayerSubpages => self.view.play_subpages(),
            Action::PlayerFound => self.view.play_found(),
            Action::PlayerMine => self.view.play_mine(),
            Action::QuitMain => unsafe { self.window.destroy() },
            Action::ConfigsSetTray(state) => {
                task::spawn(async move {
                    if let Ok(mut conf) = get_config().await {
                        conf.tray = state;
                        save_config(&conf).await.ok();
                    }
                });
            }
            Action::ConfigsSetLyrics(state) => {
                self.configs.borrow_mut().lyrics = state;
                task::spawn(async move {
                    if let Ok(mut conf) = get_config().await {
                        conf.lyrics = state;
                        save_config(&conf).await.ok();
                    }
                });
            }
            Action::ConfigsSetClear(id) => {
                task::spawn(async move {
                    if let Ok(mut conf) = get_config().await {
                        match id {
                            0 => {
                                conf.clear = ClearCached::NONE;
                            }
                            1 => {
                                conf.clear = ClearCached::MONTH(*DATE_MONTH);
                            }
                            2 => {
                                conf.clear = ClearCached::WEEK(*ISO_WEEK);
                            }
                            3 => {
                                conf.clear = ClearCached::DAY(*DATE_DAY);
                            }
                            _ => {}
                        }
                        save_config(&conf).await.ok();
                    }
                });
            }
            Action::ActivateApp => {
                self.window.show_now();
                self.window.present();
            }
            Action::PlayAddAccel => {
                self.player.play_add_accel(&self.accel_group);
            }
            Action::PlayRemoveAccel => {
                self.player.play_remove_accel(&self.accel_group);
            }
            Action::BackEvent => {
                self.header.click_back();
            }
        }

        glib::Continue(true)
    }

    pub(crate) fn run() {
        let application = gtk::Application::new(Some("com.github.gmg137.netease-cloud-music-gtk"), Default::default())
            .expect("Application initialization failed...");

        let weak_app = application.downgrade();
        application.connect_startup(move |_| {
            if let Some(application) = weak_app.upgrade() {
                let app = Self::new(&application);
                Self::init(&app);

                let weak = Rc::downgrade(&app);
                application.connect_activate(move |_| {
                    if let Some(app) = weak.upgrade() {
                        app.window.show_now();
                        app.window.present();
                    } else {
                        debug_assert!(false, "I hate computers");
                    }
                });
            };
        });

        glib::set_application_name("netease-cloud-music-gtk");
        glib::set_prgname(Some("netease-cloud-music-gtk"));
        gtk::Window::set_default_icon_name("netease-cloud-music-gtk");
        ApplicationExtManual::run(&application, &[]);
    }
}
