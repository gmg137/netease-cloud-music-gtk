//
// app.rs
// Copyright (C) 2019 gmg137 <gmg137@live.com>
// Distributed under terms of the GPLv3 license.
//

use crate::musicapi::model::{LoginInfo, SongInfo, SongList};
use crate::utils::*;
use crate::view::*;
use crate::widgets::{header::*, mark_all_notif, notice::*, player::*};
use async_std::task;
use crossbeam_channel::{unbounded, Receiver, Sender, TryRecvError};
use gio::{self, prelude::*};
use glib;
use gtk::prelude::*;
use gtk::{ApplicationWindow, Builder, Overlay};
use std::cell::RefCell;
use std::env;
use std::rc::Rc;

#[derive(Debug, Clone)]
pub(crate) enum Action {
    SwitchStackMain,
    SwitchStackSub((u32, String, String)),
    SwitchHeaderBar(String),
    RefreshHeaderUser,
    RefreshHeaderUserLogin(LoginInfo),
    RefreshHeaderUserLogout,
    RefreshHome,
    RefreshHomeView(Vec<SongList>, Vec<SongList>),
    RefreshSubUpView(u32, String, String),
    RefreshSubLowView(Vec<SongInfo>),
    ShowSubLike(bool),
    LikeSongList,
    DisLikeSongList,
    RefreshFoundViewInit(u8),
    RefreshFoundView(Vec<SongInfo>, String),
    RefreshMine,
    MineHideAll,
    MineShowFm,
    RefreshMineViewInit(i32),
    RefreshMineCurrentView(),
    RefreshMineLikeList(),
    RefreshMineView(Vec<SongInfo>, String),
    RefreshMineFm(SongInfo),
    RefreshMineSidebar(Vec<SongList>),
    PlayerFm,
    FmLike,
    FmDislike,
    RefreshMineFmPlayerList,
    CancelCollection,
    Search(String),
    PlayerInit(SongInfo, PlayerTypes),
    PlayerTypes(PlayerTypes),
    ReadyPlayer(SongInfo),
    Player(SongInfo),
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
}

#[derive(Clone)]
pub(crate) struct App {
    window: gtk::ApplicationWindow,
    view: Rc<View>,
    header: Rc<Header>,
    player: PlayerWrapper,
    notice: RefCell<Option<InAppNotification>>,
    overlay: Overlay,
    configs: Rc<RefCell<Configs>>,
    sender: Sender<Action>,
    receiver: Receiver<Action>,
}

impl App {
    pub(crate) fn new(application: &gtk::Application) -> Rc<Self> {
        let (sender, receiver) = unbounded();

        let glade_src = include_str!("../ui/window.ui");
        let builder = Builder::new_from_string(glade_src);

        let window: ApplicationWindow = builder.get_object("applicationwindow").expect("Couldn't get window");
        window.set_application(Some(application));
        window.set_title("网易云音乐");

        let configs = task::block_on(async { get_config().await }).unwrap();
        let view = View::new(&builder, &sender);
        let header = Header::new(&builder, &sender, &configs);
        let player = PlayerWrapper::new(&builder, &sender);

        window.show_all();

        let tray = configs.tray.clone();
        let weak_app = application.downgrade();
        window.connect_delete_event(move |w, _| {
            if !tray {
                let app = match weak_app.upgrade() {
                    Some(a) => a,
                    None => return Inhibit(false),
                };

                info!("Application is exiting");
                app.quit();
                return Inhibit(false);
            } else {
                w.hide_on_delete();
                return Inhibit(true);
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
        };
        Rc::new(app)
    }

    fn init(app: &Rc<Self>) {
        // Setup the Action channel
        gtk::timeout_add(25, crate::clone!(app => move || app.setup_action_channel()));
    }

    fn setup_action_channel(&self) -> glib::Continue {
        let action = match self.receiver.try_recv() {
            Ok(a) => a,
            Err(TryRecvError::Empty) => return glib::Continue(true),
            Err(TryRecvError::Disconnected) => unreachable!("How the hell was the action channel dropped."),
        };

        trace!("Incoming channel action: {:?}", action);
        match action {
            Action::SwitchHeaderBar(title) => self.header.switch_header(title),
            Action::RefreshHeaderUser => self.header.update_user_button(),
            Action::RefreshHeaderUserLogin(login_info) => self.header.update_user_login(login_info),
            Action::RefreshHeaderUserLogout => self.header.update_user_logout(),
            Action::RefreshHome => self.view.update_home(),
            Action::RefreshHomeView(tsl, rr) => self.view.update_home_view(tsl, rr),
            Action::RefreshSubUpView(id, name, image_path) => self.view.update_sub_up_view(id, name, image_path),
            Action::RefreshSubLowView(song_list) => self.view.update_sub_low_view(song_list),
            Action::ShowSubLike(show) => self.view.show_sub_like_button(show),
            Action::SwitchStackMain => self.view.switch_stack_main(),
            Action::SwitchStackSub((id, name, image_path)) => self.view.switch_stack_sub(id, name, image_path),
            Action::LikeSongList => self.view.sub_like_song_list(),
            Action::DisLikeSongList => self.view.dis_like_song_list(),
            Action::RefreshFoundViewInit(id) => self.view.update_found_view_data(id),
            Action::RefreshFoundView(song_list, title) => self.view.update_found_view(song_list, title),
            Action::RefreshMine => self.view.mine_init(),
            Action::MineHideAll => self.view.mine_hide_all(),
            Action::MineShowFm => self.view.mine_show_fm(),
            Action::RefreshMineViewInit(id) => self.view.update_mine_view_data(id, false),
            Action::RefreshMineCurrentView() => self.view.update_mine_current_view_data(),
            Action::RefreshMineLikeList() => self.view.update_like_song_list(),
            Action::RefreshMineView(song_list, title) => self.view.update_mine_view(song_list, title),
            Action::RefreshMineFm(si) => self.view.update_mine_fm(si),
            Action::RefreshMineSidebar(vsl) => self.view.update_mine_sidebar(vsl),
            Action::RefreshMineFmPlayerList => {
                self.view.refresh_fm_player_list();
            }
            Action::PlayerFm => self.view.play_fm(),
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
            Action::PlayerInit(info, pt) => self.player.initialize_player(info, pt),
            Action::PlayerTypes(pt) => self.player.set_player_typers(pt),
            Action::Player(info) => self.player.player(info),
            Action::ReadyPlayer(info) => self.player.ready_player(info, self.configs.borrow().lyrics),
            Action::ShowNotice(text) => {
                let notif = mark_all_notif(text);
                let old = self.notice.replace(Some(notif));
                old.map(|i| i.destroy());
                self.notice.borrow().as_ref().map(|i| i.show(&self.overlay));
            }
            Action::PlayerSubpages => self.view.play_subpages(),
            Action::PlayerFound => self.view.play_found(),
            Action::PlayerMine => self.view.play_mine(),
            Action::QuitMain => self.window.destroy(),
            Action::ConfigsSetTray(state) => {
                task::spawn(async move {
                    if let Ok(mut conf) = get_config().await {
                        conf.tray = state;
                        save_config(&conf).await.ok();
                    }
                });
            }
            Action::ConfigsSetLyrics(state) => {
                task::spawn(async move {
                    if let Ok(mut conf) = get_config().await {
                        conf.lyrics = state;
                        save_config(&conf).await.ok();
                    }
                });
            }
        }

        glib::Continue(true)
    }

    pub(crate) fn run() {
        let application = gtk::Application::new(
            Some("com.github.gmg137.netease-cloud-music-gtk"),
            gio::ApplicationFlags::empty(),
        )
        .expect("Application initialization failed...");

        let weak_app = application.downgrade();
        application.connect_startup(move |_| {
            weak_app.upgrade().map(|application| {
                let app = Self::new(&application);
                Self::init(&app);

                let weak = Rc::downgrade(&app);
                application.connect_activate(move |_| {
                    info!("GApplication::activate");
                    if let Some(app) = weak.upgrade() {
                        // Ideally Gtk4/GtkBuilder make this irrelvent
                        app.window.show_all();
                        app.window.present();
                        info!("Window presented");
                    } else {
                        debug_assert!(false, "I hate computers");
                    }
                });

                info!("Init complete");
            });
        });

        glib::set_application_name("netease-cloud-music-gtk");
        glib::set_prgname(Some("netease-cloud-music-gtk"));
        gtk::Window::set_default_icon_name("netease-cloud-music-gtk");
        let args: Vec<String> = env::args().collect();
        ApplicationExtManual::run(&application, &args);
    }
}
