//
// songlist_page.rs
// Copyright (C) 2022 gmg137 <gmg137 AT live.com>
// Distributed under terms of the GPL-3.0-or-later license.
//
use gettextrs::gettext;
use glib::{ParamFlags, ParamSpec, ParamSpecBoolean, Sender, Value};
pub(crate) use gtk::{glib, prelude::*, subclass::prelude::*, CompositeTemplate, *};
use ncm_api::{DetailDynamic, SongInfo, SongList};
use once_cell::sync::{Lazy, OnceCell};

use crate::{application::Action, model::DiscoverSubPage, path::CACHE};
use std::{
    cell::{Cell, RefCell},
    collections::HashSet,
    rc::Rc,
};

use super::SonglistRow;

glib::wrapper! {
    pub struct SonglistPage(ObjectSubclass<imp::SonglistPage>)
        @extends gtk::Widget, gtk::Box,
        @implements gtk::Accessible, gtk::Buildable,gtk::ConstraintTarget, gtk::Orientable;
}

impl SonglistPage {
    pub fn new() -> Self {
        let songlist_page: SonglistPage = glib::Object::new(&[]);
        songlist_page
    }

    pub fn set_sender(&self, sender: Sender<Action>) {
        self.imp().sender.set(sender).unwrap();
    }

    pub fn init_songlist_info(&self, songlist: &SongList, is_logined: bool) {
        let imp = self.imp();
        imp.songlist.replace(Some(songlist.to_owned()));

        // 判断是否显示收藏按钮
        let like_button = imp.like_button.get();
        if is_logined {
            like_button.set_visible(true);
        } else {
            like_button.set_visible(false);
        }

        // 设置专辑图
        let cover_image = imp.cover_image.get();
        let mut path = CACHE.clone();
        path.push(format!("{}-songlist.jpg", songlist.id));
        cover_image.set_from_file(Some(&path));

        // 设置标题
        let title = imp.title_label.get();
        title.set_label(&songlist.name);

        imp.num_label.get().set_label(&gettext!("{} songs", 0));
        self.set_property("like", false);

        // 删除旧内容
        let listbox = self.imp().listbox.get();
        while let Some(child) = listbox.last_child() {
            listbox.remove(&child);
        }
    }

    pub fn init_songlist(&self, sis: Vec<SongInfo>, dy: DetailDynamic, is_like_fn: impl Fn(&u64) -> bool) {
        let imp = self.imp();
        match dy {
            DetailDynamic::Album(dy) => {
                self.set_property("like", dy.is_sub);
                imp.page_type.replace(Some(DiscoverSubPage::Album));
                imp.num_label.get().set_label(&gettext!(
                    "{} songs, {} booked",
                    sis.len(),
                    dy.sub_count
                ));
            }
            DetailDynamic::SongList(dy) => {
                self.set_property("like", dy.subscribed);
                imp.page_type.replace(Some(DiscoverSubPage::SongList));
                imp.num_label.get().set_label(&gettext!(
                    "{} songs, {} booked",
                    sis.len(),
                    dy.booked_count
                ));
            }
        }

        imp.playlist.replace(sis.clone());
        let sender = imp.sender.get().unwrap();
        let listbox = imp.listbox.get();
        sis.into_iter().for_each(|si| {
            let row = SonglistRow::new();
            let sender = sender.clone();

            row.set_sender(sender.clone());
            row.set_from_song_info(&si);
            row.set_property("like", is_like_fn(&si.id));

            row.connect_activate(move |row| {
                row.switch_image(true);
                sender.send(Action::AddPlay(si.clone())).unwrap();
            });
            listbox.append(&row);
        });
    }
}

impl Default for SonglistPage {
    fn default() -> Self {
        Self::new()
    }
}

mod imp {

    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/gitee/gmg137/NeteaseCloudMusicGtk4/gtk/songlist-page.ui")]
    pub struct SonglistPage {
        #[template_child(id = "cover_image")]
        pub cover_image: TemplateChild<Image>,
        #[template_child(id = "title_label")]
        pub title_label: TemplateChild<Label>,
        #[template_child(id = "num_label")]
        pub num_label: TemplateChild<Label>,
        #[template_child(id = "play_button")]
        pub play_button: TemplateChild<Button>,
        #[template_child(id = "like_button")]
        pub like_button: TemplateChild<Button>,

        #[template_child(id = "listbox")]
        pub listbox: TemplateChild<ListBox>,

        pub playlist: Rc<RefCell<Vec<SongInfo>>>,
        pub songlist: Rc<RefCell<Option<SongList>>>,
        pub page_type: Rc<RefCell<Option<DiscoverSubPage>>>,

        pub sender: OnceCell<Sender<Action>>,

        like: Cell<bool>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for SonglistPage {
        const NAME: &'static str = "SonglistPage";
        type Type = super::SonglistPage;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[gtk::template_callbacks]
    impl SonglistPage {
        #[template_callback]
        fn play_button_clicked_cb(&self) {
            let sender = self.sender.get().unwrap();
            if !self.playlist.borrow().is_empty() {
                let playlist = &*self.playlist.borrow();
                sender.send(Action::AddPlayList(playlist.clone())).unwrap();
            } else {
                sender
                    .send(Action::AddToast(gettext("This is an empty song list！")))
                    .unwrap();
            }
        }

        #[template_callback]
        fn like_button_clicked_cb(&self) {
            let page_type = &*self.page_type.borrow();
            if let Some(pt) = page_type {
                let sender = self.sender.get().unwrap();
                if let Some(songlist) = &*self.songlist.borrow() {
                    match pt {
                        DiscoverSubPage::SongList => sender
                            .send(Action::LikeSongList(songlist.id, !self.like.get()))
                            .unwrap(),
                        DiscoverSubPage::Album => sender
                            .send(Action::LikeAlbum(songlist.id, !self.like.get()))
                            .unwrap(),
                    }
                }
            }
        }
    }

    impl ObjectImpl for SonglistPage {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();

            let select_row = Rc::new(RefCell::new(-1));
            self.listbox.connect_row_activated(move |list, row| {
                let index;
                {
                    index = *select_row.borrow();
                }
                if index != -1 && index != row.index() {
                    *select_row.borrow_mut() = row.index();
                    if let Some(row) = list.row_at_index(index) {
                        let row = row.downcast::<SonglistRow>().unwrap();
                        row.switch_image(false);
                    }
                } else {
                    *select_row.borrow_mut() = row.index();
                }
            });

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
            static PROPERTIES: Lazy<Vec<ParamSpec>> =
                Lazy::new(|| vec![ParamSpecBoolean::builder("like").readwrite().build()]);
            PROPERTIES.as_ref()
        }

        fn set_property(&self, _id: usize, value: &Value, pspec: &ParamSpec) {
            match pspec.name() {
                "like" => {
                    let like = value.get().expect("The value needs to be of type `bool`.");
                    self.like.replace(like);
                }
                _ => unimplemented!(),
            }
        }

        fn property(&self, _id: usize, pspec: &ParamSpec) -> Value {
            match pspec.name() {
                "like" => self.like.get().to_value(),
                _ => unimplemented!(),
            }
        }
    }
    impl WidgetImpl for SonglistPage {}
    impl BoxImpl for SonglistPage {}
}
