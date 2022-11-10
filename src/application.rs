use adw::subclass::prelude::*;
use gettextrs::gettext;
use gio::Settings;
use glib::{clone, timeout_future, timeout_future_seconds, MainContext, Receiver, Sender, WeakRef};
use gtk::{gio, glib, prelude::*};
use log::*;
use ncm_api::{
    AlbumDetailDynamic, BannersInfo, CookieJar, DetailDynamic, LoginInfo, SingerInfo, SongInfo,
    SongList, SongListDetailDynamic, TopList,
};
use once_cell::sync::OnceCell;
use std::fs;
use std::path::PathBuf;
use std::time::Duration;
use std::{cell::RefCell, sync::Arc};

use crate::{
    config::VERSION, gui::NeteaseCloudMusicGtk4Preferences, model::*, ncmapi::*, path::CACHE,
    NeteaseCloudMusicGtk4Window,
};

// implements Debug for Fn(Targ) using "blanket implementations"
pub trait ActionCallbackTr<TArg>: Fn(TArg) + Sync + Send {}
impl<Targ, Tr: Fn(Targ) + Sync + Send> ActionCallbackTr<Targ> for Tr {}
impl<Targ> std::fmt::Debug for dyn ActionCallbackTr<Targ> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ActionCallback")
    }
}
// wrapper dyn Fn(Targ) => ActionCallback<Targ>
// Note: we can capture glib object with glib::SendWeakRef, but only valied in MainContext thread

// callback is needed as there is no way to lookup the sender object
// alternative methods:
//   unique id for sender object, and store a map
//   sender object create new (sender, receiver) and attach, then action send back
pub type ActionCallback<Targ = ()> = Arc<dyn ActionCallbackTr<Targ>>;

#[derive(Debug, Clone)]
pub enum Action {
    AddToast(String),

    // (关键字，搜索类型，起始点，数量)
    Search(String, SearchType, u16, u16, ActionCallback<SearchResult>),
    // (url,path,width,height)
    DownloadImage(String, PathBuf, u16, u16, Option<ActionCallback>),
    LikeSongList(u64, bool, Option<ActionCallback>),
    LikeAlbum(u64, bool, Option<ActionCallback>),
    LikeSong(u64, bool, Option<ActionCallback>),

    // play
    AddPlay(SongInfo),
    PlayNextSong,
    Play(SongInfo),
    PlayStart(SongInfo),
    AddPlayList(Vec<SongInfo>),
    PlayListStart,

    // login
    CheckLogin(UserMenuChild, CookieJar),
    Logout,
    InitUserInfo(LoginInfo),
    SetAvatar(PathBuf),
    SwitchUserMenuToPhone,
    SwitchUserMenuToQr,
    SwitchUserMenuToUser(LoginInfo, UserMenuChild),
    GetCaptcha(String, String),
    CaptchaLogin(String, String, String),

    // Qr
    TryUpdateQrCode,
    SetQrImage(PathBuf),
    CheckQrTimeout(String),
    CheckQrTimeoutCb(String),
    SetQrImageTimeout,

    // discover
    InitCarousel,
    AddCarousel(BannersInfo),
    DownloadBanners(BannersInfo),
    InitTopPicks,
    SetupTopPicks(Vec<SongList>),
    InitNewAlbums,
    SetupNewAlbums(Vec<SongList>),

    // toplist
    GetToplist,
    GetToplistSongsList(u64),
    InitTopList(Vec<TopList>),
    UpdateTopList(Vec<SongInfo>),

    // my
    InitMyPage,
    InitMyPageRecSongList(Vec<SongList>),

    // playlist
    ToPlayListLyricsPage(Vec<SongInfo>, SongInfo),
    UpdateLyrics(String),
    UpdatePlayListStatus(usize),
    GetLyrics(SongInfo),

    // page routing
    ToTopPicksPage,
    ToAllAlbumsPage,
    ToSongListPage(SongList),
    ToAlbumPage(SongList),
    ToSingerSongsPage(SingerInfo),
    ToMyPageDailyRec,
    ToMyPageHeartbeat,
    ToMyPageFm,
    ToMyPageCloudDisk,
    ToMyPageAlbums,
    ToMyPageSonglist,
    PageBack,

    // gst
    GstPositionUpdate(u64),
    GstDurationChanged(u64),
    GstStateChanged(gstreamer_play::PlayState),
    GstCacheDownloadComplete(String),
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
        fn constructed(&self) {
            let obj = self.obj();
            self.parent_constructed();

            obj.setup_gactions();
            obj.setup_cache_clear();
            obj.set_accels_for_action("app.quit", &["<primary>q"]);
            obj.set_accels_for_action("win.search-button", &["<primary>f", "slash"]);
            obj.set_accels_for_action("win.back-button", &["<primary>BackSpace"]);
        }
    }

    impl ApplicationImpl for NeteaseCloudMusicGtk4Application {
        // We connect to the activate callback to create a window when the application
        // has been launched. Additionally, this callback notifies us when the user
        // tries to launch a "second instance" of the application. When they try
        // to do that, we'll just present any existing window.
        fn activate(&self) {
            let obj = self.obj();
            let app = obj
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
    }

    fn create_window(&self) -> NeteaseCloudMusicGtk4Window {
        let imp = self.imp();
        let window = NeteaseCloudMusicGtk4Window::new(&self.clone(), imp.sender.clone());

        window.present();
        window
    }

    fn init_ncmapi(&self, cli: NcmClient) -> NcmClient {
        let window = self.imp().window.get().unwrap().upgrade().unwrap();
        let mut ncmapi = cli;
        let proxy_address = window.settings().string("proxy-address").to_string();
        if !proxy_address.is_empty() && ncmapi.set_proxy(proxy_address).is_err() {
            // do nothing
        }
        ncmapi
    }

    fn process_action(&self, action: Action) -> glib::Continue {
        let imp = self.imp();
        if self.active_window().is_none() {
            return glib::Continue(true);
        }

        let window = imp.window.get().unwrap().upgrade().unwrap();
        let mut ncmapi = self.init_ncmapi(NcmClient::new());

        match action {
            Action::CheckLogin(user_menu, logined_cookie_jar) => {
                let sender = imp.sender.clone();
                let ctx = glib::MainContext::default();
                ncmapi = self.init_ncmapi(NcmClient::from_cookie_jar(logined_cookie_jar));

                ctx.spawn_local(async move {
                    if !window.is_logined() {
                        if let Ok(login_info) = ncmapi.client.login_status().await {
                            debug!("获取用户信息成功: {:?}", login_info);
                            window.set_uid(login_info.uid);

                            ncmapi.set_cookie_jar_to_global();
                            ncmapi.save_global_cookie_jar_to_file();

                            sender
                                .send(Action::InitUserInfo(login_info.to_owned()))
                                .unwrap();
                            sender
                                .send(Action::SwitchUserMenuToUser(login_info, user_menu))
                                .unwrap();
                            sender.send(Action::InitMyPage).unwrap();
                            sender
                                .send(Action::AddToast(gettext("Login successful!")))
                                .unwrap();
                        } else {
                            error!("获取用户信息失败！");
                            sender
                                .send(Action::AddToast(gettext("Login failed!")))
                                .unwrap();
                            NcmClient::clean_global_cookie_jar_and_file();
                        }
                    }
                });
            }
            Action::Logout => {
                let sender = imp.sender.clone();
                let ctx = glib::MainContext::default();
                ctx.spawn_local(async move {
                    ncmapi.client.logout().await;
                    NcmClient::clean_global_cookie_jar_and_file();

                    window.logout();
                    window.switch_my_page_to_logout();
                    sender.send(Action::SwitchUserMenuToQr).unwrap();
                    sender.send(Action::AddToast(gettext("Logout!"))).unwrap();
                });
            }
            Action::InitUserInfo(login_info) => {
                let ctx = glib::MainContext::default();
                ctx.spawn_local(async move {
                    if let Ok(song_ids) = ncmapi.client.user_song_id_list(login_info.uid).await {
                        window.set_user_like_songs(&song_ids);
                    }
                });
            }
            Action::TryUpdateQrCode => {
                if !window.is_logined() && window.is_user_menu_active(UserMenuChild::Qr) {
                    let sender = imp.sender.clone();
                    let ctx = glib::MainContext::default();
                    ctx.spawn_local(async move {
                        if let Ok(res) = ncmapi.create_qrcode().await {
                            sender.send(Action::SetQrImage(res.0)).unwrap();
                            sender.send(Action::CheckQrTimeout(res.1)).unwrap();
                        }
                    });
                }
            }
            Action::SetQrImage(path) => {
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
                debug!("检查登录二维码状态，unikey={}", unikey);
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
                                warn!("unikey 已失效，unikey={}", unikey);
                                break;
                            }
                        }
                        if let Ok(msg) = ncmapi.client.login_qr_check(unikey.to_owned()).await {
                            match msg.code {
                                // 已过期
                                800 => {
                                    debug!("二维码已过期，unikey={}", unikey);
                                    sender.send(Action::SetQrImageTimeout).unwrap();
                                    break;
                                }
                                // 等待扫码
                                801 => {
                                    debug!("等待扫码，unikey={}", unikey);
                                },
                                // 等待确认
                                802 => {
                                    debug!("等待app端确认，unikey={}", unikey);
                                    if send_toast {
                                        sender
                                            .send(Action::AddToast(gettext("Have scanned the QR code, waiting for confirmation!")))
                                            .unwrap();
                                        send_toast = false;
                                    }
                                }
                                // 登录成功
                                803 => {
                                    debug!("扫码登录成功，unikey={}", unikey);
                                    let cookie_jar = ncmapi.client.cookie_jar().cloned().unwrap_or_else(|| {
                                        error!("No login cookie found");
                                        CookieJar::new()
                                    });
                                    sender.send(Action::CheckLogin(UserMenuChild::Qr, cookie_jar)).unwrap();
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
                        debug!("发送获取验证码请求...");
                        sender
                            .send(Action::AddToast(gettext(
                                "Please pay attention to check the cell phone verification code!",
                            )))
                            .unwrap();
                    } else {
                        warn!("获取验证码失败!");
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
                    debug!("使用验证码登录：{}", captcha);
                    if let Ok(_login_info) =
                        ncmapi.client.login_cellphone(ctcode, phone, captcha).await
                    {
                        let cookie_jar = ncmapi.client.cookie_jar().cloned().unwrap_or_else(|| {
                            error!("No login cookie found");
                            CookieJar::new()
                        });
                        sender
                            .send(Action::CheckLogin(UserMenuChild::Phone, cookie_jar))
                            .unwrap();
                    } else {
                        error!("登录失败！");
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
                        debug!("获取轮播信息: {:?}", banners);
                        for banner in banners {
                            sender.send(Action::DownloadBanners(banner)).unwrap();
                        }

                        // auto check login after banners
                        // https://github.com/Binaryify/NeteaseCloudMusicApi/issues/1217
                        if let Some(cookie_jar) = NcmClient::load_cookie_jar_from_file() {
                            sender
                                .send(Action::CheckLogin(UserMenuChild::Qr, cookie_jar))
                                .unwrap();
                        }
                    } else {
                        error!("获取首页轮播信息失败！");
                        timeout_future(Duration::from_millis(500)).await;
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
                        debug!("获取热门推荐信息：{:?}", song_list);
                        sender.send(Action::SetupTopPicks(song_list)).unwrap();
                    } else {
                        error!("获取热门推荐信息失败！");
                        timeout_future(Duration::from_millis(500)).await;
                        sender.send(Action::InitTopPicks).unwrap();
                    }
                });
            }
            Action::ToTopPicksPage => {
                let window = imp.window.get().unwrap().upgrade().unwrap();
                let page = window.init_picks_songlist();
                window.page_new(&page, gettext("all top picks").as_str());
                let page = page.downgrade();

                let ctx = glib::MainContext::default();
                ctx.spawn_local(async move {
                    if let Some(SearchResult::SongLists(sls)) = window
                        .action_search(ncmapi, String::new(), SearchType::TopPicks, 0, 50)
                        .await
                    {
                        if let Some(page) = page.upgrade() {
                            page.update_songlist(sls);
                        }
                    }
                });
            }
            Action::DownloadImage(url, path, width, height, callback) => {
                let ctx = glib::MainContext::default();
                ctx.spawn_local(async move {
                    if ncmapi
                        .client
                        .download_img(url, path, width, height)
                        .await
                        .is_ok()
                    {
                        if let Some(cb) = callback {
                            cb(());
                        }
                    }
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
                        debug!("获取新碟上架信息：{:?}", song_list);
                        sender.send(Action::SetupNewAlbums(song_list)).unwrap();
                    } else {
                        error!("获取新碟上架信息失败！");
                        timeout_future(Duration::from_millis(500)).await;
                        sender.send(Action::InitNewAlbums).unwrap();
                    }
                });
            }
            Action::ToAllAlbumsPage => {
                let window = imp.window.get().unwrap().upgrade().unwrap();
                let page = window.init_all_albums();

                window.page_new(&page, gettext("all new albums").as_str());
                let page = page.downgrade();

                let ctx = glib::MainContext::default();
                ctx.spawn_local(async move {
                    if let Some(SearchResult::SongLists(sls)) = window
                        .action_search(ncmapi, String::new(), SearchType::AllAlbums, 0, 50)
                        .await
                    {
                        if let Some(page) = page.upgrade() {
                            page.update_songlist(sls);
                        }
                    }
                });
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
                let music_rate = window.settings().uint("music-rate");
                let path = crate::path::get_music_cache_path(song_info.id, music_rate);

                if !path.exists() {
                    let ctx = glib::MainContext::default();
                    ctx.spawn_local(async move {
                        ncmapi.set_rate(music_rate);
                        if song_info.song_url.is_empty() {
                            if let Ok(song_url) = ncmapi.songs_url(&[song_info.id]).await {
                                debug!("获取歌曲播放链接: {:?}", song_url);
                                if let Some(song_url) = song_url.get(0) {
                                    let song_info = SongInfo {
                                        song_url: song_url.url.to_owned(),
                                        ..song_info
                                    };
                                    sender.send(Action::PlayStart(song_info)).unwrap();
                                } else {
                                    sender
                                        .send(Action::AddToast(gettext!(
                                            "Get [{}] Playback link failed!",
                                            song_info.name
                                        )))
                                        .unwrap();
                                }
                            } else {
                                error!("获取歌曲播放链接失败: {:?}", &[song_info.id]);
                                sender
                                    .send(Action::AddToast(gettext!(
                                        "Get [{}] Playback link failed!",
                                        song_info.name
                                    )))
                                    .unwrap();
                            }
                        } else {
                            sender.send(Action::PlayStart(song_info)).unwrap();
                        }
                    });
                } else {
                    let song_info = SongInfo {
                        song_url: format!("file://{}", path.to_str().unwrap().to_owned()),
                        ..song_info
                    };
                    sender.send(Action::PlayStart(song_info)).unwrap();
                }
            }
            Action::PlayStart(song_info) => {
                debug!("播放歌曲: {:?}", song_info);
                let window = imp.window.get().unwrap().upgrade().unwrap();
                window.play(song_info);
            }
            Action::ToSongListPage(songlist) => {
                let page = window.init_songlist_page(&songlist);
                window.page_new(&page, &songlist.name);
                let page = page.downgrade();

                let sender = imp.sender.clone();
                let ctx = glib::MainContext::default();
                ctx.spawn_local(async move {
                    let detal_dynamic_as = ncmapi.client.songlist_detail_dynamic(songlist.id);
                    if let Ok(sis) = ncmapi.client.song_list_detail(songlist.id).await {
                        debug!("获取歌单详情: {:?}", sis);
                        let dy =
                            DetailDynamic::SongList(detal_dynamic_as.await.unwrap_or_else(|err| {
                                error!("{:?}", err);
                                SongListDetailDynamic::default()
                            }));
                        if let Some(page) = page.upgrade() {
                            window.update_songlist_page(page, sis, dy);
                        }
                    } else {
                        error!("获取歌单详情失败: {:?}", songlist);
                        sender
                            .send(Action::AddToast(gettext(
                                "Failed to get song list details!",
                            )))
                            .unwrap();
                    }
                });
            }
            Action::ToAlbumPage(songlist) => {
                let page = window.init_songlist_page(&songlist);
                window.page_new(&page, &songlist.name);
                let page = page.downgrade();

                let sender = imp.sender.clone();
                let ctx = glib::MainContext::default();
                ctx.spawn_local(async move {
                    let detal_dynamic_as = ncmapi.client.album_detail_dynamic(songlist.id);
                    if let Ok(sis) = ncmapi.client.album(songlist.id).await {
                        debug!("获取专辑详情: {:?}", sis);
                        let dy =
                            DetailDynamic::Album(detal_dynamic_as.await.unwrap_or_else(|err| {
                                error!("{:?}", err);
                                AlbumDetailDynamic::default()
                            }));
                        if let Some(page) = page.upgrade() {
                            window.update_songlist_page(page, sis, dy);
                        }
                    } else {
                        error!("获取专辑详情失败: {:?}", songlist);
                        sender
                            .send(Action::AddToast(gettext("Failed to get album details!")))
                            .unwrap();
                    }
                });
            }
            Action::AddPlayList(sis) => {
                let window = imp.window.get().unwrap().upgrade().unwrap();
                window.add_playlist(sis);
            }
            Action::PlayListStart => {
                let window = imp.window.get().unwrap().upgrade().unwrap();
                window.playlist_start();
            }
            Action::LikeSongList(id, is_like, callback) => {
                let sender = imp.sender.clone();
                let ctx = glib::MainContext::default();
                ctx.spawn_local(async move {
                    if ncmapi.client.song_list_like(is_like, id).await {
                        debug!("收藏/取消收藏歌单: {:?}", id);
                        if let Some(callback) = callback {
                            callback(());
                        }
                        sender
                            .send(Action::AddToast(gettext(if is_like {
                                "Song list have been collected!"
                            } else {
                                "Song list have been uncollected!"
                            })))
                            .unwrap();
                    } else {
                        error!("收藏/取消收藏歌单失败: {:?}", id);
                        sender
                            .send(Action::AddToast(gettext(if is_like {
                                "Failed to collect song list!"
                            } else {
                                "Failed to uncollect song list!"
                            })))
                            .unwrap();
                    }
                });
            }
            Action::LikeAlbum(id, is_like, callback) => {
                let sender = imp.sender.clone();
                let ctx = glib::MainContext::default();
                ctx.spawn_local(async move {
                    if ncmapi.client.album_like(is_like, id).await {
                        debug!("收藏/取消收藏专辑: {:?}", id);
                        if let Some(callback) = callback {
                            callback(());
                        }
                        sender
                            .send(Action::AddToast(gettext(if is_like {
                                "Album have been collected!"
                            } else {
                                "Album have been uncollected!"
                            })))
                            .unwrap();
                    } else {
                        error!("收藏/取消收藏专辑失败: {:?}", id);
                        sender
                            .send(Action::AddToast(gettext(if is_like {
                                "Failed to collect album!"
                            } else {
                                "Failed to uncollect album!"
                            })))
                            .unwrap();
                    }
                });
            }
            Action::LikeSong(id, is_like, callback) => {
                let sender = imp.sender.clone();
                let ctx = glib::MainContext::default();
                ctx.spawn_local(async move {
                    if ncmapi.client.like(is_like, id).await {
                        debug!("收藏/取消收藏歌曲: {:?}", id);
                        window.set_like_song(id, is_like);
                        sender
                            .send(Action::AddToast(gettext(if is_like {
                                "Songs have been collected!"
                            } else {
                                "Songs have been uncollected!"
                            })))
                            .unwrap();
                        if let Some(callback) = callback {
                            callback(());
                        }
                    } else {
                        error!("收藏/取消收藏歌曲失败: {:?}", id);
                        sender
                            .send(Action::AddToast(gettext(if is_like {
                                "Failed to collect songs!"
                            } else {
                                "Failed to uncollect songs!"
                            })))
                            .unwrap();
                    }
                });
            }
            Action::GetToplist => {
                let sender = imp.sender.clone();
                let ctx = glib::MainContext::default();
                ctx.spawn_local(async move {
                    if let Ok(toplist) = ncmapi.client.toplist().await {
                        debug!("获取排行榜: {:?}", toplist);
                        sender.send(Action::InitTopList(toplist)).unwrap();
                    } else {
                        error!("获取排行榜失败!");
                        timeout_future(Duration::from_millis(500)).await;
                        sender.send(Action::GetToplist).unwrap();
                    }
                });
            }
            Action::GetToplistSongsList(id) => {
                let sender = imp.sender.clone();
                let ctx = glib::MainContext::default();
                ctx.spawn_local(async move {
                    if let Ok(sis) = ncmapi.client.song_list_detail(id).await {
                        debug!("获取榜单 {} 详情：{:?}", id, sis);
                        sender.send(Action::UpdateTopList(sis)).unwrap();
                    } else {
                        error!("获取榜单 {} 失败!", id);
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
            Action::Search(text, search_type, offset, limit, callback) => {
                let ctx = glib::MainContext::default();
                ctx.spawn_local(async move {
                    let res = window
                        .action_search(ncmapi, text, search_type, offset, limit)
                        .await;
                    if let Some(res) = res {
                        callback(res);
                    }
                });
            }
            Action::ToSingerSongsPage(singer) => {
                let title = &singer.name;
                let page = window.init_search_song_page(title, SearchType::SingerSongs);
                window.page_new(&page, title);
                let page = page.downgrade();

                let sender = imp.sender.clone();
                let ctx = glib::MainContext::default();
                ctx.spawn_local(async move {
                    if let Ok(sis) = ncmapi.client.singer_songs(singer.id).await {
                        debug!("获取歌手单曲：{:?}", sis);
                        if let Some(page) = page.upgrade() {
                            window.update_search_song_page(page, sis);
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
            Action::ToMyPageDailyRec => {
                let title = gettext("Daily Recommendation");
                let page = window.init_search_song_page(&title, SearchType::DailyRec);
                window.page_new(&page, &title);
                let page = page.downgrade();

                let sender = imp.sender.clone();
                let ctx = glib::MainContext::default();
                ctx.spawn_local(async move {
                    if let Ok(sis) = ncmapi.client.recommend_songs().await {
                        debug!("获取每日推荐：{:?}", sis);
                        if let Some(page) = page.upgrade() {
                            window.update_search_song_page(page, sis);
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
            Action::ToMyPageHeartbeat => {
                let title = gettext("Favorite Songs");
                let page = window.init_search_song_page(&title, SearchType::Heartbeat);
                window.page_new(&page, &title);
                let page = page.downgrade();

                let sender = imp.sender.clone();
                let ctx = glib::MainContext::default();
                ctx.spawn_local(async move {
                    let uid = window.get_uid();
                    if let Ok(sls) = ncmapi.client.user_song_list(uid, 0, 30).await {
                        debug!("获取心动歌单：{:?}", sls);
                        if !sls.is_empty() {
                            if let Ok(sis) = ncmapi.client.song_list_detail(sls[0].id).await {
                                if let Some(page) = page.upgrade() {
                                    window.update_search_song_page(page, sis);
                                }
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
                let title = gettext("Cloud Music");
                let page = window.init_search_song_page(&title, SearchType::CloudDisk);
                window.page_new(&page, &title);
                let page = page.downgrade();

                let sender = imp.sender.clone();
                let ctx = glib::MainContext::default();
                ctx.spawn_local(async move {
                    if let Ok(sis) = ncmapi.client.user_cloud_disk().await {
                        debug!("获取云盘音乐：{:?}", sis);
                        if let Some(page) = page.upgrade() {
                            window.update_search_song_page(page, sis);
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
            Action::ToMyPageFm => {
                let title = gettext("Private FM");
                let page = window.init_search_song_page(&title, SearchType::Fm);
                window.page_new(&page, &title);
                let page = page.downgrade();

                let ctx = glib::MainContext::default();
                ctx.spawn_local(async move {
                    let mut vec = Vec::new();
                    for _ in 0..7 {
                        if let Ok(mut sis) = ncmapi.client.personal_fm().await {
                            vec.append(&mut sis);
                        }
                    }
                    debug!("获取 FM：{:?}", vec);
                    if let Some(page) = page.upgrade() {
                        window.update_search_song_page(page, vec);
                    }
                });
            }
            Action::ToMyPageAlbums => {
                let title = gettext("Favorite Album");
                let page = window.init_search_songlist_page(&title, SearchType::LikeAlbums);
                window.page_new(&page, &title);
                let page = page.downgrade();

                let ctx = glib::MainContext::default();
                ctx.spawn_local(async move {
                    let res = window
                        .action_search(ncmapi, String::new(), SearchType::LikeAlbums, 0, 50)
                        .await;
                    if let Some(page) = page.upgrade() {
                        if let Some(SearchResult::SongLists(sls)) = res {
                            page.update_songlist(sls);
                        }
                    }
                });
            }
            Action::ToMyPageSonglist => {
                let title = gettext("Favorite Song List");
                let page = window.init_search_songlist_page(&title, SearchType::LikeSongList);
                window.page_new(&page, &title);
                let page = page.downgrade();

                let ctx = glib::MainContext::default();
                ctx.spawn_local(async move {
                    let res = window
                        .action_search(ncmapi, String::new(), SearchType::LikeSongList, 1, 50)
                        .await;
                    if let Some(page) = page.upgrade() {
                        if let Some(SearchResult::SongLists(sls)) = res {
                            page.update_songlist(sls);
                        }
                    }
                });
            }
            Action::InitMyPage => {
                window.switch_my_page_to_login();
                let sender = imp.sender.clone();
                let ctx = glib::MainContext::default();
                ctx.spawn_local(async move {
                    if let Ok(sls) = ncmapi.client.recommend_resource().await {
                        debug!("获取推荐歌单：{:?}", sls);
                        sender.send(Action::InitMyPageRecSongList(sls)).unwrap();
                    } else {
                        sender.send(Action::InitMyPage).unwrap();
                    }
                });
            }
            Action::InitMyPageRecSongList(sls) => {
                window.init_my_page(sls);
            }
            Action::ToPlayListLyricsPage(sis, si) => {
                let sender = imp.sender.clone();
                if !window.page_cur_playlist_lyrics_page() {
                    window.init_playlist_lyrics_page(sis, si.to_owned());
                    sender.send(Action::GetLyrics(si)).unwrap();
                } else {
                    sender.send(Action::PageBack).unwrap();
                }
            }
            Action::GetLyrics(si) => {
                let sender = imp.sender.clone();
                let ctx = glib::MainContext::default();
                ctx.spawn_local(async move {
                    if let Ok(lrc) = ncmapi.get_lyrics(si.id).await {
                        debug!("获取歌词：{:?}", lrc);
                        sender.send(Action::UpdateLyrics(lrc)).unwrap();
                    } else {
                        sender
                            .send(Action::UpdateLyrics(gettext("No lyrics found!")))
                            .unwrap();
                    }
                });
            }
            Action::UpdateLyrics(lrc) => {
                window.update_lyrics(lrc);
            }
            Action::UpdatePlayListStatus(index) => {
                window.updat_playlist_status(index);
            }
            Action::GstPositionUpdate(sec) => {
                window.gst_position_update(sec);
            }
            Action::GstDurationChanged(sec) => {
                window.gst_duration_changed(sec);
            }
            Action::GstStateChanged(state) => {
                window.gst_state_changed(state);
            }
            Action::GstCacheDownloadComplete(loc) => {
                window.gst_cache_download_complete(loc);
            }

            Action::PageBack => {
                window.page_back();
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

        let (size, unit) = crate::path::get_cache_size();
        preferences.set_cache_size_label(size, unit);

        preferences.present();
    }

    fn show_about(&self) {
        let window = self.active_window().unwrap();
        let dialog = gtk::AboutDialog::builder()
            .transient_for(&window)
            .modal(true)
            .program_name(&gettext(crate::APP_NAME))
            .logo_icon_name("logo")
            .version(VERSION)
            .authors(vec!["gmg137".into(), "catsout".into()])
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
