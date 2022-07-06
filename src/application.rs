use adw::subclass::prelude::*;
use gettextrs::gettext;
use gio::Settings;
use glib::{clone, timeout_future_seconds, MainContext, Receiver, Sender, WeakRef};
use gtk::{gio, glib, prelude::*, subclass::prelude::*};
use ncm_api::{BannersInfo, LoginInfo, SingerInfo, SongInfo, SongList, TopList};
use once_cell::sync::OnceCell;
use std::cell::RefCell;
use std::fs;
use std::path::PathBuf;

use crate::{
    config::VERSION, gui::NeteaseCloudMusicGtk4Preferences, model::*, ncmapi::*, path::CACHE,
    NeteaseCloudMusicGtk4Window,
};

#[derive(Debug, Clone)]
pub enum Action {
    UpdateQrCode,
    SetQrImage(PathBuf),
    CheckQrTimeout(String),
    CheckQrTimeoutCb(String),
    SetQrImageTimeout,
    SwitchUserMenuToPhone,
    SwitchUserMenuToQr,
    GetCaptcha(String, String),
    CaptchaLogin(String, String, String),
    SwitchUserMenuToUser(LoginInfo, UserMenuChild),
    AddToast(String),
    SetAvatar(PathBuf),
    InitCarousel,
    DownloadBanners(BannersInfo),
    AddCarousel(BannersInfo),
    InitTopPicks,
    InitTopPicksSongList,
    // (url,path,width,height)
    DownloadImage(String, PathBuf, u16, u16),
    SetupTopPicks(Vec<SongList>),
    InitNewAlbums,
    InitAllAlbums,
    SetupNewAlbums(Vec<SongList>),
    AddPlay(SongInfo),
    PlayNextSong,
    Play(SongInfo),
    PlayStart(SongInfo),
    DownloadSong(SongInfo),
    ToSongListPage(SongList),
    InitSongListPage(Vec<SongInfo>),
    ToAlbumPage(SongList),
    InitAlbumPage(Vec<SongInfo>),
    AddPlayList(Vec<SongInfo>),
    PlayListStart,
    LikeSongList(u64),
    LikeAlbum(u64),
    LikeSong(u64),
    GetToplist,
    GetToplistSongsList(u64),
    InitTopList(Vec<TopList>),
    UpdateTopList(Vec<SongInfo>),
    // (关键字，搜索类型，起始点，数量)
    Search(String, SearchType, u16, u16),
    UpdateSearchSongPage(Vec<SongInfo>),
    UpdateSearchSongListPage(Vec<SongList>),
    UpdateSearchSingerPage(Vec<SingerInfo>),
    ToSingerSongsPage(SingerInfo),
    ToMyPageDailyRec,
    ToMyPageHeartbeat,
    ToMyPageFm,
    ToMyPageCloudDisk,
    ToMyPageAlbums,
    ToMyPageSonglist,
    InitMyPage,
    InitMyPageRecSongList(Vec<SongList>),
    ToPlayListLyricsPage(Vec<SongInfo>, SongInfo),
    UpdateLyrics(String),
    UpdatePlayListStatus(usize),
    GetLyrics(SongInfo),
}

mod imp {

    use std::sync::{Arc, RwLock};

    use super::*;

    pub struct NeteaseCloudMusicGtk4Application {
        pub window: OnceCell<WeakRef<NeteaseCloudMusicGtk4Window>>,
        pub sender: Sender<Action>,
        pub receiver: RefCell<Option<Receiver<Action>>>,
        pub unikey: Arc<RwLock<String>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for NeteaseCloudMusicGtk4Application {
        const NAME: &'static str = "NeteaseCloudMusicGtk4Application";
        type Type = super::NeteaseCloudMusicGtk4Application;
        type ParentType = adw::Application;
        fn new() -> Self {
            let (sender, r) = MainContext::channel(glib::PRIORITY_DEFAULT);
            let receiver = RefCell::new(Some(r));
            let window = OnceCell::new();
            let unikey = Arc::new(RwLock::new(String::new()));

            Self {
                window,
                sender,
                receiver,
                unikey,
            }
        }
    }

    impl ObjectImpl for NeteaseCloudMusicGtk4Application {
        fn constructed(&self, obj: &Self::Type) {
            self.parent_constructed(obj);

            obj.setup_gactions();
            obj.setup_cache_clear();
            obj.set_accels_for_action("app.quit", &["<primary>q"]);
            obj.set_accels_for_action("app.about", &["<primary>a"]);
            obj.set_accels_for_action("win.search-button", &["<primary>f", "slash"]);
            obj.set_accels_for_action("win.back-button", &["<primary>BackSpace"]);
        }
    }

    impl ApplicationImpl for NeteaseCloudMusicGtk4Application {
        // We connect to the activate callback to create a window when the application
        // has been launched. Additionally, this callback notifies us when the user
        // tries to launch a "second instance" of the application. When they try
        // to do that, we'll just present any existing window.
        fn activate(&self, application: &Self::Type) {
            let app = application
                .downcast_ref::<super::NeteaseCloudMusicGtk4Application>()
                .unwrap();

            if let Some(weak_window) = self.window.get() {
                weak_window.upgrade().unwrap().present();
                return;
            }

            let window = app.create_window();
            let _ = self.window.set(window.downgrade());

            // Setup action channel
            let receiver = self.receiver.borrow_mut().take().unwrap();
            receiver.attach(
                None,
                clone!(@strong app => move |action| app.process_action(action)),
            );

            // Ask the window manager/compositor to present the window
            window.present();
        }
    }

    impl GtkApplicationImpl for NeteaseCloudMusicGtk4Application {}
    impl AdwApplicationImpl for NeteaseCloudMusicGtk4Application {}
}

glib::wrapper! {
    pub struct NeteaseCloudMusicGtk4Application(ObjectSubclass<imp::NeteaseCloudMusicGtk4Application>)
        @extends gio::Application, gtk::Application, adw::Application,
        @implements gio::ActionGroup, gio::ActionMap;
}

impl NeteaseCloudMusicGtk4Application {
    pub fn new(application_id: &str, flags: &gio::ApplicationFlags) -> Self {
        glib::Object::new(&[("application-id", &application_id), ("flags", flags)])
            .expect("Failed to create NeteaseCloudMusicGtk4Application")
    }

    fn create_window(&self) -> NeteaseCloudMusicGtk4Window {
        let imp = self.imp();
        let window = NeteaseCloudMusicGtk4Window::new(&self.clone(), imp.sender.clone());

        window.present();
        window
    }

    fn process_action(&self, action: Action) -> glib::Continue {
        let imp = self.imp();
        if self.active_window().is_none() {
            return glib::Continue(true);
        }

        let window = imp.window.get().unwrap().upgrade().unwrap();
        let proxy_address = window.settings().string("proxy-address").to_string();
        let mut ncmapi = if !proxy_address.is_empty() {
            let mut ncmapi = NcmClient::new();
            if ncmapi.set_proxy(proxy_address).is_ok() {
                ncmapi
            } else {
                NcmClient::new()
            }
        } else {
            NcmClient::new()
        };

        match action {
            Action::UpdateQrCode => {
                let sender = imp.sender.clone();
                let ctx = glib::MainContext::default();
                ctx.spawn_local(async move {
                    if let Ok(res) = ncmapi.create_qrcode().await {
                        sender.send(Action::SetQrImage(res.0)).unwrap();
                        sender.send(Action::CheckQrTimeout(res.1)).unwrap();
                    }
                });
            }
            Action::SetQrImage(path) => {
                let window = imp.window.get().unwrap().upgrade().unwrap();
                window.set_user_qrimage(path);
            }
            Action::CheckQrTimeout(unikey) => {
                if let Ok(key) = imp.unikey.read() {
                    if unikey != *key {
                        let sender = imp.sender.clone();
                        sender.send(Action::CheckQrTimeoutCb(unikey)).unwrap();
                    }
                }
            }
            Action::CheckQrTimeoutCb(unikey) => {
                {
                    let mut key = imp.unikey.write().unwrap();
                    *key = unikey.clone();
                }
                let sender = imp.sender.clone();
                let key = imp.unikey.clone();
                let ctx = glib::MainContext::default();
                ctx.spawn_local(async move {
                    let mut send_toast = true;
                    loop {
                        {
                            let key = key.read().unwrap();
                            if *key != unikey {
                                break;
                            }
                        }
                        if let Ok(msg) = ncmapi.client.login_qr_check(unikey.to_owned()).await {
                            match msg.code {
                                // 已过期
                                800 => {
                                    sender.send(Action::SetQrImageTimeout).unwrap();
                                    break;
                                }
                                // 等待扫码
                                801 => (),
                                // 等待确认
                                802 => {
                                    if send_toast {
                                        sender
                                            .send(Action::AddToast(gettext("Have scanned the QR code, waiting for confirmation!")))
                                            .unwrap();
                                        send_toast = false;
                                    }
                                }
                                // 登陆成功
                                803 => {
                                    if let Ok(login_info) = ncmapi.client.login_status().await {
                                        UID.set(login_info.uid).unwrap();
                                        ncmapi.set_cookie_jar();
                                        sender
                                            .send(Action::SwitchUserMenuToUser(
                                                login_info,
                                                UserMenuChild::Qr,
                                            ))
                                            .unwrap();
                                        sender
                                            .send(Action::AddToast(gettext("Login successful!")))
                                            .unwrap();
                                        sender.send(Action::InitMyPage).unwrap();
                                    }
                                    break;
                                }
                                _ => break,
                            }
                        }
                        timeout_future_seconds(1).await;
                    }
                });
            }
            Action::SetQrImageTimeout => {
                let window = imp.window.get().unwrap().upgrade().unwrap();
                window.set_user_qrimage_timeout();
            }
            Action::SwitchUserMenuToPhone => {
                let window = imp.window.get().unwrap().upgrade().unwrap();
                window.switch_user_menu_to_phone();
            }
            Action::SwitchUserMenuToQr => {
                let window = imp.window.get().unwrap().upgrade().unwrap();
                window.switch_user_menu_to_qr();
            }
            Action::GetCaptcha(ctcode, phone) => {
                let sender = imp.sender.clone();
                let ctx = glib::MainContext::default();
                ctx.spawn_local(async move {
                    if ncmapi.client.captcha(ctcode, phone).await.is_ok() {
                        sender
                            .send(Action::AddToast(gettext(
                                "Please pay attention to check the cell phone verification code!",
                            )))
                            .unwrap();
                    } else {
                        sender
                            .send(Action::AddToast(gettext(
                                "Failed to get verification code!",
                            )))
                            .unwrap();
                    }
                });
            }
            Action::CaptchaLogin(ctcode, phone, captcha) => {
                let sender = imp.sender.clone();
                let ctx = glib::MainContext::default();
                ctx.spawn_local(async move {
                    if let Ok(login_info) =
                        ncmapi.client.login_cellphone(ctcode, phone, captcha).await
                    {
                        UID.set(login_info.uid).unwrap();
                        ncmapi.set_cookie_jar();
                        sender
                            .send(Action::SwitchUserMenuToUser(
                                login_info,
                                UserMenuChild::Phone,
                            ))
                            .unwrap();
                        sender
                            .send(Action::AddToast(gettext("Login successful!")))
                            .unwrap();
                        sender.send(Action::InitMyPage).unwrap();
                    } else {
                        sender
                            .send(Action::AddToast(gettext("Login failed!")))
                            .unwrap();
                    }
                });
            }
            Action::SwitchUserMenuToUser(login_info, menu) => {
                let window = imp.window.get().unwrap().upgrade().unwrap();
                window.switch_user_menu_to_user(login_info.clone(), menu);
                let sender = imp.sender.clone();
                let avatar_url = login_info.avatar_url;
                let mut path = CACHE.clone();
                path.push("avatar.jpg");
                let ctx = glib::MainContext::default();
                ctx.spawn_local(async move {
                    if ncmapi
                        .client
                        .download_img(avatar_url, path.clone(), 50, 50)
                        .await
                        .is_ok()
                    {
                        sender.send(Action::SetAvatar(path)).unwrap();
                    }
                });
            }
            Action::AddToast(mes) => {
                let window = imp.window.get().unwrap().upgrade().unwrap();
                window.add_toast(mes);
            }
            Action::SetAvatar(path) => {
                let window = imp.window.get().unwrap().upgrade().unwrap();
                window.set_avatar(path);
            }
            Action::InitCarousel => {
                let sender = imp.sender.clone();
                let ctx = glib::MainContext::default();
                ctx.spawn_local(async move {
                    if let Ok(banners) = ncmapi.client.banners().await {
                        for banner in banners {
                            sender.send(Action::DownloadBanners(banner)).unwrap();
                        }
                    } else {
                        sender.send(Action::InitCarousel).unwrap();
                    }
                });
            }
            Action::DownloadBanners(banner) => {
                let sender = imp.sender.clone();
                let mut path = CACHE.clone();
                path.push(format!("{}-banner.jpg", banner.id));
                let ctx = glib::MainContext::default();
                ctx.spawn_local(async move {
                    if ncmapi
                        .client
                        .download_img(banner.pic.to_owned(), path, 730, 283)
                        .await
                        .is_ok()
                    {
                        sender.send(Action::AddCarousel(banner)).unwrap();
                    }
                });
            }
            Action::AddCarousel(banner) => {
                let window = imp.window.get().unwrap().upgrade().unwrap();
                window.add_carousel(banner);
            }
            Action::InitTopPicks => {
                let sender = imp.sender.clone();
                let ctx = glib::MainContext::default();
                ctx.spawn_local(async move {
                    if let Ok(song_list) = ncmapi.client.top_song_list("全部", "hot", 0, 8).await
                    {
                        for sl in song_list.clone() {
                            let mut path = CACHE.clone();
                            path.push(format!("{}-songlist.jpg", sl.id));
                            sender
                                .send(Action::DownloadImage(sl.cover_img_url, path, 140, 140))
                                .unwrap();
                        }
                        sender.send(Action::SetupTopPicks(song_list)).unwrap();
                    } else {
                        sender.send(Action::InitTopPicks).unwrap();
                    }
                });
            }
            Action::InitTopPicksSongList => {
                let window = imp.window.get().unwrap().upgrade().unwrap();
                window.init_picks_songlist();
                let sender = imp.sender.clone();
                sender
                    .send(Action::Search(String::new(), SearchType::TopPicks, 0, 50))
                    .unwrap();
            }
            Action::DownloadImage(url, path, width, height) => {
                let ctx = glib::MainContext::default();
                ctx.spawn_local(async move {
                    ncmapi
                        .client
                        .download_img(url, path, width, height)
                        .await
                        .ok();
                });
            }
            Action::SetupTopPicks(song_list) => {
                let window = imp.window.get().unwrap().upgrade().unwrap();
                window.setup_top_picks(song_list);
            }
            Action::InitNewAlbums => {
                let sender = imp.sender.clone();
                let ctx = glib::MainContext::default();
                ctx.spawn_local(async move {
                    if let Ok(song_list) = ncmapi.client.new_albums("ALL", 0, 8).await {
                        for sl in song_list.clone() {
                            let mut path = CACHE.clone();
                            path.push(format!("{}-songlist.jpg", sl.id));
                            sender
                                .send(Action::DownloadImage(sl.cover_img_url, path, 140, 140))
                                .unwrap();
                        }
                        sender.send(Action::SetupNewAlbums(song_list)).unwrap();
                    } else {
                        sender.send(Action::InitNewAlbums).unwrap();
                    }
                });
            }
            Action::InitAllAlbums => {
                let window = imp.window.get().unwrap().upgrade().unwrap();
                window.init_all_albums();
                let sender = imp.sender.clone();
                sender
                    .send(Action::Search(String::new(), SearchType::AllAlbums, 0, 50))
                    .unwrap();
            }
            Action::SetupNewAlbums(song_list) => {
                let window = imp.window.get().unwrap().upgrade().unwrap();
                window.setup_new_albums(song_list);
            }
            Action::AddPlay(song_info) => {
                let window = imp.window.get().unwrap().upgrade().unwrap();
                window.add_play(song_info.clone());
                let sender = imp.sender.clone();
                sender.send(Action::Play(song_info)).unwrap();
            }
            Action::PlayNextSong => {
                let window = imp.window.get().unwrap().upgrade().unwrap();
                window.play_next();
            }
            Action::Play(song_info) => {
                let sender = imp.sender.clone();
                let mut path_m4a = CACHE.clone();
                path_m4a.push(format!("{}.m4a", song_info.id));
                let mut path_mp3 = CACHE.clone();
                path_mp3.push(format!("{}.mp3", song_info.id));
                let mut path_flac = CACHE.clone();
                path_flac.push(format!("{}.flac", song_info.id));
                let mut path_cover = CACHE.clone();
                path_cover.push(format!("{}-cover.jpg", song_info.album_id));
                sender
                    .send(Action::DownloadImage(
                        song_info.pic_url.to_owned(),
                        path_cover,
                        50,
                        50,
                    ))
                    .unwrap();
                if !path_mp3.exists() && !path_flac.exists() && !path_m4a.exists() {
                    let ctx = glib::MainContext::default();
                    ctx.spawn_local(async move {
                        let music_rate = window.settings().uint("music-rate");
                        ncmapi.set_rate(music_rate);
                        if song_info.song_url.is_empty() {
                            if let Ok(song_url) = ncmapi.songs_url(&[song_info.id]).await {
                                if let Some(song_url) = song_url.get(0) {
                                    let song_info = SongInfo {
                                        song_url: song_url.url.to_owned(),
                                        ..song_info
                                    };
                                    sender.send(Action::DownloadSong(song_info)).unwrap();
                                } else {
                                    sender
                                        .send(Action::AddToast(gettext!(
                                            "Get [{}] Playback link failed!",
                                            song_info.name
                                        )))
                                        .unwrap();
                                }
                            } else {
                                sender
                                    .send(Action::AddToast(gettext!(
                                        "Get [{}] Playback link failed!",
                                        song_info.name
                                    )))
                                    .unwrap();
                            }
                        } else {
                            sender.send(Action::DownloadSong(song_info)).unwrap();
                        }
                    });
                } else {
                    let song_info = SongInfo {
                        song_url: if path_mp3.exists() {
                            format!("file://{}", path_mp3.to_str().unwrap().to_owned())
                        } else if path_flac.exists() {
                            format!("file://{}", path_flac.to_str().unwrap().to_owned())
                        } else {
                            format!("file://{}", path_m4a.to_str().unwrap().to_owned())
                        },
                        ..song_info
                    };
                    sender.send(Action::PlayStart(song_info)).unwrap();
                }
            }
            Action::DownloadSong(song_info) => {
                let sender = imp.sender.clone();
                let ctx = glib::MainContext::default();
                let mut path = CACHE.clone();
                if song_info.song_url.ends_with("mp3") {
                    path.push(format!("{}.mp3", song_info.id));
                } else if song_info.song_url.ends_with("flac") {
                    path.push(format!("{}.flac", song_info.id));
                } else {
                    path.push(format!("{}.m4a", song_info.id));
                }
                ctx.spawn_local(async move {
                    if ncmapi
                        .client
                        .download_song(song_info.song_url.to_owned(), path.clone())
                        .await
                        .is_ok()
                    {
                        let song_info = SongInfo {
                            song_url: format!("file://{}", path.to_str().unwrap().to_owned()),
                            ..song_info
                        };
                        sender.send(Action::PlayStart(song_info)).unwrap();
                    }
                });
            }
            Action::PlayStart(song_info) => {
                let window = imp.window.get().unwrap().upgrade().unwrap();
                window.play(song_info);
            }
            Action::ToSongListPage(songlist) => {
                let sender = imp.sender.clone();
                let ctx = glib::MainContext::default();
                ctx.spawn_local(async move {
                    if let Ok(sis) = ncmapi.client.song_list_detail(songlist.id).await {
                        sender.send(Action::InitSongListPage(sis)).unwrap();
                    } else {
                        sender
                            .send(Action::AddToast(gettext(
                                "Failed to get song list details!",
                            )))
                            .unwrap();
                    }
                });
                let window = imp.window.get().unwrap().upgrade().unwrap();
                window.switch_stack_to_songlist_page(&songlist);
            }
            Action::InitSongListPage(sis) => {
                let window = imp.window.get().unwrap().upgrade().unwrap();
                window.init_songlist_page(sis, DiscoverSubPage::SongList);
            }
            Action::ToAlbumPage(songlist) => {
                let sender = imp.sender.clone();
                let ctx = glib::MainContext::default();
                ctx.spawn_local(async move {
                    if let Ok(sis) = ncmapi.client.album(songlist.id).await {
                        sender.send(Action::InitAlbumPage(sis)).unwrap();
                    } else {
                        sender
                            .send(Action::AddToast(gettext("Failed to get album details!")))
                            .unwrap();
                    }
                });
                let window = imp.window.get().unwrap().upgrade().unwrap();
                window.switch_stack_to_songlist_page(&songlist);
            }
            Action::InitAlbumPage(sis) => {
                let window = imp.window.get().unwrap().upgrade().unwrap();
                window.init_songlist_page(sis, DiscoverSubPage::Album);
            }
            Action::AddPlayList(sis) => {
                let window = imp.window.get().unwrap().upgrade().unwrap();
                window.add_playlist(sis);
            }
            Action::PlayListStart => {
                let window = imp.window.get().unwrap().upgrade().unwrap();
                window.playlist_start();
            }
            Action::LikeSongList(id) => {
                let sender = imp.sender.clone();
                let ctx = glib::MainContext::default();
                ctx.spawn_local(async move {
                    if ncmapi.client.song_list_like(true, id).await {
                        sender
                            .send(Action::AddToast(gettext("Song list have been collected!")))
                            .unwrap();
                    } else {
                        sender
                            .send(Action::AddToast(gettext("Failed to collect song list!")))
                            .unwrap();
                    }
                });
            }
            Action::LikeAlbum(id) => {
                let sender = imp.sender.clone();
                let ctx = glib::MainContext::default();
                ctx.spawn_local(async move {
                    if ncmapi.client.album_like(true, id).await {
                        sender
                            .send(Action::AddToast(gettext("Album have been collected!")))
                            .unwrap();
                    } else {
                        sender
                            .send(Action::AddToast(gettext("Failed to collect album!")))
                            .unwrap();
                    }
                });
            }
            Action::LikeSong(id) => {
                let sender = imp.sender.clone();
                let ctx = glib::MainContext::default();
                ctx.spawn_local(async move {
                    if ncmapi.client.like(true, id).await {
                        sender
                            .send(Action::AddToast(gettext("Songs have been collected!")))
                            .unwrap();
                    } else {
                        sender
                            .send(Action::AddToast(gettext("Failed to collect songs!")))
                            .unwrap();
                    }
                });
            }
            Action::GetToplist => {
                let sender = imp.sender.clone();
                let ctx = glib::MainContext::default();
                ctx.spawn_local(async move {
                    if let Ok(toplist) = ncmapi.client.toplist().await {
                        for t in &toplist {
                            let mut path = CACHE.clone();
                            path.push(format!("{}-toplist.jpg", t.id));
                            sender
                                .send(Action::DownloadImage(t.cover.to_owned(), path, 140, 140))
                                .unwrap();
                        }
                        timeout_future_seconds(1).await;
                        sender.send(Action::InitTopList(toplist)).unwrap();
                    } else {
                        sender.send(Action::GetToplist).unwrap();
                    }
                });
            }
            Action::GetToplistSongsList(id) => {
                let sender = imp.sender.clone();
                let ctx = glib::MainContext::default();
                ctx.spawn_local(async move {
                    if let Ok(sis) = ncmapi.client.song_list_detail(id).await {
                        sender.send(Action::UpdateTopList(sis)).unwrap();
                    } else {
                        sender
                            .send(Action::AddToast(gettext(
                                "Request for interface failed, please try again!",
                            )))
                            .unwrap();
                    }
                });
            }
            Action::InitTopList(toplist) => {
                let window = imp.window.get().unwrap().upgrade().unwrap();
                window.init_toplist(toplist);
            }
            Action::UpdateTopList(sis) => {
                let window = imp.window.get().unwrap().upgrade().unwrap();
                window.update_toplist(sis);
            }
            Action::Search(text, search_type, offset, limit) => {
                let sender = imp.sender.clone();
                let ctx = glib::MainContext::default();
                ctx.spawn_local(async move {
                    match search_type {
                        SearchType::Song => {
                            if let Ok(sis) = ncmapi.client.search_song(text, offset, limit).await {
                                sender.send(Action::UpdateSearchSongPage(sis)).unwrap();
                            } else {
                                sender
                                    .send(Action::AddToast(gettext(
                                        "Request for interface failed, please try again!",
                                    )))
                                    .unwrap();
                            }
                        }
                        SearchType::Singer => {
                            if let Ok(sgs) = ncmapi.client.search_singer(text, offset, limit).await
                            {
                                for t in &sgs {
                                    let mut path = CACHE.clone();
                                    path.push(format!("{}-singer.jpg", t.id));
                                    sender
                                        .send(Action::DownloadImage(
                                            t.pic_url.to_owned(),
                                            path,
                                            140,
                                            140,
                                        ))
                                        .unwrap();
                                }
                                timeout_future_seconds(1).await;
                                sender.send(Action::UpdateSearchSingerPage(sgs)).unwrap();
                            } else {
                                sender
                                    .send(Action::AddToast(gettext(
                                        "Request for interface failed, please try again!",
                                    )))
                                    .unwrap();
                            }
                        }
                        SearchType::Album => {
                            if let Ok(sls) = ncmapi.client.search_album(text, offset, limit).await {
                                for t in &sls {
                                    let mut path = CACHE.clone();
                                    path.push(format!("{}-songlist.jpg", t.id));
                                    sender
                                        .send(Action::DownloadImage(
                                            t.cover_img_url.to_owned(),
                                            path,
                                            140,
                                            140,
                                        ))
                                        .unwrap();
                                }
                                timeout_future_seconds(1).await;
                                sender.send(Action::UpdateSearchSongListPage(sls)).unwrap();
                            } else {
                                sender
                                    .send(Action::AddToast(gettext(
                                        "Request for interface failed, please try again!",
                                    )))
                                    .unwrap();
                            }
                        }
                        SearchType::Lyrics => {
                            if let Ok(sis) = ncmapi.client.search_lyrics(text, offset, limit).await
                            {
                                sender.send(Action::UpdateSearchSongPage(sis)).unwrap();
                            } else {
                                sender
                                    .send(Action::AddToast(gettext(
                                        "Request for interface failed, please try again!",
                                    )))
                                    .unwrap();
                            }
                        }
                        SearchType::SongList => {
                            if let Ok(sls) =
                                ncmapi.client.search_songlist(text, offset, limit).await
                            {
                                for t in &sls {
                                    let mut path = CACHE.clone();
                                    path.push(format!("{}-songlist.jpg", t.id));
                                    sender
                                        .send(Action::DownloadImage(
                                            t.cover_img_url.to_owned(),
                                            path,
                                            140,
                                            140,
                                        ))
                                        .unwrap();
                                }
                                timeout_future_seconds(1).await;
                                sender.send(Action::UpdateSearchSongListPage(sls)).unwrap();
                            } else {
                                sender
                                    .send(Action::AddToast(gettext(
                                        "Request for interface failed, please try again!",
                                    )))
                                    .unwrap();
                            }
                        }
                        SearchType::TopPicks => {
                            if let Ok(sls) = ncmapi
                                .client
                                .top_song_list("全部", "hot", offset, limit)
                                .await
                            {
                                for t in &sls {
                                    let mut path = CACHE.clone();
                                    path.push(format!("{}-songlist.jpg", t.id));
                                    sender
                                        .send(Action::DownloadImage(
                                            t.cover_img_url.to_owned(),
                                            path,
                                            140,
                                            140,
                                        ))
                                        .unwrap();
                                }
                                timeout_future_seconds(1).await;
                                sender.send(Action::UpdateSearchSongListPage(sls)).unwrap();
                            } else {
                                sender
                                    .send(Action::AddToast(gettext(
                                        "Request for interface failed, please try again!",
                                    )))
                                    .unwrap();
                            }
                        }
                        SearchType::AllAlbums => {
                            if let Ok(sls) = ncmapi.client.new_albums("ALL", offset, limit).await {
                                for sl in sls.clone() {
                                    let mut path = CACHE.clone();
                                    path.push(format!("{}-songlist.jpg", sl.id));
                                    sender
                                        .send(Action::DownloadImage(
                                            sl.cover_img_url,
                                            path,
                                            140,
                                            140,
                                        ))
                                        .unwrap();
                                }
                                timeout_future_seconds(1).await;
                                sender.send(Action::UpdateSearchSongListPage(sls)).unwrap();
                            } else {
                                sender
                                    .send(Action::AddToast(gettext(
                                        "Request for interface failed, please try again!",
                                    )))
                                    .unwrap();
                            }
                        }
                        SearchType::Fm => {
                            if let Ok(sis) = ncmapi.client.personal_fm().await {
                                sender.send(Action::UpdateSearchSongPage(sis)).unwrap();
                            } else {
                                sender
                                    .send(Action::AddToast(gettext(
                                        "Request for interface failed, please try again!",
                                    )))
                                    .unwrap();
                            }
                        }
                        SearchType::LikeAlbums => {
                            if let Ok(sls) = ncmapi.client.album_sublist(offset, limit).await {
                                for t in &sls {
                                    let mut path = CACHE.clone();
                                    path.push(format!("{}-songlist.jpg", t.id));
                                    sender
                                        .send(Action::DownloadImage(
                                            t.cover_img_url.to_owned(),
                                            path,
                                            140,
                                            140,
                                        ))
                                        .unwrap();
                                }
                                timeout_future_seconds(1).await;
                                sender.send(Action::UpdateSearchSongListPage(sls)).unwrap();
                            } else {
                                sender
                                    .send(Action::AddToast(gettext(
                                        "Request for interface failed, please try again!",
                                    )))
                                    .unwrap();
                            }
                        }
                        SearchType::LikeSongList => {
                            let uid = UID.get().unwrap();
                            if let Ok(sls) = ncmapi.client.user_song_list(*uid, offset, limit).await
                            {
                                for t in &sls {
                                    let mut path = CACHE.clone();
                                    path.push(format!("{}-songlist.jpg", t.id));
                                    sender
                                        .send(Action::DownloadImage(
                                            t.cover_img_url.to_owned(),
                                            path,
                                            140,
                                            140,
                                        ))
                                        .unwrap();
                                }
                                timeout_future_seconds(1).await;
                                sender.send(Action::UpdateSearchSongListPage(sls)).unwrap();
                            } else {
                                sender
                                    .send(Action::AddToast(gettext(
                                        "Request for interface failed, please try again!",
                                    )))
                                    .unwrap();
                            }
                        }
                        _ => (),
                    }
                });
            }
            Action::UpdateSearchSongPage(sis) => {
                let window = imp.window.get().unwrap().upgrade().unwrap();
                window.update_search_song_page(sis);
            }
            Action::UpdateSearchSongListPage(sls) => {
                let window = imp.window.get().unwrap().upgrade().unwrap();
                window.update_search_songlist_page(sls);
            }
            Action::UpdateSearchSingerPage(sgs) => {
                let window = imp.window.get().unwrap().upgrade().unwrap();
                window.update_search_singer_page(sgs);
            }
            Action::ToSingerSongsPage(singer) => {
                let window = imp.window.get().unwrap().upgrade().unwrap();
                window.init_search_song_page(&singer.name, SearchType::SingerSongs);
                let sender = imp.sender.clone();
                let ctx = glib::MainContext::default();
                ctx.spawn_local(async move {
                    if let Ok(sis) = ncmapi.client.singer_songs(singer.id).await {
                        sender.send(Action::UpdateSearchSongPage(sis)).unwrap();
                    } else {
                        sender
                            .send(Action::AddToast(gettext(
                                "Request for interface failed, please try again!",
                            )))
                            .unwrap();
                    }
                });
            }
            Action::ToMyPageDailyRec => {
                let window = imp.window.get().unwrap().upgrade().unwrap();
                window
                    .init_search_song_page(&gettext("Daily Recommendation"), SearchType::DailyRec);
                let sender = imp.sender.clone();
                let ctx = glib::MainContext::default();
                ctx.spawn_local(async move {
                    if let Ok(sis) = ncmapi.client.recommend_songs().await {
                        sender.send(Action::UpdateSearchSongPage(sis)).unwrap();
                    } else {
                        sender
                            .send(Action::AddToast(gettext(
                                "Request for interface failed, please try again!",
                            )))
                            .unwrap();
                    }
                });
            }
            Action::ToMyPageHeartbeat => {
                let window = imp.window.get().unwrap().upgrade().unwrap();
                window.init_search_song_page(&gettext("Favorite Songs"), SearchType::Heartbeat);
                let sender = imp.sender.clone();
                let ctx = glib::MainContext::default();
                ctx.spawn_local(async move {
                    let uid = UID.get().unwrap();
                    if let Ok(sls) = ncmapi.client.user_song_list(*uid, 0, 30).await {
                        if !sls.is_empty() {
                            if let Ok(sis) = ncmapi.client.song_list_detail(sls[0].id).await {
                                sender.send(Action::UpdateSearchSongPage(sis)).unwrap();
                            } else {
                                sender
                                    .send(Action::AddToast(gettext(
                                        "Failed to get song list details!",
                                    )))
                                    .unwrap();
                            }
                        }
                    } else {
                        sender
                            .send(Action::AddToast(gettext(
                                "Request for interface failed, please try again!",
                            )))
                            .unwrap();
                    }
                });
            }
            Action::ToMyPageCloudDisk => {
                let window = imp.window.get().unwrap().upgrade().unwrap();
                window.init_search_song_page(&gettext("Cloud Music"), SearchType::CloudDisk);
                let sender = imp.sender.clone();
                let ctx = glib::MainContext::default();
                ctx.spawn_local(async move {
                    if let Ok(sis) = ncmapi.client.user_cloud_disk().await {
                        sender.send(Action::UpdateSearchSongPage(sis)).unwrap();
                    } else {
                        sender
                            .send(Action::AddToast(gettext(
                                "Request for interface failed, please try again!",
                            )))
                            .unwrap();
                    }
                });
            }
            Action::ToMyPageFm => {
                let window = imp.window.get().unwrap().upgrade().unwrap();
                window.init_search_song_page(&gettext("Private FM"), SearchType::Fm);
                let sender = imp.sender.clone();
                let ctx = glib::MainContext::default();
                ctx.spawn_local(async move {
                    let mut vec = Vec::new();
                    for _ in 0..7 {
                        if let Ok(mut sis) = ncmapi.client.personal_fm().await {
                            vec.append(&mut sis);
                        }
                    }
                    sender.send(Action::UpdateSearchSongPage(vec)).unwrap();
                });
            }
            Action::ToMyPageAlbums => {
                let window = imp.window.get().unwrap().upgrade().unwrap();
                window
                    .init_search_songlist_page(&gettext("Favorite Album"), SearchType::LikeAlbums);
                let sender = imp.sender.clone();
                sender
                    .send(Action::Search(String::new(), SearchType::LikeAlbums, 0, 50))
                    .unwrap();
            }
            Action::ToMyPageSonglist => {
                let window = imp.window.get().unwrap().upgrade().unwrap();
                window.init_search_songlist_page(
                    &gettext("Favorite Song List"),
                    SearchType::LikeSongList,
                );
                let sender = imp.sender.clone();
                sender
                    .send(Action::Search(
                        String::new(),
                        SearchType::LikeSongList,
                        1,
                        50,
                    ))
                    .unwrap();
            }
            Action::InitMyPage => {
                let window = imp.window.get().unwrap().upgrade().unwrap();
                window.switch_my_page_to_login();
                let sender = imp.sender.clone();
                let ctx = glib::MainContext::default();
                ctx.spawn_local(async move {
                    if let Ok(sls) = ncmapi.client.recommend_resource().await {
                        for t in &sls {
                            let mut path = CACHE.clone();
                            path.push(format!("{}-songlist.jpg", t.id));
                            sender
                                .send(Action::DownloadImage(
                                    t.cover_img_url.to_owned(),
                                    path,
                                    140,
                                    140,
                                ))
                                .unwrap();
                        }
                        timeout_future_seconds(1).await;
                        sender.send(Action::InitMyPageRecSongList(sls)).unwrap();
                    } else {
                        sender.send(Action::InitMyPage).unwrap();
                    }
                });
            }
            Action::InitMyPageRecSongList(sls) => {
                let window = imp.window.get().unwrap().upgrade().unwrap();
                window.init_my_page(sls);
            }
            Action::ToPlayListLyricsPage(sis, si) => {
                let window = imp.window.get().unwrap().upgrade().unwrap();
                window.init_playlist_lyrics_page(sis, si.to_owned());
                let sender = imp.sender.clone();
                sender.send(Action::GetLyrics(si)).unwrap();
            }
            Action::GetLyrics(si) => {
                let sender = imp.sender.clone();
                let ctx = glib::MainContext::default();
                ctx.spawn_local(async move {
                    if let Ok(lrc) = ncmapi.get_lyrics(si.id).await {
                        sender.send(Action::UpdateLyrics(lrc)).unwrap();
                    } else {
                        sender
                            .send(Action::UpdateLyrics(gettext("No lyrics found!")))
                            .unwrap();
                    }
                });
            }
            Action::UpdateLyrics(lrc) => {
                let window = imp.window.get().unwrap().upgrade().unwrap();
                window.update_lyrics(lrc);
            }
            Action::UpdatePlayListStatus(index) => {
                let window = imp.window.get().unwrap().upgrade().unwrap();
                window.updat_playlist_status(index);
            }
        }
        glib::Continue(true)
    }

    fn setup_gactions(&self) {
        let preferences_action = gio::SimpleAction::new("preferences", None);
        preferences_action.connect_activate(clone!(@weak self as app => move |_, _| {
            app.show_prefrerences();
        }));
        self.add_action(&preferences_action);

        let quit_action = gio::SimpleAction::new("quit", None);
        quit_action.connect_activate(clone!(@weak self as app => move |_, _| {
            app.quit();
        }));
        self.add_action(&quit_action);

        let about_action = gio::SimpleAction::new("about", None);
        about_action.connect_activate(clone!(@weak self as app => move |_, _| {
            app.show_about();
        }));
        self.add_action(&about_action);
    }

    fn show_prefrerences(&self) {
        let window = self.active_window().unwrap();
        let preferences = NeteaseCloudMusicGtk4Preferences::new();
        preferences.set_modal(true);
        preferences.set_transient_for(Some(&window));
        preferences.present();
    }

    fn show_about(&self) {
        let window = self.active_window().unwrap();
        let dialog = gtk::AboutDialog::builder()
            .transient_for(&window)
            .modal(true)
            .program_name(&gettext("Netease Cloud Music Gtk4"))
            .logo_icon_name("logo")
            .version(VERSION)
            .authors(vec!["gmg137".into()])
            .website("https://github.com/gmg137/netease-cloud-music-gtk")
            .license_type(gtk::License::Gpl30)
            .build();

        dialog.present();
    }

    fn setup_cache_clear(&self) {
        let sender = self.imp().sender.clone();
        let settings = Settings::new(crate::APP_ID);
        let cache_clear = settings.uint("cache-clear");
        let flag = settings.boolean("cache-clear-flag");
        let cache_path = CACHE.clone();
        let ctx = glib::MainContext::default();
        ctx.spawn_local(async move {
            match cache_clear {
                1 => {
                    if remove_all_file(cache_path).is_ok() {
                        sender
                            .send(Action::AddToast(gettext("Cache cleared.")))
                            .unwrap();
                    }
                }
                2 => {
                    if let Ok(datetime) = glib::DateTime::now_local() {
                        if datetime.day_of_week() == 1 && !flag {
                            if remove_all_file(cache_path).is_ok() {
                                sender
                                    .send(Action::AddToast(gettext("Cache cleared.")))
                                    .unwrap();
                            }
                            settings.set_boolean("cache-clear-flag", true).unwrap();
                        } else if datetime.day_of_week() != 1 {
                            settings.set_boolean("cache-clear-flag", false).unwrap();
                        }
                    }
                }
                3 => {
                    if let Ok(datetime) = glib::DateTime::now_local() {
                        if datetime.day_of_month() == 1 && !flag {
                            if remove_all_file(cache_path).is_ok() {
                                sender
                                    .send(Action::AddToast(gettext("Cache cleared.")))
                                    .unwrap();
                            }
                            settings.set_boolean("cache-clear-flag", true).unwrap();
                        } else if datetime.day_of_month() != 1 {
                            settings.set_boolean("cache-clear-flag", false).unwrap();
                        }
                    }
                }
                _ => {
                    settings.set_boolean("cache-clear-flag", false).unwrap();
                }
            }
        });
    }
}

impl Default for NeteaseCloudMusicGtk4Application {
    fn default() -> Self {
        gio::Application::default()
            .expect("Could not get default GApplication")
            .downcast()
            .unwrap()
    }
}

fn remove_all_file(path: PathBuf) -> anyhow::Result<()> {
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() {
            fs::remove_file(path)?;
        }
    }
    Ok(())
}
