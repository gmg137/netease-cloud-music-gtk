use crate::{
    application::{Action, NeteaseCloudMusicGtk4Application},
    audio::MprisController,
    gui::*,
    model::*,
    ncmapi::NcmClient,
};
use adw::{ColorScheme, StyleManager, Toast};
use async_channel::Sender;
use gettextrs::gettext;
use gio::{Settings, SimpleAction};
use glib::{
    clone, source::Priority, ParamSpec, ParamSpecEnum, ParamSpecObject, ParamSpecUInt64, Value,
};
use gtk::{
    gio::{self, SettingsBindFlags},
    glib, CompositeTemplate,
};
use log::*;
use ncm_api::{BannersInfo, LoginInfo, SongInfo, SongList, TopList};
use once_cell::sync::{Lazy, OnceCell};
use std::{
    cell::{Cell, RefCell},
    collections::LinkedList,
    path::PathBuf,
    sync::{Arc, Mutex},
};

mod imp {

    use super::*;

    #[derive(Default, CompositeTemplate)]
    #[template(resource = "/com/gitee/gmg137/NeteaseCloudMusicGtk4/gtk/window.ui")]
    pub struct NeteaseCloudMusicGtk4Window {
        #[template_child]
        pub header_bar: TemplateChild<adw::HeaderBar>,
        #[template_child]
        pub gbox: TemplateChild<Box>,
        #[template_child]
        pub toast_overlay: TemplateChild<adw::ToastOverlay>,
        #[template_child]
        pub base_stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub stack: TemplateChild<adw::ViewStack>,
        #[template_child]
        pub back_button: TemplateChild<Button>,
        #[template_child]
        pub search_button: TemplateChild<ToggleButton>,
        #[template_child]
        pub search_bar: TemplateChild<SearchBar>,
        #[template_child]
        pub search_entry: TemplateChild<SearchEntry>,
        #[template_child]
        pub search_menu: TemplateChild<MenuButton>,
        #[template_child]
        pub primary_menu_button: TemplateChild<MenuButton>,
        #[template_child]
        pub switcher_title: TemplateChild<adw::ViewSwitcher>,
        #[template_child]
        pub label_title: TemplateChild<Label>,
        #[template_child]
        pub user_button: TemplateChild<MenuButton>,
        #[template_child]
        pub player_revealer: TemplateChild<Revealer>,
        #[template_child]
        pub player_controls: TemplateChild<PlayerControls>,
        #[template_child]
        pub toplist: TemplateChild<TopListView>,
        #[template_child]
        pub discover: TemplateChild<Discover>,
        #[template_child]
        pub my_stack: TemplateChild<adw::ViewStack>,
        #[template_child]
        pub my_page: TemplateChild<MyPage>,

        pub playlist_lyrics_page: OnceCell<PlayListLyricsPage>,

        pub user_menus: OnceCell<UserMenus>,
        pub popover_menu: OnceCell<PopoverMenu>,
        pub settings: OnceCell<Settings>,
        pub sender: OnceCell<Sender<Action>>,
        pub stack_child: Arc<Mutex<LinkedList<(String, String)>>>,
        pub page_stack: OnceCell<PageStack>,

        search_type: Cell<SearchType>,
        toast: RefCell<Option<Toast>>,
        user_info: RefCell<UserInfo>,
    }

    impl NeteaseCloudMusicGtk4Window {
        pub fn user_like_song_contains(&self, id: &u64) -> bool {
            self.user_info.borrow().like_songs.contains(id)
        }
        pub fn user_like_song_add(&self, id: u64) {
            self.user_info.borrow_mut().like_songs.insert(id);
        }
        pub fn user_like_song_remove(&self, id: &u64) {
            self.user_info.borrow_mut().like_songs.remove(id);
        }
        pub fn clear_user_info(&self) {
            self.user_info.take();
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for NeteaseCloudMusicGtk4Window {
        const NAME: &'static str = "NeteaseCloudMusicGtk4Window";
        type Type = super::NeteaseCloudMusicGtk4Window;
        type ParentType = ApplicationWindow;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_instance_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for NeteaseCloudMusicGtk4Window {
        fn constructed(&self) {
            let obj = self.obj();
            self.parent_constructed();

            self.page_stack
                .set(PageStack::new(self.base_stack.get()))
                .unwrap();

            self.playlist_lyrics_page
                .set(PlayListLyricsPage::new())
                .unwrap();

            if let Ok(mut stack_child) = self.stack_child.lock() {
                stack_child.push_back(("discover".to_owned(), "".to_owned()));
            }

            self.toast.replace(Some(Toast::new("")));

            obj.setup_settings();
            obj.bind_settings();
        }

        fn properties() -> &'static [ParamSpec] {
            static PROPERTIES: Lazy<Vec<ParamSpec>> = Lazy::new(|| {
                vec![
                    ParamSpecEnum::builder::<SearchType>("search-type")
                        .explicit_notify()
                        .build(),
                    ParamSpecObject::builder::<Toast>("toast").build(),
                    ParamSpecUInt64::builder("uid").build(),
                ]
            });
            PROPERTIES.as_ref()
        }

        fn set_property(&self, _id: usize, value: &Value, pspec: &ParamSpec) {
            match pspec.name() {
                "toast" => {
                    let toast = value.get().unwrap();
                    self.toast.replace(toast);
                }
                "search-type" => {
                    let input_type = value
                        .get()
                        .expect("The value needs to be of type `SearchType`.");
                    self.search_type.replace(input_type);
                }
                "uid" => {
                    let uid = value.get().unwrap();
                    self.user_info.borrow_mut().uid = uid;
                }
                _ => unimplemented!(),
            }
        }

        fn property(&self, _id: usize, pspec: &ParamSpec) -> Value {
            match pspec.name() {
                "toast" => self.toast.borrow().to_value(),
                "search-type" => self.search_type.get().to_value(),
                "uid" => self.user_info.borrow().uid.to_value(),
                _ => unimplemented!(),
            }
        }
    }
    impl WidgetImpl for NeteaseCloudMusicGtk4Window {}
    impl WindowImpl for NeteaseCloudMusicGtk4Window {}
    impl ApplicationWindowImpl for NeteaseCloudMusicGtk4Window {}
}

glib::wrapper! {
    pub struct NeteaseCloudMusicGtk4Window(ObjectSubclass<imp::NeteaseCloudMusicGtk4Window>)
        @extends gtk::Widget, gtk::Window, gtk::ApplicationWindow,
        @implements gio::ActionGroup, gio::ActionMap;
}

impl NeteaseCloudMusicGtk4Window {
    pub fn new<P: glib::object::IsA<gtk::Application>>(
        application: &P,
        sender: Sender<Action>,
    ) -> Self {
        let window: NeteaseCloudMusicGtk4Window = glib::Object::builder()
            .property("application", application)
            .build();

        window.imp().sender.set(sender).unwrap();
        window.setup_widget();
        window.setup_action();
        window.init_page_data();
        window
    }

    fn setup_settings(&self) {
        let settings = Settings::new(crate::APP_ID);
        self.imp()
            .settings
            .set(settings)
            .expect("Could not set `Settings`.");
    }

    pub fn settings(&self) -> &Settings {
        self.imp().settings.get().expect("Could not get settings.")
    }

    fn setup_action(&self) {
        let imp = self.imp();
        let sender_ = imp.sender.get().unwrap().clone();
        // 监测用户菜单弹出
        let popover = imp.popover_menu.get().unwrap();
        let sender = sender_.clone();
        popover.connect_child_notify(move |_| {
            sender.send_blocking(Action::TryUpdateQrCode).unwrap();
        });
        let sender = sender_.clone();
        popover.connect_show(move |_| {
            sender.send_blocking(Action::TryUpdateQrCode).unwrap();
        });

        // 绑定设置与主题
        let action_style = self.settings().create_action("style-variant");
        self.add_action(&action_style);

        // 绑定搜索按钮和搜索栏
        let search_button = imp.search_button.get();
        // let search_bar = imp.search_bar.get();
        // search_button
        //     .bind_property("active", &search_bar, "search-mode-enabled")
        //     .flags(BindingFlags::BIDIRECTIONAL)
        //     .build();
        let search_entry = imp.search_entry.get();

        // 设置搜索动作
        let action_search = SimpleAction::new("search-button", None);
        action_search.connect_activate(clone!(
            #[weak]
            search_button,
            move |_, _| {
                search_button.emit_clicked();
            }
        ));
        self.add_action(&action_search);

        let search_bar = imp.search_bar.get();
        search_bar.connect_search_mode_enabled_notify(clone!(
            #[weak]
            search_entry,
            move |bar| {
                if bar.is_search_mode() {
                    // 清空搜索框
                    search_entry.set_text("");
                    // 使搜索框获取输入焦点
                    search_entry.grab_focus();
                }
            }
        ));

        // 设置返回键功能
        let action_back = SimpleAction::new("back-button", None);
        self.add_action(&action_back);

        let sender = sender_;
        action_back.connect_activate(move |_, _| {
            sender.send_blocking(Action::PageBack).unwrap();
        });
    }

    fn bind_settings(&self) {
        let style = StyleManager::default();
        self.settings()
            .bind("style-variant", &style, "color-scheme")
            .mapping(|themes, _| {
                let themes = themes
                    .get::<String>()
                    .expect("The variant needs to be of type `String`.");
                let scheme = match themes.as_str() {
                    "system" => ColorScheme::Default,
                    "light" => ColorScheme::ForceLight,
                    "dark" => ColorScheme::ForceDark,
                    _ => ColorScheme::Default,
                };
                Some(scheme.to_value())
            })
            .build();

        self.settings()
            .bind("exit-switch", self, "hide-on-close")
            .flags(SettingsBindFlags::DEFAULT)
            .build();
    }

    fn setup_widget(&self) {
        let imp = self.imp();
        let sender = imp.sender.get().unwrap();
        let primary_menu_button = imp.primary_menu_button.get();
        let popover = primary_menu_button.popover().unwrap();
        let popover = popover.downcast::<gtk::PopoverMenu>().unwrap();
        let theme = crate::gui::ThemeSelector::new();
        popover.add_child(&theme, "theme");

        let user_menus = UserMenus::new(sender.clone());

        let user_button = imp.user_button.get();
        let popover = user_button.popover().unwrap();
        let popover = popover.downcast::<PopoverMenu>().unwrap();
        popover.add_child(&user_menus.qrbox, "user_popover");

        imp.user_menus.set(user_menus).unwrap();
        imp.popover_menu.set(popover).unwrap();
    }

    pub fn get_uid(&self) -> u64 {
        self.property::<u64>("uid")
    }

    pub fn set_uid(&self, val: u64) {
        self.set_property("uid", val);
    }

    pub fn is_logined(&self) -> bool {
        self.get_uid() != 0u64
    }

    pub fn logout(&self) {
        self.imp().clear_user_info();
    }

    pub fn get_song_likes(&self, sis: &[SongInfo]) -> Vec<bool> {
        sis.iter()
            .map(|si| self.imp().user_like_song_contains(&si.id))
            .collect()
    }

    pub fn set_like_song(&self, id: u64, val: bool) {
        let imp = self.imp();
        if let Some(song) = imp.player_controls.get().get_current_song() {
            if song.id == id {
                imp.player_controls.get().set_property("like", val);
            }
        }

        if val {
            imp.user_like_song_add(id);
        } else {
            imp.user_like_song_remove(&id);
        }
    }

    pub fn set_user_like_songs(&self, song_ids: &[u64]) {
        song_ids
            .iter()
            .for_each(|id| self.imp().user_like_song_add(id.to_owned()));
    }

    pub fn set_user_qrimage(&self, path: PathBuf) {
        let user_menus = self.imp().user_menus.get().unwrap();
        user_menus.set_qrimage(path);
    }

    pub fn set_user_qrimage_timeout(&self) {
        let user_menus = self.imp().user_menus.get().unwrap();
        user_menus.set_qrimage_timeout();
    }

    pub fn is_user_menu_active(&self, menu: UserMenuChild) -> bool {
        self.imp().user_menus.get().unwrap().is_menu_active(menu)
    }

    pub fn switch_user_menu_to_phone(&self) {
        let popover = self.imp().popover_menu.get().unwrap();
        let user_menus = self.imp().user_menus.get().unwrap();
        user_menus.switch_menu(UserMenuChild::Phone, popover);
    }

    pub fn switch_user_menu_to_qr(&self) {
        let popover = self.imp().popover_menu.get().unwrap();
        let user_menus = self.imp().user_menus.get().unwrap();
        user_menus.switch_menu(UserMenuChild::Qr, popover);
    }

    pub fn switch_user_menu_to_user(&self, login_info: LoginInfo, _menu: UserMenuChild) {
        let popover = self.imp().popover_menu.get().unwrap();
        let user_menus = self.imp().user_menus.get().unwrap();
        user_menus.switch_menu(UserMenuChild::User, popover);
        if login_info.vip_type == 0 {
            user_menus.set_user_name(login_info.nickname);
        } else {
            user_menus.set_user_name(format!("👑{}", login_info.nickname));
        }
    }

    pub fn set_avatar(&self, url: String, path: PathBuf) {
        self.imp()
            .user_menus
            .get()
            .unwrap()
            .set_user_avatar(url, path);
    }

    pub fn add_toast(&self, mes: String) {
        let pre = self.property::<Toast>("toast");

        let toast = Toast::builder()
            .title(glib::markup_escape_text(&mes))
            .priority(adw::ToastPriority::High)
            .build();
        self.set_property("toast", &toast);
        self.imp().toast_overlay.add_toast(toast);

        // seems that dismiss will clear something used by animation
        // cause adw_animation_skip emit 'done' segfault on closure(https://github.com/gmg137/netease-cloud-music-gtk/issues/202)
        // delay to wait for animation skipped/done
        crate::MAINCONTEXT.spawn_local_with_priority(Priority::DEFAULT_IDLE, async move {
            glib::timeout_future(std::time::Duration::from_millis(500)).await;
            // removed from overlay toast queue by signal
            pre.dismiss();
        });
    }

    pub fn add_carousel(&self, banner: BannersInfo) {
        let discover = self.imp().discover.get();
        discover.add_carousel(banner);
    }

    pub fn setup_top_picks(&self, song_list: Vec<SongList>) {
        let discover = self.imp().discover.get();
        discover.setup_top_picks(song_list);
    }

    pub fn setup_new_albums(&self, song_list: Vec<SongList>) {
        let discover = self.imp().discover.get();
        discover.setup_new_albums(song_list);
    }

    pub fn add_play(&self, song_info: SongInfo) {
        let player_controls = self.imp().player_controls.get();
        player_controls.add_song(song_info);
    }

    pub fn remove_from_playlist(&self, song_info: SongInfo) {
        let player_controls = self.imp().player_controls.get();
        player_controls.remove_song(song_info);

        let sis = player_controls.get_list();
        let si = player_controls.get_current_song().unwrap_or(SongInfo {
            id: 0,
            name: String::new(),
            singer: String::new(),
            album: String::new(),
            album_id: 0,
            pic_url: String::new(),
            duration: 0,
            song_url: String::new(),
            copyright: ncm_api::SongCopyright::Unknown,
        });

        self.init_playlist_lyrics_page(sis, si.to_owned());

        if si.id == 0 {
            let player_revealer = self.imp().player_revealer.get();
            player_revealer.set_reveal_child(false);
            player_revealer.set_visible(false);
            player_revealer.set_reveal_child(false);
            let sender = self.imp().sender.get().unwrap();
            sender.send_blocking(Action::PageBack).unwrap();
        }
    }

    pub fn add_playlist(&self, sis: Vec<SongInfo>, is_play: bool) {
        let player_controls = self.imp().player_controls.get();
        player_controls.add_list(sis);
        let sender = self.imp().sender.get().unwrap();
        if is_play {
            sender.send_blocking(Action::PlayListStart).unwrap();
        }
    }

    pub fn playlist_start(&self) {
        let sender = self.imp().sender.get().unwrap();
        let player_controls = self.imp().player_controls.get();
        if let Some(song_info) = player_controls.get_current_song() {
            sender.send_blocking(Action::Play(song_info)).unwrap();
            return;
        }
        sender
            .send_blocking(Action::AddToast(gettext("No playable songs found！")))
            .unwrap();
    }

    pub fn play_next(&self) {
        let player_controls = self.imp().player_controls.get();
        player_controls.next_song();
    }

    pub fn play(&self, song_info: SongInfo) {
        let player_controls = self.imp().player_controls.get();
        player_controls.set_property("like", self.imp().user_like_song_contains(&song_info.id));
        player_controls.play(song_info);
        let player_revealer = self.imp().player_revealer.get();
        if !player_revealer.reveals_child() {
            player_revealer.set_visible(true);
            player_revealer.set_reveal_child(true);
        }
    }

    pub fn init_page_data(&self) {
        let imp = self.imp();
        let sender = imp.sender.get().unwrap();

        // 初始化我的页面
        let my_page = imp.my_page.get();
        my_page.set_sender(sender.clone());

        // 初始化播放栏
        let player_controls = imp.player_controls.get();
        player_controls.set_sender(sender.clone());

        // 初始化发现页
        let discover = imp.discover.get();
        discover.set_sender(sender.clone());
        discover.init_page();

        // 初始化榜单
        sender.send_blocking(Action::GetToplist).unwrap();
        let toplist = imp.toplist.get();
        toplist.set_sender(sender.clone());

        // 初始化播放列表页
        let playlist_lyrics_page = imp.playlist_lyrics_page.get().unwrap();
        playlist_lyrics_page.set_sender(sender.clone());

        let page_stack = imp.page_stack.get().unwrap();
        page_stack.set_transition_type(StackTransitionType::Crossfade);
        page_stack.set_transition_duration(100); // default 200
    }

    pub fn init_toplist(&self, list: Vec<TopList>) {
        let toplist = self.imp().toplist.get();
        toplist.init_sidebar(list);
    }

    pub fn update_toplist(&self, list: Vec<SongInfo>) {
        let toplist = self.imp().toplist.get();
        toplist.update_songs_list(
            &list,
            &list
                .iter()
                .map(|si| self.imp().user_like_song_contains(&si.id))
                .collect::<Vec<bool>>(),
        );
    }

    // page routing
    fn page_widget_switch(&self, need_back: bool) {
        let imp = self.imp();
        let switcher_title = imp.switcher_title.get();
        let label_title = imp.label_title.get();
        let back_button = imp.back_button.get();

        let visible = need_back;
        back_button.set_visible(visible);
        label_title.set_visible(visible);
        switcher_title.set_visible(!visible);
    }
    pub fn page_set_info(&self, title: &str) {
        let imp = self.imp();
        let label_title = imp.label_title.get();

        label_title.set_label(title);
    }
    // same name will clear old page
    pub fn page_new_with_name(
        &self,
        name: &str,
        page: &impl glib::object::IsA<Widget>,
        title: &str,
    ) {
        let imp = self.imp();
        let stack = imp.page_stack.get().unwrap();
        // stack.set_transition_type(StackTransitionType::SlideLeft);
        let stack_page = stack.new_page_with_name(page, name);
        stack_page.set_title(title);
        self.page_set_info(title);
        self.page_widget_switch(true);
    }
    pub fn page_new(&self, page: &impl glib::object::IsA<Widget>, title: &str) {
        let imp = self.imp();
        let stack = imp.page_stack.get().unwrap();
        if stack.len() > 1 {
            let top_page = stack.top_page();
            if top_page.title().unwrap() == title {
                return;
            }
        }
        // stack.set_transition_type(StackTransitionType::SlideLeft);
        let stack_page = stack.new_page(page);
        stack_page.set_title(title);
        self.page_set_info(title);
        self.page_widget_switch(true);
    }
    pub fn page_back(&self) -> Option<Widget> {
        let imp = self.imp();
        let stack = imp.page_stack.get().unwrap();

        // stack.set_transition_type(StackTransitionType::UnderRight);
        stack.back_page();

        if stack.len() > 1 {
            let top_page = stack.top_page();
            self.page_set_info(top_page.title().unwrap().to_string().as_str());
            self.page_widget_switch(true);
        } else {
            self.page_widget_switch(false);
        }
        None
    }
    pub fn persist_volume(&self, value: f64) {
        let imp = self.imp();
        imp.player_controls.persist_volume(value);
    }
    pub fn page_cur_playlist_lyrics_page(&self) -> bool {
        let imp = self.imp();
        let page = imp.playlist_lyrics_page.get().unwrap();
        let cur = &imp.page_stack.get().unwrap().top_page().child();
        cur == page
    }

    pub fn init_picks_songlist(&self) -> SearchSongListPage {
        let imp = self.imp();
        let sender = imp.sender.get().unwrap().clone();
        let page = SearchSongListPage::new();
        page.set_sender(sender);
        page.init_page("全部歌单", SearchType::TopPicks);
        page
    }

    pub fn init_all_albums(&self) -> SearchSongListPage {
        let imp = self.imp();
        let sender = imp.sender.get().unwrap().clone();
        let page = SearchSongListPage::new();
        page.set_sender(sender);
        page.init_page("全部新碟", SearchType::AllAlbums);
        page
    }

    pub fn init_search_song_page(&self, text: &str, search_type: SearchType) -> SearchSongPage {
        let imp = self.imp();
        let sender = imp.sender.get().unwrap().clone();
        let page = SearchSongPage::new();
        page.set_sender(sender);
        page.init_page(text, search_type);
        page
    }

    pub fn init_search_songlist_page(
        &self,
        text: &str,
        search_type: SearchType,
    ) -> SearchSongListPage {
        let imp = self.imp();
        let sender = imp.sender.get().unwrap().clone();
        let page = SearchSongListPage::new();
        page.set_sender(sender);
        page.init_page(text, search_type);
        page
    }
    pub fn init_search_singer_page(&self, text: &str) -> SearchSingerPage {
        let imp = self.imp();
        let sender = imp.sender.get().unwrap().clone();
        let page = SearchSingerPage::new();
        page.set_sender(sender);
        page.init_page(text.to_string());
        page
    }

    pub fn init_songlist_page(&self, songlist: &SongList, is_album: bool) -> SonglistPage {
        let sender = self.imp().sender.get().unwrap().clone();
        let page = SonglistPage::new();
        page.set_sender(sender);
        page.init_songlist_info(songlist, is_album, self.is_logined());
        page
    }

    pub fn update_search_song_page(&self, page: SearchSongPage, sis: Vec<SongInfo>) {
        page.update_songs(&sis, &self.get_song_likes(&sis));
    }

    pub fn update_songlist_page(&self, page: SonglistPage, detail: &SongListDetail) {
        page.init_songlist(detail, &self.get_song_likes(detail.sis()));
    }

    pub fn switch_my_page_to_login(&self) {
        let imp = self.imp();
        imp.my_stack.set_visible_child_name("my_login");
    }

    pub fn switch_my_page_to_logout(&self) {
        let imp = self.imp();
        imp.my_stack.set_visible_child_name("my_no_login");
    }

    pub fn init_my_page(&self, sls: Vec<SongList>) {
        self.imp().my_page.init_page(sls);
    }

    pub fn init_playlist_lyrics_page(&self, sis: Vec<SongInfo>, si: SongInfo) {
        let imp = self.imp();
        let page = imp.playlist_lyrics_page.get().unwrap();
        page.init_page(&sis, si, &self.get_song_likes(&sis));

        self.page_new(page, &gettext("Play List&Lyrics"));
    }

    /// 更新歌词内容，不调整位置
    pub fn update_lyrics(&self, lrc: Vec<(u64, String)>) {
        let imp = self.imp();
        let page = imp.playlist_lyrics_page.get().unwrap();
        page.update_lyrics(lrc);
    }

    /// 强行更新歌词区文字，用于显示歌词加载提示
    pub fn update_lyrics_text(&self, text: &str) {
        let imp = self.imp();
        let page = imp.playlist_lyrics_page.get().unwrap();
        page.update_lyrics_text(text);
    }

    // 更新歌词高亮位置
    pub fn update_lyrics_timestamp(&self, time: u64) {
        let imp = self.imp();
        let page = imp.playlist_lyrics_page.get().unwrap();
        if self.page_cur_playlist_lyrics_page() {
            page.update_lyrics_highlight(time);
        }
    }

    pub fn update_playlist_status(&self, index: usize) {
        let imp = self.imp();
        let page = imp.playlist_lyrics_page.get().unwrap();
        if self.page_cur_playlist_lyrics_page() {
            page.switch_row(index as i32);
        }
    }

    pub fn set_song_url(&self, si: SongInfo) {
        self.imp().player_controls.get().set_song_url(si);
    }
    pub fn gst_duration_changed(&self, sec: u64) {
        self.imp().player_controls.get().gst_duration_changed(sec);
    }
    pub fn gst_state_changed(&self, state: gstreamer_play::PlayState) {
        self.imp().player_controls.get().gst_state_changed(state);
    }
    pub fn gst_volume_changed(&self, volume: f64) {
        self.imp().player_controls.get().gst_volume_changed(volume);
    }
    pub fn gst_cache_download_complete(&self, loc: String) {
        self.imp()
            .player_controls
            .get()
            .gst_cache_download_complete(loc);
    }
    pub fn scale_seek_update(&self, sec: u64) {
        self.imp().player_controls.get().scale_seek_update(sec);
    }
    pub fn scale_value_update(&self) {
        self.imp().player_controls.get().scale_value_update();
    }
    pub fn init_mpris(&self, mpris: MprisController) {
        self.imp().player_controls.get().init_mpris(mpris);
    }

    pub async fn action_search(
        &self,
        ncmapi: NcmClient,
        text: String,
        search_type: SearchType,
        offset: u16,
        limit: u16,
    ) -> Option<SearchResult> {
        let imp = self.imp();
        let sender = imp.sender.get().unwrap().clone();
        let window = self;

        let res = match search_type {
            SearchType::Song => ncmapi
                .client
                .search_song(text, offset, limit)
                .await
                .map(|res| {
                    debug!("搜索歌曲：{:?}", res);
                    let likes = window.get_song_likes(&res);
                    SearchResult::Songs(res, likes)
                }),
            SearchType::Singer => {
                ncmapi
                    .client
                    .search_singer(text, offset, limit)
                    .await
                    .map(|res| {
                        debug!("搜索歌手：{:?}", res);
                        SearchResult::Singers(res)
                    })
            }
            SearchType::Album => ncmapi
                .client
                .search_album(text, offset, limit)
                .await
                .map(|res| {
                    debug!("搜索专辑：{:?}", res);
                    SearchResult::SongLists(res)
                }),
            SearchType::Lyrics => {
                ncmapi
                    .client
                    .search_lyrics(text, offset, limit)
                    .await
                    .map(|res| {
                        debug!("搜索歌词：{:?}", res);
                        let likes = window.get_song_likes(&res);
                        SearchResult::Songs(res, likes)
                    })
            }
            SearchType::SongList => ncmapi
                .client
                .search_songlist(text, offset, limit)
                .await
                .map(|res| {
                    debug!("搜索歌单：{:?}", res);
                    SearchResult::SongLists(res)
                }),
            SearchType::TopPicks => ncmapi
                .client
                .top_song_list("全部", "hot", offset, limit)
                .await
                .map(|res| {
                    debug!("获取歌单：{:?}", res);
                    SearchResult::SongLists(res)
                }),
            SearchType::AllAlbums => {
                ncmapi
                    .client
                    .new_albums("ALL", offset, limit)
                    .await
                    .map(|res| {
                        debug!("获取专辑：{:?}", res);
                        SearchResult::SongLists(res)
                    })
            }
            SearchType::Radio => ncmapi
                .client
                .user_radio_sublist(offset, limit)
                .await
                .map(|res| {
                    debug!("获取电台：{:?}", res);
                    SearchResult::SongLists(res)
                }),
            SearchType::LikeAlbums => ncmapi.client.album_sublist(offset, limit).await.map(|res| {
                debug!("获取收藏的专辑：{:?}", res);
                SearchResult::SongLists(res)
            }),
            SearchType::LikeSongList => {
                let uid = window.get_uid();
                ncmapi
                    .client
                    .user_song_list(uid, offset, limit)
                    .await
                    .map(|res| {
                        debug!("获取收藏的歌单：{:?}", res);
                        SearchResult::SongLists(res)
                    })
            }
            _ => Err(anyhow::anyhow!("")),
        };
        if let Err(err) = &res {
            error!("{:?}", err);
            sender
                .send_blocking(Action::AddToast(gettext(
                    "Request for interface failed, please try again!",
                )))
                .unwrap();
        }
        res.ok()
    }
}

#[gtk::template_callbacks]
impl NeteaseCloudMusicGtk4Window {
    #[template_callback]
    fn stack_visible_child_cb(&self) {
        let imp = self.imp();
        let stack = imp.stack.get();
        let label = imp.label_title.get();
        if let Some(visible_child_name) = stack.visible_child_name() {
            let mut stack_child = LinkedList::new();
            if let Ok(sc) = imp.stack_child.lock() {
                stack_child = (*sc).clone();
            }
            if let Some(child) = stack_child.back() {
                if visible_child_name == child.0 {
                    return;
                }
            }
            if stack_child.len() == 1 {
                if visible_child_name == "discover"
                    || visible_child_name == "toplist"
                    || visible_child_name == "my"
                {
                    if let Ok(mut sc) = imp.stack_child.lock() {
                        sc.pop_back();
                        sc.push_back((visible_child_name.to_string(), "".to_owned()));
                    }
                } else if let Ok(mut sc) = imp.stack_child.lock() {
                    sc.push_back((visible_child_name.to_string(), label.text().to_string()));
                }
            } else if visible_child_name == "discover"
                || visible_child_name == "toplist"
                || visible_child_name == "my"
            {
                if let Ok(mut sc) = imp.stack_child.lock() {
                    sc.clear();
                    sc.push_back((visible_child_name.to_string(), "".to_owned()));
                }
            } else if let Ok(mut sc) = imp.stack_child.lock() {
                sc.push_back((visible_child_name.to_string(), label.text().to_string()));
            }
        }
    }

    #[template_callback]
    fn search_song_cb(&self, check: CheckButton) {
        let menu = self.imp().search_menu.get();
        menu.set_label(&check.label().unwrap());
        self.set_property("search-type", SearchType::Song);
    }

    #[template_callback]
    fn search_singer_cb(&self, check: CheckButton) {
        let menu = self.imp().search_menu.get();
        menu.set_label(&check.label().unwrap());
        self.set_property("search-type", SearchType::Singer);
    }

    #[template_callback]
    fn search_album_cb(&self, check: CheckButton) {
        let menu = self.imp().search_menu.get();
        menu.set_label(&check.label().unwrap());
        self.set_property("search-type", SearchType::Album);
    }

    #[template_callback]
    fn search_lyrics_cb(&self, check: CheckButton) {
        let menu = self.imp().search_menu.get();
        menu.set_label(&check.label().unwrap());
        self.set_property("search-type", SearchType::Lyrics);
    }

    #[template_callback]
    fn search_songlist_cb(&self, check: CheckButton) {
        let menu = self.imp().search_menu.get();
        menu.set_label(&check.label().unwrap());
        self.set_property("search-type", SearchType::SongList);
    }

    #[template_callback]
    fn search_entry_cb(&self, entry: SearchEntry) {
        let imp = self.imp();
        let sender = imp.sender.get().unwrap();
        let text = entry.text().to_string();
        imp.label_title.set_label(&text);
        imp.switcher_title.set_visible(false);
        imp.label_title.set_visible(true);
        imp.back_button.set_visible(true);

        let search_type = self.property::<SearchType>("search-type");

        let page = match search_type {
            SearchType::Lyrics | SearchType::Song => {
                let page = self.init_search_song_page(&text, search_type);
                Some(page.upcast::<Widget>())
            }
            SearchType::Singer => {
                let page = self.init_search_singer_page(&text);
                Some(page.upcast::<Widget>())
            }
            SearchType::Album | SearchType::SongList => {
                let page = self.init_search_songlist_page(&text, search_type);
                Some(page.upcast::<Widget>())
            }
            _ => None,
        };
        if let Some(page) = page {
            self.page_new_with_name("search", &page, text.as_str());
            let page = glib::SendWeakRef::from(page.downgrade());
            sender
                .send_blocking(Action::Search(
                    text,
                    search_type,
                    0,
                    50,
                    Arc::new(move |res| {
                        if let Some(page) = page.upgrade() {
                            match res {
                                SearchResult::Songs(sis, likes) => {
                                    page.downcast::<SearchSongPage>()
                                        .unwrap()
                                        .update_songs(&sis, &likes);
                                }
                                SearchResult::Singers(sgs) => {
                                    page.downcast::<SearchSingerPage>()
                                        .unwrap()
                                        .update_singer(sgs);
                                }
                                SearchResult::SongLists(sls) => {
                                    page.downcast::<SearchSongListPage>()
                                        .unwrap()
                                        .update_songlist(&sls);
                                }
                            };
                        }
                    }),
                ))
                .unwrap();
        }
    }
}

impl Default for NeteaseCloudMusicGtk4Window {
    fn default() -> Self {
        NeteaseCloudMusicGtk4Application::default()
            .active_window()
            .unwrap()
            .downcast()
            .unwrap()
    }
}
