//
// songlist_view.rs
// Copyright (C) 2022 gmg137 <gmg137 AT live.com>
// Distributed under terms of the GPL-3.0-or-later license.
//
// 歌曲列表：GtkListView + ListStore 虚拟化渲染。
// 早先用 GtkListBox 一次性插入所有行，几千首时是 5 万多个 widget（RSS ~780MB、
// 滚动明显卡顿）；现在只渲染可视区域内的行，数据放 ListStore。
use gio::Settings;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::{glib, CompositeTemplate, *};

use crate::{application::Action, gui::songlist_row::SonglistRow};
use async_channel::Sender;
use glib::{
    clone, subclass::Signal, ParamSpec, ParamSpecBoolean, ParamSpecInt, RustClosure,
    SignalHandlerId, Value,
};
use ncm_api::SongInfo;
use once_cell::sync::{Lazy, OnceCell};
use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
};

/// ListStore 里每一项：歌曲 + 是否已收藏
pub struct RowEntry {
    pub si: SongInfo,
    pub like: bool,
}

glib::wrapper! {
    pub struct SongListView(ObjectSubclass<imp::SongListView>)
        @extends Widget, Box,
        @implements Accessible, Actionable, Buildable, ConstraintTarget;
}

impl Default for SongListView {
    fn default() -> Self {
        Self::new()
    }
}

impl SongListView {
    pub fn new() -> Self {
        glib::Object::builder().build()
    }

    pub fn set_sender(&self, _sender: Sender<Action>) {
        let sender = &self.imp().sender;
        if sender.get().is_none() {
            sender.set(_sender).unwrap();
        }
    }

    fn setup_settings(&self) {
        let settings = Settings::new(crate::APP_ID);

        self.imp()
            .settings
            .set(settings)
            .expect("Could not set `Settings`.");
    }

    /// 保留接口：虚拟化之后没有分片任务，不需要取消
    pub fn cancel_render(&self) {}

    /// 重建整个列表（数据进 ListStore，行由 ListView 按需创建）
    pub fn init_new_list(&self, sis: &[SongInfo], likes: &[bool]) {
        let imp = self.imp();
        let store = imp.store.get().unwrap();
        store.remove_all();
        imp.live_rows.borrow_mut().clear();
        let items: Vec<glib::BoxedAnyObject> = sis
            .iter()
            .zip(likes.iter())
            .map(|(si, like)| {
                glib::BoxedAnyObject::new(RowEntry {
                    si: si.clone(),
                    like: *like,
                })
            })
            .collect();
        store.extend_from_slice(&items);
        imp.list_view.scroll_to(0, ListScrollFlags::NONE, None);
    }

    /// 追加一批（搜索/翻页增量用）
    pub fn append_rows(&self, sis: &[SongInfo], likes: &[bool]) {
        let imp = self.imp();
        let store = imp.store.get().unwrap();
        let items: Vec<glib::BoxedAnyObject> = sis
            .iter()
            .zip(likes.iter())
            .map(|(si, like)| {
                glib::BoxedAnyObject::new(RowEntry {
                    si: si.clone(),
                    like: *like,
                })
            })
            .collect();
        store.extend_from_slice(&items);
    }

    /// 列表里的全部歌曲（按当前顺序）
    pub fn get_songinfo_list(&self) -> Vec<SongInfo> {
        let store = self.imp().store.get().unwrap();
        (0..store.n_items())
            .filter_map(|index| store.item(index))
            .filter_map(|item| item.downcast::<glib::BoxedAnyObject>().ok())
            .map(|item| item.borrow::<RowEntry>().si.clone())
            .collect()
    }

    pub fn clear_list(&self) {
        let imp = self.imp();
        if let Some(store) = imp.store.get() {
            store.remove_all();
        }
        imp.live_rows.borrow_mut().clear();
    }

    /// 滚动到正在播放的那首；列表里没有它（比如搜索过滤掉了）就返回 false
    pub fn scroll_to_playing(&self) -> bool {
        let imp = self.imp();
        let playing_id = imp.playing_id.get();
        if playing_id == 0 {
            return false;
        }
        let Some(store) = imp.store.get() else {
            return false;
        };
        for index in 0..store.n_items() {
            let hit = store
                .item(index)
                .and_then(|item| item.downcast::<glib::BoxedAnyObject>().ok())
                .map(|item| item.borrow::<RowEntry>().si.id == playing_id)
                .unwrap_or(false);
            if hit {
                imp.list_view.scroll_to(index, ListScrollFlags::NONE, None);
                return true;
            }
        }
        false
    }

    /// 根据当前播放歌曲 id 更新指示符（只改已经渲染出来的行）
    pub fn update_playing_song(&self, song_id: u64) {
        self.imp().playing_id.set(song_id);
        for row in self.imp().live_rows.borrow().values() {
            let visible = row
                .get_song_info()
                .map(|si| si.id == song_id)
                .unwrap_or(false);
            row.switch_image(visible);
        }
    }

    /// 以当前列表顺序构造播放队列，从被点击的行开始（连续播放用）
    fn play_list_from(&self, clicked: &SonglistRow) -> Vec<SongInfo> {
        let mut songs = self.get_songinfo_list();
        if let Some(si) = clicked.get_song_info() {
            if let Some(pos) = songs.iter().position(|song| song.id == si.id) {
                songs.rotate_left(pos);
            }
        }
        songs
    }

    // ListView 的 bind：为这一项创建一个行（滚出视野就被回收，控件数始终只跟可视区域有关）
    fn bind_row(&self, list_item: &ListItem) {
        let imp = self.imp();
        let Some(item) = list_item.item() else {
            return;
        };
        let Ok(boxed) = item.downcast::<glib::BoxedAnyObject>() else {
            return;
        };
        let (si, like) = {
            let entry = boxed.borrow::<RowEntry>();
            (entry.si.clone(), entry.like)
        };
        let Some(sender) = imp.sender.get().cloned() else {
            return;
        };

        let row = SonglistRow::new(sender.clone(), &si);
        // ListView 的每一项默认按自然宽度摆放，行要自己撑满整行
        row.set_halign(Align::Fill);
        row.set_hexpand(true);
        row.set_property("like", like);
        row.set_like_button_visible(!imp.no_act_like.get());
        row.set_album_button_visible(!imp.no_act_album.get());
        row.set_remove_button_visible(!imp.no_act_remove.get());
        row.set_block_remove(imp.block_remove.get());
        row.switch_image(si.id == imp.playing_id.get());

        let replace_on_activate = imp.replace_on_activate.get();
        let si_click = si.clone();
        row.connect_activate(clone!(
            #[weak(rename_to = s)]
            self,
            move |row| {
                if row.is_activatable() || row.not_ignore_grey() {
                    row.switch_image(true);
                    if replace_on_activate {
                        // 连续播放：用当前列表（含搜索过滤）替换队列，从点击的这首开始
                        let list = s.play_list_from(row);
                        if list.is_empty() {
                            sender.send_blocking(Action::AddPlay(si_click.clone())).unwrap();
                        } else {
                            sender.send_blocking(Action::AddPlayList(list, true)).unwrap();
                        }
                    } else {
                        sender.send_blocking(Action::AddPlay(si_click.clone())).unwrap();
                    }
                    s.emit_row_activated(row);
                }
            }
        ));

        if let Some(settings) = imp.settings.get() {
            settings
                .bind("not-ignore-grey", &row, "not-ignore-grey")
                .get_only()
                .build();
        }

        imp.live_rows.borrow_mut().insert(si.id, row.clone());
        list_item.set_child(Some(&row));
    }

    pub fn emit_row_activated(&self, row: &SonglistRow) {
        self.emit_by_name::<()>("row-activated", &[&row]);
    }

    pub fn connect_row_activated(&self, f: RustClosure) -> SignalHandlerId {
        self.connect_closure("row-activated", false, f)
    }
}

#[gtk::template_callbacks]
impl SongListView {}

mod imp {

    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/gitee/gmg137/NeteaseCloudMusicGtk4/gtk/songlist-view.ui")]
    pub struct SongListView {
        #[template_child]
        pub scroll_win: TemplateChild<ScrolledWindow>,
        #[template_child]
        pub list_view: TemplateChild<ListView>,

        pub store: OnceCell<gio::ListStore>,
        // 当前已渲染出来的行（播放在播指示更新用）
        pub live_rows: RefCell<HashMap<u64, SonglistRow>>,

        pub sender: OnceCell<Sender<Action>>,
        pub settings: OnceCell<Settings>,
        // 保留字段：老代码在重建列表前会调 cancel_render()
        pub render_gen: Cell<u64>,

        pub no_act_like: Cell<bool>,
        pub no_act_album: Cell<bool>,
        pub no_act_remove: Cell<bool>,
        // 垃圾桶按钮语义改为「不再推荐」
        pub block_remove: Cell<bool>,
        // 当前播放歌曲 id
        pub playing_id: Cell<u64>,
        // 点击行时用「整个当前列表」替换播放队列并从该行开始（连续播放）
        pub replace_on_activate: Cell<bool>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for SongListView {
        const NAME: &'static str = "SongListView";
        type Type = super::SongListView;
        type ParentType = Box;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
            klass.bind_template_callbacks();
            klass.bind_template_instance_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[gtk::template_callbacks]
    impl SongListView {}

    impl ObjectImpl for SongListView {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();
            obj.setup_settings();

            let store = gio::ListStore::new::<glib::BoxedAnyObject>();
            self.store.set(store.clone()).unwrap();

            let factory = SignalListItemFactory::new();
            factory.connect_bind(clone!(
                #[weak]
                obj,
                move |_, list_item| {
                    if let Some(list_item) = list_item.downcast_ref::<ListItem>() {
                        obj.bind_row(list_item);
                    }
                }
            ));
            // 行滚出视野就解绑：从 live_rows 里摘掉，避免持有已回收的行
            factory.connect_unbind(clone!(
                #[weak]
                obj,
                move |_, list_item| {
                    let Some(list_item) = list_item.downcast_ref::<ListItem>() else {
                        return;
                    };
                    if let Some(row) = list_item
                        .child()
                        .and_then(|child| child.downcast::<SonglistRow>().ok())
                    {
                        if let Some(si) = row.get_song_info() {
                            obj.imp().live_rows.borrow_mut().remove(&si.id);
                        }
                    }
                    list_item.set_child(None::<&Widget>);
                }
            ));

            // 不用选中态：行内自己处理点击
            let selection = NoSelection::new(Some(store.clone()));
            self.list_view.set_factory(Some(&factory));
            self.list_view.set_model(Some(&selection));
        }

        fn signals() -> &'static [Signal] {
            static SIGNALS: Lazy<Vec<Signal>> = Lazy::new(|| {
                vec![Signal::builder("row-activated")
                    .param_types([SonglistRow::static_type()])
                    .build()]
            });
            SIGNALS.as_ref()
        }

        fn properties() -> &'static [ParamSpec] {
            static PROPERTIES: Lazy<Vec<ParamSpec>> = Lazy::new(|| {
                vec![
                    ParamSpecBoolean::builder("no-act-like").build(),
                    ParamSpecBoolean::builder("no-act-album").build(),
                    ParamSpecBoolean::builder("no-act-remove").build(),
                    ParamSpecBoolean::builder("block-remove").build(),
                    ParamSpecBoolean::builder("replace-on-activate").build(),
                    ParamSpecInt::builder("clamp-margin-top").build(),
                    ParamSpecInt::builder("clamp-margin-bottom").build(),
                    ParamSpecInt::builder("clamp-maximum-size").build(),
                    ParamSpecInt::builder("clamp-tightening-threshold").build(),
                ]
            });
            PROPERTIES.as_ref()
        }

        fn set_property(&self, _id: usize, value: &Value, pspec: &ParamSpec) {
            match pspec.name() {
                "no-act-like" => {
                    let val = value.get().unwrap();
                    self.no_act_like.replace(val);
                }
                "no-act-album" => {
                    let val = value.get().unwrap();
                    self.no_act_album.replace(val);
                }
                "no-act-remove" => {
                    let val = value.get().unwrap();
                    self.no_act_remove.replace(val);
                }
                "block-remove" => {
                    let val = value.get().unwrap();
                    self.block_remove.replace(val);
                }
                "replace-on-activate" => {
                    let val = value.get().unwrap();
                    self.replace_on_activate.replace(val);
                }
                "clamp-margin-top" | "clamp-margin-bottom" | "clamp-maximum-size"
                | "clamp-tightening-threshold" => {}
                n => unimplemented!("{}", n),
            }
        }

        fn property(&self, _id: usize, pspec: &ParamSpec) -> Value {
            match pspec.name() {
                "no-act-like" => self.no_act_like.get().to_value(),
                "no-act-album" => self.no_act_album.get().to_value(),
                "no-act-remove" => self.no_act_remove.get().to_value(),
                "block-remove" => self.block_remove.get().to_value(),
                "replace-on-activate" => self.replace_on_activate.get().to_value(),
                "clamp-margin-top" | "clamp-margin-bottom" | "clamp-maximum-size"
                | "clamp-tightening-threshold" => 0i32.to_value(),
                n => unimplemented!("{}", n),
            }
        }
    }
    impl WidgetImpl for SongListView {}
    impl BoxImpl for SongListView {}
}
