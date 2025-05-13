use adw::{prelude::AdwDialogExt, subclass::prelude::*};
use async_channel::{unbounded, Receiver, Sender};
use gettextrs::gettext;
use gio::Settings;
use glib::{clone, source::Priority, timeout_future, timeout_future_seconds, WeakRef};
use gtk::{gio, glib, prelude::*};
use log::*;
use ncm_api::{
    AlbumDetailDynamic, BannersInfo, CookieJar, LoginInfo, PlayListDetailDynamic, SingerInfo,
    SongInfo, SongList, TargetType, TopList,
};
use once_cell::sync::OnceCell;
use std::{cell::RefCell, fs, path::PathBuf, sync::Arc, time::Duration};

use crate::{
    audio::MprisController, config::VERSION, gui::NeteaseCloudMusicGtk4Preferences, model::*,
    ncmapi::*, path::CACHE, utils::*, NeteaseCloudMusicGtk4Window, MAINCONTEXT,
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
    Moved(SongInfo),

    // play
    AddPlay(SongInfo),
    PlayNextSong,
    Play(SongInfo),
    PlayStart(SongInfo),
    // (歌单, 是否立即播放)
    AddPlayList(Vec<SongInfo>, bool),
    PlayListStart,
    PersistVolume(f64),
    GetSongUrl(SongInfo),
    SetSongUrl(SongInfo),

    // login
    CheckLogin(UserMenuChild, CookieJar),
    Logout,
    InitUserInfo(LoginInfo),
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
    InitTopPicks,
    SetupTopPicks(Vec<SongList>),
    InitNewAlbums,
    SetupNewAlbums(Vec<SongList>),
    BannerTo(BannersInfo),

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
    UpdateLyrics(SongInfo, u64),
    UpdatePlayListStatus(usize),
    RemoveFromPlayList(SongInfo),

    // page routing
    ToTopPicksPage,
    ToAllAlbumsPage,
    ToSongListPage(SongList),
    ToAlbumPage(SongList),
    ToRadioPage(SongList),
    ToSingerSongsPage(SingerInfo),
    ToMyPageDailyRec,
    ToMyPageHeartbeat,
    ToMyPageRadio,
    ToMyPageCloudDisk,
    ToMyPageAlbums,
    ToMyPageSonglist,
    PageBack,

    // gst
    GstDurationChanged(u64),
    GstStateChanged(gstreamer_play::PlayState),
    GstVolumeChanged(f64),
    GstCacheDownloadComplete(String),
    ScaleSeekUpdate(u64),
    ScaleValueUpdate,

    InitMpris(MprisController),
}

mod imp {

    use std::sync::{Arc, RwLock};

    use super::*;

    pub struct NeteaseCloudMusicGtk4Application {
        pub window: OnceCell<WeakRef<NeteaseCloudMusicGtk4Window>>,
        pub sender: Sender<Action>,
        pub receiver: RefCell<Option<Receiver<Action>>>,
        pub unikey: Arc<RwLock<String>>,
        pub ncmapi: RefCell<Option<NcmClient>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for NeteaseCloudMusicGtk4Application {
        const NAME: &'static str = "NeteaseCloudMusicGtk4Application";
        type Type = super::NeteaseCloudMusicGtk4Application;
        type ParentType = adw::Application;
        fn new() -> Self {
            let (sender, r) = unbounded();
            let receiver = RefCell::new(Some(r));
            let window = OnceCell::new();
            let unikey = Arc::new(RwLock::new(String::new()));
            let ncmapi = RefCell::new(None);

            Self {
                window,
                sender,
                receiver,
                unikey,
                ncmapi,
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
            MAINCONTEXT.spawn_local_with_priority(
                Priority::HIGH,
                clone!(
                    #[strong]
                    app,
                    async move {
                        while let Ok(action) = receiver.recv().await {
                            app.process_action(action);
                        }
                    }
                ),
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
        glib::Object::builder()
            .property("application-id", application_id)
            .property("flags", flags)
            .build()
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

    fn process_action(&self, action: Action) -> glib::ControlFlow {
        let imp = self.imp();
        if self.active_window().is_none() {
            return glib::ControlFlow::Continue;
        }

        let window = imp.window.get().unwrap().upgrade().unwrap();
        let ncmapi = {
            let ncmapi_opt = { imp.ncmapi.borrow().as_ref().cloned() };
            if let Some(ncmapi) = ncmapi_opt {
                ncmapi
            } else {
                let ncmapi = self.init_ncmapi(NcmClient::new());
                imp.ncmapi.replace(Some(ncmapi.clone()));
                ncmapi
            }
        };

        match action {
            Action::CheckLogin(user_menu, logined_cookie_jar) => {
                let sender = imp.sender.clone();
                let ncmapi = self.init_ncmapi(NcmClient::from_cookie_jar(logined_cookie_jar));
                let s = self.clone();

                MAINCONTEXT.spawn_local_with_priority(Priority::HIGH_IDLE, async move {
                    if !window.is_logined() {
                        match ncmapi.client.login_status().await {
                            Ok(login_info) => {
                                debug!("获取用户信息成功: {:?}", login_info);
                                window.set_uid(login_info.uid);

                                ncmapi.save_cookie_jar_to_file();
                                s.imp().ncmapi.replace(Some(ncmapi));

                                sender
                                    .send(Action::InitUserInfo(login_info.to_owned()))
                                    .await
                                    .unwrap();
                                sender
                                    .send(Action::SwitchUserMenuToUser(login_info, user_menu))
                                    .await
                                    .unwrap();
                                sender.send(Action::InitMyPage).await.unwrap();
                                sender
                                    .send(Action::AddToast(gettext("Login successful!")))
                                    .await
                                    .unwrap();
                            }
                            Err(err) => {
                                error!("获取用户信息失败！{:?}", err);
                                sender
                                    .send(Action::AddToast(gettext("Login failed!")))
                                    .await
                                    .unwrap();

                                s.imp().ncmapi.replace(None);
                                NcmClient::clean_cookie_file();
                            }
                        }
                    }
                });
            }
            Action::Logout => {
                let sender = imp.sender.clone();
                let s = self.clone();
                MAINCONTEXT.spawn_local_with_priority(Priority::DEFAULT_IDLE, async move {
                    ncmapi.client.logout().await;

                    s.imp().ncmapi.replace(None);
                    NcmClient::clean_cookie_file();

                    window.logout();
                    window.switch_my_page_to_logout();
                    sender.send(Action::SwitchUserMenuToQr).await.unwrap();
                    sender
                        .send(Action::AddToast(gettext("Logout!")))
                        .await
                        .unwrap();
                });
            }
            Action::InitUserInfo(login_info) => {
                MAINCONTEXT.spawn_local_with_priority(Priority::DEFAULT_IDLE, async move {
                    match ncmapi.client.user_song_id_list(login_info.uid).await {
                        Ok(song_ids) => {
                            window.set_user_like_songs(&song_ids);
                        }
                        Err(err) => error!("{:?}", err),
                    }
                });
            }
            Action::TryUpdateQrCode => {
                if !window.is_logined() && window.is_user_menu_active(UserMenuChild::Qr) {
                    let sender = imp.sender.clone();
                    MAINCONTEXT.spawn_local_with_priority(Priority::DEFAULT_IDLE, async move {
                        if let Ok((path, unikey)) = ncmapi.create_qrcode().await {
                            sender.send(Action::SetQrImage(path)).await.unwrap();
                            sender.send(Action::CheckQrTimeout(unikey)).await.unwrap();
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
                        sender
                            .send_blocking(Action::CheckQrTimeoutCb(unikey))
                            .unwrap();
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
                MAINCONTEXT.spawn_local_with_priority(Priority::DEFAULT_IDLE, async move {
                    let mut send_toast = true;
                    loop {
                        {
                            let key = key.read().unwrap();
                            if *key != unikey {
                                warn!("unikey 已失效，unikey={}", unikey);
                                break;
                            }
                        }
                        match  ncmapi.client.login_qr_check(unikey.to_owned()).await {
                            Ok(msg) => {
                                match msg.code {
                                    // 已过期
                                    800 => {
                                        debug!("二维码已过期，unikey={}", unikey);
                                        sender.send(Action::SetQrImageTimeout).await.unwrap();
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
                                                .await.unwrap();
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
                                        sender.send(Action::CheckLogin(UserMenuChild::Qr, cookie_jar)).await.unwrap();
                                        break;
                                    }
                                    _ => break,
                                }
                            },
                            Err(err) => error!("{:?}", err),
                        }
                        timeout_future_seconds(1).await;
                    }
                });
            }
            Action::SetQrImageTimeout => {
                window.set_user_qrimage_timeout();
            }
            Action::SwitchUserMenuToPhone => {
                window.switch_user_menu_to_phone();
            }
            Action::SwitchUserMenuToQr => {
                window.switch_user_menu_to_qr();
            }
            Action::GetCaptcha(ctcode, phone) => {
                let sender = imp.sender.clone();
                MAINCONTEXT.spawn_local_with_priority(Priority::DEFAULT_IDLE, async move {
                    match ncmapi.client.captcha(ctcode, phone).await {
                        Ok(..) => {
                            debug!("发送获取验证码请求...");
                            sender
                            .send(Action::AddToast(gettext(
                                "Please pay attention to check the cell phone verification code!",
                            )))
                            .await.unwrap();
                        }
                        Err(err) => {
                            warn!("获取验证码失败! {:?}", err);
                            sender
                                .send(Action::AddToast(gettext(
                                    "Failed to get verification code!",
                                )))
                                .await
                                .unwrap();
                        }
                    }
                });
            }
            Action::CaptchaLogin(ctcode, phone, captcha) => {
                let sender = imp.sender.clone();
                MAINCONTEXT.spawn_local_with_priority(Priority::DEFAULT_IDLE, async move {
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
                            .await
                            .unwrap();
                    } else {
                        error!("登录失败！");
                        sender
                            .send(Action::AddToast(gettext("Login failed!")))
                            .await
                            .unwrap();
                    }
                });
            }
            Action::SwitchUserMenuToUser(login_info, menu) => {
                window.switch_user_menu_to_user(login_info.clone(), menu);
                let avatar_url = login_info.avatar_url;
                let mut path = CACHE.clone();
                path.push("avatar.jpg");
                window.set_avatar(avatar_url, path);
            }
            Action::AddToast(mes) => {
                window.add_toast(mes);
            }
            Action::InitCarousel => {
                let sender = imp.sender.clone();
                MAINCONTEXT.spawn_local_with_priority(Priority::DEFAULT_IDLE, async move {
                    match ncmapi.client.banners().await {
                        Ok(banners) => {
                            debug!("获取轮播信息: {:?}", banners);
                            for banner in banners {
                                window.add_carousel(banner);
                            }

                            // auto check login after banners
                            // https://github.com/Binaryify/NeteaseCloudMusicApi/issues/1217
                            if let Some(cookie_jar) = NcmClient::load_cookie_jar_from_file() {
                                sender
                                    .send(Action::CheckLogin(UserMenuChild::Qr, cookie_jar))
                                    .await
                                    .unwrap();
                            }
                        }
                        Err(err) => {
                            error!("获取首页轮播信息失败！{:?}", err);
                            timeout_future(Duration::from_millis(500)).await;
                            sender.send(Action::InitCarousel).await.unwrap();
                        }
                    }
                });
            }
            Action::InitTopPicks => {
                let sender = imp.sender.clone();
                MAINCONTEXT.spawn_local_with_priority(Priority::DEFAULT_IDLE, async move {
                    match ncmapi.client.top_song_list("全部", "hot", 0, 8).await {
                        Ok(song_list) => {
                            debug!("获取热门推荐信息：{:?}", song_list);
                            sender.send(Action::SetupTopPicks(song_list)).await.unwrap();
                        }
                        Err(err) => {
                            error!("获取热门推荐信息失败！{:?}", err);
                            timeout_future(Duration::from_millis(500)).await;
                            sender.send(Action::InitTopPicks).await.unwrap();
                        }
                    }
                });
            }
            Action::ToTopPicksPage => {
                let page = window.init_picks_songlist();
                window.page_new(&page, gettext("all top picks").as_str());
                let page = page.downgrade();

                MAINCONTEXT.spawn_local_with_priority(Priority::DEFAULT_IDLE, async move {
                    if let Some(SearchResult::SongLists(sls)) = window
                        .action_search(ncmapi, String::new(), SearchType::TopPicks, 0, 50)
                        .await
                    {
                        if let Some(page) = page.upgrade() {
                            page.update_songlist(&sls);
                        }
                    }
                });
            }
            Action::DownloadImage(url, path, width, height, callback) => {
                MAINCONTEXT.spawn_local_with_priority(Priority::DEFAULT_IDLE, async move {
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
                window.setup_top_picks(song_list);
            }
            Action::InitNewAlbums => {
                let sender = imp.sender.clone();
                MAINCONTEXT.spawn_local_with_priority(Priority::DEFAULT_IDLE, async move {
                    match ncmapi.client.new_albums("ALL", 0, 8).await {
                        Ok(song_list) => {
                            debug!("获取新碟上架信息：{:?}", song_list);
                            sender
                                .send(Action::SetupNewAlbums(song_list))
                                .await
                                .unwrap();
                        }
                        Err(err) => {
                            error!("获取新碟上架信息失败！{:?}", err);
                            timeout_future(Duration::from_millis(500)).await;
                            sender.send(Action::InitNewAlbums).await.unwrap();
                        }
                    }
                });
            }
            Action::ToAllAlbumsPage => {
                let page = window.init_all_albums();

                window.page_new(&page, gettext("all new albums").as_str());
                let page = page.downgrade();

                MAINCONTEXT.spawn_local_with_priority(Priority::DEFAULT_IDLE, async move {
                    if let Some(SearchResult::SongLists(sls)) = window
                        .action_search(ncmapi, String::new(), SearchType::AllAlbums, 0, 50)
                        .await
                    {
                        if let Some(page) = page.upgrade() {
                            page.update_songlist(&sls);
                        }
                    }
                });
            }
            Action::SetupNewAlbums(song_list) => {
                window.setup_new_albums(song_list);
            }
            Action::BannerTo(banner) => {
                let sender = imp.sender.clone();
                match banner.target_type {
                    TargetType::Song => {
                        MAINCONTEXT.spawn_local_with_priority(Priority::DEFAULT_IDLE, async move {
                            match ncmapi.client.songs_detail(&[banner.target_id]).await {
                                Ok(songs) => {
                                    debug!("获取轮播歌曲信息：{:?}", songs);
                                    if let Some(song) = songs.first() {
                                        sender.send(Action::AddPlay(song.clone())).await.unwrap();
                                    }
                                }
                                Err(err) => {
                                    error!("获取轮播歌曲信息失败！{:?}", err);
                                }
                            }
                        });
                    }
                    TargetType::Album => {
                        MAINCONTEXT.spawn_local_with_priority(Priority::DEFAULT_IDLE, async move {
                            match ncmapi.client.album(banner.target_id).await {
                                Ok(album) => {
                                    debug!("获取轮播专辑信息：{:?}", album);
                                    let page = window.init_songlist_page(
                                        &SongList {
                                            id: album.id,
                                            name: album.name.to_owned(),
                                            cover_img_url: album.pic_url.to_owned(),
                                            author: album.artist_name.to_owned(),
                                        },
                                        true,
                                    );
                                    window.page_new(&page, &album.name);
                                    let page = page.downgrade();
                                    let detal_dynamic_as =
                                        ncmapi.client.album_detail_dynamic(album.id);
                                    let dy = detal_dynamic_as.await.unwrap_or_else(|err| {
                                        error!("{:?}", err);
                                        AlbumDetailDynamic::default()
                                    });
                                    let detail = SongListDetail::Album(album, dy);
                                    if let Some(page) = page.upgrade() {
                                        window.update_songlist_page(page, &detail);
                                    }
                                }
                                Err(err) => {
                                    error!("获取轮播专辑信息失败！{:?}", err);
                                }
                            }
                        });
                    }
                    TargetType::Unknown => (),
                }
            }
            Action::AddPlay(song_info) => {
                window.add_play(song_info.clone());
                let sender = imp.sender.clone();
                sender.send_blocking(Action::Play(song_info)).unwrap();
            }
            Action::PlayNextSong => {
                window.play_next();
            }
            Action::Play(song_info) => {
                let sender = imp.sender.clone();
                let music_rate = window.settings().uint("music-rate");
                let path = crate::path::get_music_cache_path(song_info.id, music_rate);

                if !path.exists() {
                    MAINCONTEXT.spawn_local_with_priority(Priority::DEFAULT_IDLE, async move {
                        if song_info.song_url.is_empty() {
                            if let Ok(song_url) =
                                ncmapi.songs_url(&[song_info.id], music_rate).await
                            {
                                debug!("获取歌曲播放链接: {:?}", song_url);
                                if let Some(song_url) = song_url.first() {
                                    let song_info = SongInfo {
                                        song_url: song_url.url.to_owned(),
                                        ..song_info
                                    };
                                    sender.send(Action::PlayStart(song_info)).await.unwrap();
                                } else {
                                    error!("获取歌曲播放链接失败: {:?}", &[song_info.id]);
                                    sender
                                        .send(Action::AddToast(gettext_f(
                                            "Get [{name}] Playback link failed!",
                                            &[("name", &song_info.name)],
                                        )))
                                        .await
                                        .unwrap();
                                    timeout_future_seconds(2).await;
                                    sender.send(Action::PlayNextSong).await.unwrap();
                                }
                            } else {
                                error!("获取歌曲播放链接失败: {:?}", &[song_info.id]);
                                sender
                                    .send(Action::AddToast(gettext_f(
                                        "Get [{name}] Playback link failed!",
                                        &[("name", &song_info.name)],
                                    )))
                                    .await
                                    .unwrap();
                                timeout_future_seconds(2).await;
                                sender.send(Action::PlayNextSong).await.unwrap();
                            }
                        } else {
                            sender.send(Action::PlayStart(song_info)).await.unwrap();
                        }
                    });
                } else {
                    let song_info = SongInfo {
                        song_url: format!("file://{}", path.to_str().unwrap().to_owned()),
                        ..song_info
                    };
                    sender.send_blocking(Action::PlayStart(song_info)).unwrap();
                }
            }
            Action::PlayStart(song_info) => {
                // 启用桌面歌词
                if window.settings().boolean("desktop-lyrics") {
                    let sender = imp.sender.clone();
                    sender
                        .send_blocking(Action::UpdateLyrics(song_info.to_owned(), 0))
                        .unwrap();
                };
                debug!("播放歌曲: {:?}", song_info);
                window.play(song_info);
            }
            Action::ToSongListPage(songlist) => {
                let page = window.init_songlist_page(&songlist, false);
                window.page_new(&page, &songlist.name);
                let page = page.downgrade();

                let sender = imp.sender.clone();
                MAINCONTEXT.spawn_local_with_priority(Priority::DEFAULT_IDLE, async move {
                    let detal_dynamic_as = ncmapi.client.songlist_detail_dynamic(songlist.id);
                    match ncmapi.client.song_list_detail(songlist.id).await {
                        Ok(detail) => {
                            debug!("获取歌单详情: {:?}", detail);
                            let dy = detal_dynamic_as.await.unwrap_or_else(|err| {
                                error!("{:?}", err);
                                PlayListDetailDynamic::default()
                            });
                            let detail = SongListDetail::PlayList(detail, dy);
                            if let Some(page) = page.upgrade() {
                                window.update_songlist_page(page, &detail);
                            }
                        }
                        Err(err) => {
                            error!("获取歌单详情失败: {:?}", err);
                            sender
                                .send(Action::AddToast(gettext(
                                    "Failed to get song list details!",
                                )))
                                .await
                                .unwrap();
                        }
                    }
                });
            }
            Action::PersistVolume(value) => {
                window.persist_volume(value);
            }
            Action::GetSongUrl(song_info) => {
                let sender = imp.sender.clone();
                let music_rate = window.settings().uint("music-rate");
                let path = crate::path::get_music_cache_path(song_info.id, music_rate);

                if !path.exists() {
                    MAINCONTEXT.spawn_local_with_priority(Priority::DEFAULT_IDLE, async move {
                        if let Ok(song_url) = ncmapi.songs_url(&[song_info.id], music_rate).await {
                            debug!("获取歌曲播放链接: {:?}", song_url);
                            if let Some(song_url) = song_url.first() {
                                let song_info = SongInfo {
                                    song_url: song_url.url.to_owned(),
                                    ..song_info
                                };
                                sender.send(Action::SetSongUrl(song_info)).await.unwrap();
                            }
                        }
                    });
                } else {
                    let song_info = SongInfo {
                        song_url: format!("file://{}", path.to_str().unwrap().to_owned()),
                        ..song_info
                    };
                    window.set_song_url(song_info);
                }
            }
            Action::SetSongUrl(song_info) => {
                window.set_song_url(song_info);
            }
            Action::ToAlbumPage(songlist) => {
                let page = window.init_songlist_page(&songlist, true);
                window.page_new(&page, &songlist.name);
                let page = page.downgrade();

                let sender = imp.sender.clone();
                MAINCONTEXT.spawn_local_with_priority(Priority::DEFAULT_IDLE, async move {
                    let detal_dynamic_as = ncmapi.client.album_detail_dynamic(songlist.id);
                    match ncmapi.client.album(songlist.id).await {
                        Ok(detail) => {
                            debug!("获取专辑详情: {:?}", detail);
                            let dy = detal_dynamic_as.await.unwrap_or_else(|err| {
                                error!("{:?}", err);
                                AlbumDetailDynamic::default()
                            });
                            let detail = SongListDetail::Album(detail, dy);
                            if let Some(page) = page.upgrade() {
                                window.update_songlist_page(page, &detail);
                            }
                        }
                        Err(err) => {
                            error!("获取专辑详情失败: {:?}", err);
                            sender
                                .send(Action::AddToast(gettext("Failed to get album details!")))
                                .await
                                .unwrap();
                        }
                    }
                });
            }
            Action::ToRadioPage(songlist) => {
                let page = window.init_songlist_page(&songlist, true);
                window.page_new(&page, &songlist.name);
                let page = page.downgrade();

                let sender = imp.sender.clone();
                MAINCONTEXT.spawn_local_with_priority(Priority::DEFAULT_IDLE, async move {
                    match ncmapi.client.radio_program(songlist.id, 0, 1001).await {
                        Ok(detail) => {
                            debug!("获取电台详情: {:?}", detail);
                            let detail = SongListDetail::Radio(detail);
                            if let Some(page) = page.upgrade() {
                                window.update_songlist_page(page, &detail);
                            }
                        }
                        Err(err) => {
                            error!("获取电台详情失败: {:?}", err);
                            sender
                                .send(Action::AddToast(gettext("Failed to get radio details!")))
                                .await
                                .unwrap();
                        }
                    }
                });
            }
            Action::AddPlayList(sis, is_play) => {
                window.add_playlist(sis, is_play);
            }
            Action::PlayListStart => {
                window.playlist_start();
            }
            Action::LikeSongList(id, is_like, callback) => {
                let sender = imp.sender.clone();
                MAINCONTEXT.spawn_local_with_priority(Priority::DEFAULT_IDLE, async move {
                    if ncmapi.client.song_list_like(is_like, id).await {
                        debug!("收藏/取消收藏歌单: {:?}", id);
                        if let Some(callback) = callback {
                            callback(());
                        }
                        sender
                            .send(Action::AddToast(if is_like {
                                gettext("Song list have been collected!")
                            } else {
                                gettext("Song list have been uncollected!")
                            }))
                            .await
                            .unwrap();
                    } else {
                        error!("收藏/取消收藏歌单失败: {:?}", id);
                        sender
                            .send(Action::AddToast(if is_like {
                                gettext("Failed to collect song list!")
                            } else {
                                gettext("Failed to uncollect song list!")
                            }))
                            .await
                            .unwrap();
                    }
                });
            }
            Action::LikeAlbum(id, is_like, callback) => {
                let sender = imp.sender.clone();
                MAINCONTEXT.spawn_local_with_priority(Priority::DEFAULT_IDLE, async move {
                    if ncmapi.client.album_like(is_like, id).await {
                        debug!("收藏/取消收藏专辑: {:?}", id);
                        if let Some(callback) = callback {
                            callback(());
                        }
                        sender
                            .send(Action::AddToast(if is_like {
                                gettext("Album have been collected!")
                            } else {
                                gettext("Album have been uncollected!")
                            }))
                            .await
                            .unwrap();
                    } else {
                        error!("收藏/取消收藏专辑失败: {:?}", id);
                        sender
                            .send(Action::AddToast(if is_like {
                                gettext("Failed to collect album!")
                            } else {
                                gettext("Failed to uncollect album!")
                            }))
                            .await
                            .unwrap();
                    }
                });
            }
            Action::LikeSong(id, is_like, callback) => {
                let sender = imp.sender.clone();
                MAINCONTEXT.spawn_local_with_priority(Priority::DEFAULT_IDLE, async move {
                    if ncmapi.client.like(is_like, id).await {
                        debug!("收藏/取消收藏歌曲: {:?}", id);
                        window.set_like_song(id, is_like);
                        sender
                            .send(Action::AddToast(if is_like {
                                gettext("Songs have been collected!")
                            } else {
                                gettext("Songs have been uncollected!")
                            }))
                            .await
                            .unwrap();
                        if let Some(callback) = callback {
                            callback(());
                        }
                    } else {
                        error!("收藏/取消收藏歌曲失败: {:?}", id);
                        sender
                            .send(Action::AddToast(if is_like {
                                gettext("Failed to collect songs!")
                            } else {
                                gettext("Failed to uncollect songs!")
                            }))
                            .await
                            .unwrap();
                    }
                });
            }
            Action::Moved(si) => {
                let sender = imp.sender.clone();
                MAINCONTEXT.spawn_local_with_priority(Priority::DEFAULT_IDLE, async move {
                    match ncmapi
                        .client
                        .playmode_intelligence_list(si.id, si.album_id)
                        .await
                    {
                        Ok(mut pl) => {
                            debug!("获取心动歌曲：{:?}", pl);
                            let mut pla = vec![si];
                            pla.append(&mut pl);

                            sender.send(Action::AddPlayList(pla, false)).await.unwrap();
                            sender
                                .send(Action::AddToast(gettext("Intelligent Mode")))
                                .await
                                .unwrap();
                        }
                        Err(err) => {
                            error!("获取心动歌曲 {} 失败! {:?}", si.name, err);
                            sender
                                .send(Action::AddToast(gettext("Intelligent mode failed!")))
                                .await
                                .unwrap();
                        }
                    }
                });
            }
            Action::GetToplist => {
                let sender = imp.sender.clone();
                MAINCONTEXT.spawn_local_with_priority(Priority::DEFAULT_IDLE, async move {
                    match ncmapi.client.toplist().await {
                        Ok(toplist) => {
                            debug!("获取排行榜: {:?}", toplist);
                            sender.send(Action::InitTopList(toplist)).await.unwrap();
                        }
                        Err(err) => {
                            error!("获取排行榜失败! {:?}", err);
                            timeout_future(Duration::from_millis(500)).await;
                            sender.send(Action::GetToplist).await.unwrap();
                        }
                    }
                });
            }
            Action::GetToplistSongsList(id) => {
                let sender = imp.sender.clone();
                MAINCONTEXT.spawn_local_with_priority(Priority::DEFAULT_IDLE, async move {
                    match ncmapi.client.song_list_detail(id).await {
                        Ok(detail) => {
                            debug!("获取榜单 {} 详情：{:?}", id, detail);
                            sender
                                .send(Action::UpdateTopList(detail.songs))
                                .await
                                .unwrap();
                        }
                        Err(err) => {
                            error!("获取榜单 {} 失败! {:?}", id, err);
                            sender
                                .send(Action::AddToast(gettext(
                                    "Request for interface failed, please try again!",
                                )))
                                .await
                                .unwrap();
                        }
                    }
                });
            }
            Action::InitTopList(toplist) => {
                window.init_toplist(toplist);
            }
            Action::UpdateTopList(sis) => {
                window.update_toplist(sis);
            }
            Action::Search(text, search_type, offset, limit, callback) => {
                MAINCONTEXT.spawn_local_with_priority(Priority::DEFAULT_IDLE, async move {
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
                MAINCONTEXT.spawn_local_with_priority(Priority::DEFAULT_IDLE, async move {
                    match ncmapi.client.singer_songs(singer.id).await {
                        Ok(sis) => {
                            debug!("获取歌手单曲：{:?}", sis);
                            if let Some(page) = page.upgrade() {
                                window.update_search_song_page(page, sis);
                            }
                        }
                        Err(err) => {
                            error!("{:?}", err);
                            sender
                                .send(Action::AddToast(gettext(
                                    "Request for interface failed, please try again!",
                                )))
                                .await
                                .unwrap();
                        }
                    }
                });
            }
            Action::ToMyPageDailyRec => {
                let title = gettext("Daily Recommendation");
                let page = window.init_search_song_page(&title, SearchType::DailyRec);
                window.page_new(&page, &title);
                let page = page.downgrade();

                let sender = imp.sender.clone();
                MAINCONTEXT.spawn_local_with_priority(Priority::DEFAULT_IDLE, async move {
                    match ncmapi.client.recommend_songs().await {
                        Ok(sis) => {
                            debug!("获取每日推荐：{:?}", sis);
                            if let Some(page) = page.upgrade() {
                                window.update_search_song_page(page, sis);
                            }
                        }
                        Err(err) => {
                            error!("{:?}", err);
                            sender
                                .send(Action::AddToast(gettext(
                                    "Request for interface failed, please try again!",
                                )))
                                .await
                                .unwrap();
                        }
                    }
                });
            }
            Action::ToMyPageHeartbeat => {
                let title = gettext("Favorite Songs");
                let page = window.init_search_song_page(&title, SearchType::Heartbeat);
                window.page_new(&page, &title);
                let page = page.downgrade();

                let sender = imp.sender.clone();
                MAINCONTEXT.spawn_local_with_priority(Priority::DEFAULT_IDLE, async move {
                    let uid = window.get_uid();
                    match ncmapi.client.user_song_list(uid, 0, 1).await {
                        Ok(sls) => {
                            debug!("获取心动歌单：{:?}", sls);
                            if !sls.is_empty() {
                                match ncmapi.client.song_list_detail(sls[0].id).await {
                                    Ok(detail) => {
                                        if let Some(page) = page.upgrade() {
                                            window.update_search_song_page(page, detail.songs);
                                        }
                                    }
                                    Err(err) => {
                                        error!("{:?}", err);
                                        sender
                                            .send(Action::AddToast(gettext(
                                                "Failed to get song list details!",
                                            )))
                                            .await
                                            .unwrap();
                                    }
                                }
                            }
                        }
                        Err(err) => {
                            error!("{:?}", err);
                            sender
                                .send(Action::AddToast(gettext(
                                    "Request for interface failed, please try again!",
                                )))
                                .await
                                .unwrap();
                        }
                    }
                });
            }
            Action::ToMyPageCloudDisk => {
                let title = gettext("Cloud Music");
                let page = window.init_search_song_page(&title, SearchType::CloudDisk);
                window.page_new(&page, &title);
                let page = page.downgrade();

                let sender = imp.sender.clone();
                MAINCONTEXT.spawn_local_with_priority(Priority::DEFAULT_IDLE, async move {
                    match ncmapi.client.user_cloud_disk().await {
                        Ok(sis) => {
                            debug!("获取云盘音乐：{:?}", sis);
                            if let Some(page) = page.upgrade() {
                                window.update_search_song_page(page, sis);
                            }
                        }
                        Err(err) => {
                            error!("{:?}", err);
                            sender
                                .send(Action::AddToast(gettext(
                                    "Request for interface failed, please try again!",
                                )))
                                .await
                                .unwrap();
                        }
                    }
                });
            }
            Action::ToMyPageRadio => {
                let title = gettext("My Radio");
                let page = window.init_search_songlist_page(&title, SearchType::Radio);
                window.page_new(&page, &title);
                let page = page.downgrade();

                MAINCONTEXT.spawn_local_with_priority(Priority::DEFAULT_IDLE, async move {
                    let res = window
                        .action_search(ncmapi, String::new(), SearchType::Radio, 0, 1001)
                        .await;
                    if let Some(page) = page.upgrade() {
                        if let Some(SearchResult::SongLists(sls)) = res {
                            page.update_songlist(&sls);
                        }
                    }
                });
            }
            Action::ToMyPageAlbums => {
                let title = gettext("Favorite Album");
                let page = window.init_search_songlist_page(&title, SearchType::LikeAlbums);
                window.page_new(&page, &title);
                let page = page.downgrade();

                MAINCONTEXT.spawn_local_with_priority(Priority::DEFAULT_IDLE, async move {
                    let res = window
                        .action_search(ncmapi, String::new(), SearchType::LikeAlbums, 0, 50)
                        .await;
                    if let Some(page) = page.upgrade() {
                        if let Some(SearchResult::SongLists(sls)) = res {
                            page.update_songlist(&sls);
                        }
                    }
                });
            }
            Action::ToMyPageSonglist => {
                let title = gettext("Favorite Song List");
                let page = window.init_search_songlist_page(&title, SearchType::LikeSongList);
                window.page_new(&page, &title);
                let page = page.downgrade();

                MAINCONTEXT.spawn_local_with_priority(Priority::DEFAULT_IDLE, async move {
                    let res = window
                        .action_search(ncmapi, String::new(), SearchType::LikeSongList, 0, 1001)
                        .await;
                    if let Some(page) = page.upgrade() {
                        if let Some(SearchResult::SongLists(sls)) = res {
                            page.update_songlist(&sls[1..]);
                        }
                    }
                });
            }
            Action::InitMyPage => {
                window.switch_my_page_to_login();
                let sender = imp.sender.clone();
                MAINCONTEXT.spawn_local_with_priority(Priority::DEFAULT_IDLE, async move {
                    match ncmapi.client.recommend_resource().await {
                        Ok(sls) => {
                            debug!("获取推荐歌单：{:?}", sls);
                            sender
                                .send(Action::InitMyPageRecSongList(sls))
                                .await
                                .unwrap();
                        }
                        Err(err) => {
                            error!("{:?}", err);
                            sender.send(Action::InitMyPage).await.unwrap();
                        }
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
                } else {
                    sender.send_blocking(Action::PageBack).unwrap();
                }
            }
            Action::UpdateLyrics(si, time) => {
                MAINCONTEXT.spawn_local_with_priority(Priority::DEFAULT_IDLE, async move {
                    if time == 0 {
                        // 当新曲目播放时，写入歌词内容
                        window.update_lyrics_text(&gettext("Loading lyrics..."));
                        match ncmapi.get_lyrics(si).await {
                            Ok(lrc) => {
                                debug!("获取歌词：{:?}", lrc);
                                window.update_lyrics(lrc);
                            }
                            Err(e) => debug!("{}", e),
                        }
                    }
                    // 更新歌词高亮位置
                    window.update_lyrics_timestamp(time);
                });
            }
            Action::UpdatePlayListStatus(index) => {
                window.update_playlist_status(index);
            }
            Action::RemoveFromPlayList(song_info) => {
                MAINCONTEXT.spawn_local_with_priority(Priority::DEFAULT_IDLE, async move {
                    window.remove_from_playlist(song_info);
                });
            }
            Action::GstDurationChanged(sec) => {
                window.gst_duration_changed(sec);
            }
            Action::GstStateChanged(state) => {
                window.gst_state_changed(state);
            }
            Action::GstVolumeChanged(volume) => {
                window.gst_volume_changed(volume);
            }
            Action::GstCacheDownloadComplete(loc) => {
                window.gst_cache_download_complete(loc);
            }
            Action::ScaleSeekUpdate(sec) => {
                window.scale_seek_update(sec);
            }
            Action::ScaleValueUpdate => {
                window.scale_value_update();
            }

            Action::PageBack => {
                window.page_back();
            }
            Action::InitMpris(mpris) => {
                window.init_mpris(mpris);
            }
        }
        glib::ControlFlow::Continue
    }

    fn setup_gactions(&self) {
        let preferences_action = gio::SimpleAction::new("preferences", None);
        preferences_action.connect_activate(clone!(
            #[weak(rename_to = app)]
            self,
            move |_, _| {
                app.show_prefrerences();
            }
        ));
        self.add_action(&preferences_action);

        let quit_action = gio::SimpleAction::new("quit", None);
        quit_action.connect_activate(clone!(
            #[weak(rename_to = app)]
            self,
            move |_, _| {
                app.quit();
            }
        ));
        self.add_action(&quit_action);

        let about_action = gio::SimpleAction::new("about", None);
        about_action.connect_activate(clone!(
            #[weak(rename_to = app)]
            self,
            move |_, _| {
                app.show_about();
            }
        ));
        self.add_action(&about_action);
    }

    fn show_prefrerences(&self) {
        let window = self.active_window().unwrap();
        let preferences = NeteaseCloudMusicGtk4Preferences::new();

        let (size, unit) = crate::path::get_cache_size();
        preferences.set_cache_size_label(size, unit);

        preferences.present(Some(&window));
    }

    fn show_about(&self) {
        let window = self.active_window().unwrap();
        let dialog = gtk::AboutDialog::builder()
            .transient_for(&window)
            .modal(true)
            .program_name(gettext(crate::APP_NAME))
            .logo_icon_name("logo")
            .version(VERSION)
            .authors(vec!["gmg137", "catsout"])
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
        MAINCONTEXT.spawn_local_with_priority(Priority::LOW, async move {
            match cache_clear {
                1 => {
                    if remove_all_file(cache_path).is_ok() {
                        sender
                            .send(Action::AddToast(gettext("Cache cleared.")))
                            .await
                            .unwrap();
                    }
                }
                2 => {
                    if let Ok(datetime) = glib::DateTime::now_local() {
                        if datetime.day_of_week() == 1 && !flag {
                            if remove_all_file(cache_path).is_ok() {
                                sender
                                    .send(Action::AddToast(gettext("Cache cleared.")))
                                    .await
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
                                    .await
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
