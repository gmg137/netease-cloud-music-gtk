use crate::{
    application::{Action, NeteaseCloudMusicGtk4Application},
    gui::*,
    model::*,
    ncmapi::COOKIE_JAR,
};
use adw::{ColorScheme, StyleManager, Toast};
use gettextrs::gettext;
use gio::{Settings, SimpleAction};
use glib::{clone, ParamFlags, ParamSpec, ParamSpecEnum, ParamSpecObject, Sender, Value};
use gtk::{
    gio::{self, SettingsBindFlags},
    glib, CompositeTemplate,
};
use ncm_api::{BannersInfo, LoginInfo, SingerInfo, SongInfo, SongList, TopList};
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
        pub switcher_title: TemplateChild<adw::ViewSwitcherTitle>,
        #[template_child]
        pub label_title: TemplateChild<Label>,
        #[template_child]
        pub user_button: TemplateChild<MenuButton>,
        #[template_child]
        pub player_revealer: TemplateChild<Revealer>,
        #[template_child]
        pub songlist_page: TemplateChild<SonglistPage>,
        #[template_child]
        pub player_controls: TemplateChild<PlayerControls>,
        #[template_child]
        pub toplist: TemplateChild<TopListView>,
        #[template_child]
        pub discover: TemplateChild<Discover>,
        #[template_child]
        pub search_song_page: TemplateChild<SearchSongPage>,
        #[template_child]
        pub search_songlist_page: TemplateChild<SearchSongListPage>,
        #[template_child]
        pub search_singer_page: TemplateChild<SearchSingerPage>,
        #[template_child]
        pub my_stack: TemplateChild<adw::ViewStack>,
        #[template_child]
        pub my_page: TemplateChild<MyPage>,
        #[template_child]
        pub playlist_lyrics_page: TemplateChild<PlayListLyricsPage>,

        pub user_menus: OnceCell<UserMenus>,
        pub popover_menu: OnceCell<PopoverMenu>,
        pub settings: OnceCell<Settings>,
        pub sender: OnceCell<Sender<Action>>,
        pub stack_child: Arc<Mutex<LinkedList<(String, String)>>>,
        search_type: Cell<SearchType>,
        toast: RefCell<Option<Toast>>,
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
        fn constructed(&self, obj: &Self::Type) {
            self.parent_constructed(obj);

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
                    ParamSpecEnum::new(
                        // Name
                        "search-type",
                        // Nickname
                        "search-type",
                        // Short description
                        "search type",
                        // Enum type
                        SearchType::static_type(),
                        // Default value
                        SearchType::default() as i32,
                        // The property can be read and written to
                        ParamFlags::READWRITE | glib::ParamFlags::EXPLICIT_NOTIFY,
                    ),
                    ParamSpecObject::new(
                        "toast",
                        "toast",
                        "toast",
                        Toast::static_type(),
                        glib::ParamFlags::READWRITE | glib::ParamFlags::CONSTRUCT_ONLY,
                    ),
                ]
            });
            PROPERTIES.as_ref()
        }

        fn set_property(&self, _obj: &Self::Type, _id: usize, value: &Value, pspec: &ParamSpec) {
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
                _ => unimplemented!(),
            }
        }

        fn property(&self, _obj: &Self::Type, _id: usize, pspec: &ParamSpec) -> Value {
            match pspec.name() {
                "toast" => self.toast.borrow().to_value(),
                "search-type" => self.search_type.get().to_value(),
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
    pub fn new<P: glib::IsA<gtk::Application>>(application: &P, sender: Sender<Action>) -> Self {
        let window: NeteaseCloudMusicGtk4Window =
            glib::Object::new(&[("application", application)])
                .expect("Failed to create NeteaseCloudMusicGtk4Window");
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
        // 监测用户菜单弹出
        let popover = imp.popover_menu.get().unwrap();
        let sender = imp.sender.get().unwrap().clone();
        popover.connect_show(move |_| {
            if COOKIE_JAR.get().is_none() {
                sender.send(Action::UpdateQrCode).unwrap();
            }
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
        action_search.connect_activate(
            clone!(@weak search_button, @weak search_entry => move |_,_|{
                search_button.emit_clicked();
            }),
        );
        self.add_action(&action_search);

        let search_bar = imp.search_bar.get();
        search_bar.connect_search_mode_enabled_notify(clone!(@weak search_entry =>move |bar| {
            if bar.is_search_mode() {
                // 清空搜索框
                search_entry.set_text("");
                // 使搜索框获取输入焦点
                search_entry.grab_focus();
            }
        }));

        // 设置返回键功能
        let switcher_title = imp.switcher_title.get();
        let label_title = imp.label_title.get();
        let stack = imp.stack.get();
        let back_button = imp.back_button.get();
        let action_back = SimpleAction::new("back-button", None);
        let stack_child = imp.stack_child.clone();
        self.add_action(&action_back);
        action_back.connect_activate(
            clone!(@weak switcher_title, @weak label_title, @weak stack, @weak back_button => move |_,_|{
                let mut child_name = String::new();
                if let Ok(mut sc) = stack_child.lock() {
                    if sc.len() == 2 {
                        switcher_title.set_visible(true);
                        label_title.set_visible(false);
                        back_button.set_visible(false);
                        if let Some(s) = sc.front() {
                            child_name = s.0.to_owned();
                        }
                    } else {
                        sc.pop_back();
                        if let Some(s) = sc.pop_back() {
                            child_name = s.0;
                            label_title.set_text(&s.1);
                        }
                    }
                }
                if !child_name.is_empty() {
                    stack.set_visible_child_name(&child_name);
                }
            }),
        );

        let toast = self.property::<Toast>("toast");
        toast.connect_dismissed(|toast| {
            toast.set_title("");
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

    pub fn set_user_qrimage(&self, path: PathBuf) {
        let user_menus = self.imp().user_menus.get().unwrap();
        user_menus.set_qrimage(path);
    }

    pub fn set_user_qrimage_timeout(&self) {
        let user_menus = self.imp().user_menus.get().unwrap();
        user_menus.set_qrimage_timeout();
    }

    pub fn switch_user_menu_to_phone(&self) {
        let popover = self.imp().popover_menu.get().unwrap();
        let user_menus = self.imp().user_menus.get().unwrap();
        popover.remove_child(&user_menus.qrbox);
        popover.add_child(&user_menus.phonebox, "user_popover");
    }

    pub fn switch_user_menu_to_qr(&self) {
        let popover = self.imp().popover_menu.get().unwrap();
        let user_menus = self.imp().user_menus.get().unwrap();
        popover.remove_child(&user_menus.phonebox);
        popover.add_child(&user_menus.qrbox, "user_popover");
    }

    pub fn switch_user_menu_to_user(&self, login_info: LoginInfo, menu: UserMenuChild) {
        let popover = self.imp().popover_menu.get().unwrap();
        let user_menus = self.imp().user_menus.get().unwrap();
        match menu {
            UserMenuChild::Qr => popover.remove_child(&user_menus.qrbox),
            UserMenuChild::Phone => popover.remove_child(&user_menus.phonebox),
            UserMenuChild::User => return,
        };
        popover.add_child(&user_menus.userbox, "user_popover");
        user_menus.set_user_name(login_info.nickname);
    }

    pub fn set_avatar(&self, url: PathBuf) {
        self.imp().user_menus.get().unwrap().set_user_avatar(url);
    }

    pub fn add_toast(&self, mes: String) {
        let toast = self.property::<Toast>("toast");
        if !toast.title().is_empty() {
            if toast.title() == mes {
                return;
            }
            toast.dismiss();
        }
        toast.set_title(&mes);
        self.imp().toast_overlay.add_toast(&toast);
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

    pub fn add_playlist(&self, sis: Vec<SongInfo>) {
        let player_controls = self.imp().player_controls.get();
        player_controls.add_list(sis);
        let sender = self.imp().sender.get().unwrap();
        sender.send(Action::PlayListStart).unwrap();
    }

    pub fn playlist_start(&self) {
        let sender = self.imp().sender.get().unwrap();
        let player_controls = self.imp().player_controls.get();
        if let Some(song_info) = player_controls.get_current_song() {
            sender.send(Action::Play(song_info)).unwrap();
            return;
        }
        sender
            .send(Action::AddToast(gettext("No playable songs found！")))
            .unwrap();
    }

    pub fn play_next(&self) {
        let player_controls = self.imp().player_controls.get();
        player_controls.next_song();
    }

    pub fn play(&self, song_info: SongInfo) {
        let player_controls = self.imp().player_controls.get();
        player_controls.play(song_info);
        let player_revealer = self.imp().player_revealer.get();
        if !player_revealer.reveals_child() {
            player_revealer.set_visible(true);
            player_revealer.set_reveal_child(true);
        }
    }

    pub fn switch_stack_to_songlist_page(&self, songlist: &SongList) {
        let imp = self.imp();
        let switcher_title = imp.switcher_title.get();
        switcher_title.set_visible(false);

        let label_title = imp.label_title.get();
        label_title.set_visible(true);
        label_title.set_label(&songlist.name);

        let stack = imp.stack.get();
        stack.set_visible_child_name("songlist");

        let back_button = imp.back_button.get();
        back_button.set_visible(true);

        let songlist_page = imp.songlist_page.get();
        songlist_page.init_songlist_info(songlist);
    }

    pub fn init_songlist_page(&self, sis: Vec<SongInfo>, page_type: DiscoverSubPage) {
        let songlist_page = self.imp().songlist_page.get();
        songlist_page.init_songlist(sis, page_type);
    }

    pub fn init_page_data(&self) {
        let imp = self.imp();
        let sender = imp.sender.get().unwrap();

        // 初始化我的页面
        let my_page = imp.my_page.get();
        my_page.set_sender(sender.clone());

        // 初始化搜索单曲页
        let search_song_page = imp.search_song_page.get();
        search_song_page.set_sender(sender.clone());

        // 初始化搜索歌手页
        let search_singer_page = imp.search_singer_page.get();
        search_singer_page.set_sender(sender.clone());

        // 初始化搜索歌单页
        let search_songlist_page = imp.search_songlist_page.get();
        search_songlist_page.set_sender(sender.clone());

        // 初始化播放栏
        let player_controls = imp.player_controls.get();
        player_controls.set_sender(sender.clone());

        // 初始化发现页
        let discover = imp.discover.get();
        discover.set_sender(sender.clone());
        discover.init_page();

        // 初始化歌单页
        let songlist_page = imp.songlist_page.get();
        songlist_page.set_sender(sender.clone());

        // 初始化榜单
        sender.send(Action::GetToplist).unwrap();
        let toplist = imp.toplist.get();
        toplist.set_sender(sender.clone());

        // 初始化播放列表页
        let playlist_lyrics_page = imp.playlist_lyrics_page.get();
        playlist_lyrics_page.set_sender(sender.clone());
    }

    pub fn init_toplist(&self, list: Vec<TopList>) {
        let toplist = self.imp().toplist.get();
        toplist.init_sidebar(list);
    }

    pub fn update_toplist(&self, list: Vec<SongInfo>) {
        let toplist = self.imp().toplist.get();
        toplist.update_songs_list(list);
    }

    pub fn update_search_song_page(&self, list: Vec<SongInfo>) {
        let search_song_page = self.imp().search_song_page.get();
        search_song_page.update_songs(list);
    }

    pub fn update_search_songlist_page(&self, list: Vec<SongList>) {
        let search_songlist_page = self.imp().search_songlist_page.get();
        search_songlist_page.update_songlist(list);
    }

    pub fn update_search_singer_page(&self, list: Vec<SingerInfo>) {
        let search_singer_page = self.imp().search_singer_page.get();
        search_singer_page.update_singer(list);
    }

    pub fn init_picks_songlist(&self) {
        let imp = self.imp();
        imp.label_title.set_label("全部歌单");
        imp.switcher_title.set_visible(false);
        imp.label_title.set_visible(true);
        imp.back_button.set_visible(true);
        imp.search_songlist_page
            .init_page("全部歌单".to_owned(), SearchType::TopPicks);
        imp.stack.set_visible_child_name("search_songlist_page");
    }

    pub fn init_all_albums(&self) {
        let imp = self.imp();
        imp.label_title.set_label("全部新碟");
        imp.switcher_title.set_visible(false);
        imp.label_title.set_visible(true);
        imp.back_button.set_visible(true);
        imp.search_songlist_page
            .init_page("全部新碟".to_owned(), SearchType::AllAlbums);
        imp.stack.set_visible_child_name("search_songlist_page");
    }

    pub fn init_search_song_page(&self, text: &str, search_type: SearchType) {
        let imp = self.imp();
        imp.search_song_page.init_page(text.to_owned(), search_type);
        imp.label_title.set_label(text);
        imp.switcher_title.set_visible(false);
        imp.label_title.set_visible(true);
        imp.back_button.set_visible(true);
        imp.stack.set_visible_child_name("search_song_page");
    }

    pub fn init_search_songlist_page(&self, text: &str, search_type: SearchType) {
        let imp = self.imp();
        imp.search_songlist_page
            .init_page(text.to_owned(), search_type);
        imp.label_title.set_label(text);
        imp.switcher_title.set_visible(false);
        imp.label_title.set_visible(true);
        imp.back_button.set_visible(true);
        imp.stack.set_visible_child_name("search_songlist_page");
    }

    pub fn switch_my_page_to_login(&self) {
        let imp = self.imp();
        imp.my_stack.set_visible_child_name("my_login");
    }

    pub fn init_my_page(&self, sls: Vec<SongList>) {
        self.imp().my_page.init_page(sls);
    }

    pub fn init_playlist_lyrics_page(&self, sis: Vec<SongInfo>, si: SongInfo) {
        let imp = self.imp();
        imp.playlist_lyrics_page.init_page(sis, si);
        imp.label_title.set_label(&gettext("Play List&Lyrics"));
        imp.switcher_title.set_visible(false);
        imp.label_title.set_visible(true);
        imp.back_button.set_visible(true);
        imp.stack.set_visible_child_name("playlist_lyrics_page");
    }

    pub fn update_lyrics(&self, lrc: String) {
        let imp = self.imp();
        if let Some(name) = imp.stack.visible_child_name() {
            if name == "playlist_lyrics_page" {
                imp.playlist_lyrics_page.update_lyrics(lrc);
            }
        }
    }

    pub fn updat_playlist_status(&self, index: usize) {
        let imp = self.imp();
        if let Some(name) = imp.stack.visible_child_name() {
            if name == "playlist_lyrics_page" {
                imp.playlist_lyrics_page.switch_row(index as i32);
            }
        }
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
        match search_type {
            SearchType::Song => {
                imp.search_song_page.init_page(text.to_owned(), search_type);
                imp.stack.set_visible_child_name("search_song_page");
            }
            SearchType::Singer => {
                imp.search_singer_page.init_page(text.to_owned());
                imp.stack.set_visible_child_name("search_singer_page");
            }
            SearchType::Lyrics => {
                imp.search_song_page.init_page(text.to_owned(), search_type);
                imp.stack.set_visible_child_name("search_song_page");
            }
            SearchType::Album => {
                imp.search_songlist_page
                    .init_page(text.to_owned(), search_type);
                imp.stack.set_visible_child_name("search_songlist_page");
            }
            SearchType::SongList => {
                imp.search_songlist_page
                    .init_page(text.to_owned(), search_type);
                imp.stack.set_visible_child_name("search_songlist_page");
            }
            _ => (),
        }
        sender
            .send(Action::Search(
                text,
                self.property::<SearchType>("search-type"),
                0,
                50,
            ))
            .unwrap();
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
