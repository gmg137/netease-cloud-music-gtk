//
// songlist_page.rs
// Copyright (C) 2022 gmg137 <gmg137 AT live.com>
// Distributed under terms of the GPL-3.0-or-later license.
//
use chrono::{TimeZone, Utc};
use gettextrs::gettext;
use glib::{ParamSpec, ParamSpecBoolean, Sender, Value};
pub(crate) use gtk::{glib, prelude::*, subclass::prelude::*, CompositeTemplate, *};
use ncm_api::SongList;
use once_cell::sync::{Lazy, OnceCell};

use crate::{
    application::Action,
    gui::songlist_view::SongListView,
    model::{DiscoverSubPage, ImageDownloadImpl, SongListDetail},
    path::CACHE,
};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    sync::Arc,
};

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

    pub fn init_songlist_info(&self, songlist: &SongList, is_album: bool, is_logined: bool) {
        let imp = self.imp();
        let sender = imp.sender.get().unwrap();
        imp.songlist.replace(Some(songlist.to_owned()));

        if is_album {
            imp.time_label.set_visible(true);
        }

        // 判断是否显示收藏按钮
        let like_button = imp.like_button.get();
        if is_logined {
            like_button.set_visible(true);
            imp.songs_list.set_property("no-act-like", false);
        } else {
            like_button.set_visible(false);
            imp.songs_list.set_property("no-act-like", true);
        }

        // 设置专辑图
        let cover_image = imp.cover_image.get();
        let mut path = CACHE.clone();
        path.push(format!("{}-songlist.jpg", songlist.id));
        if !path.exists() {
            cover_image.set_from_icon_name(Some("image-missing-symbolic"));
            cover_image.set_from_net(songlist.cover_img_url.to_owned(), path, (140, 140), sender);
        } else {
            cover_image.set_from_file(Some(&path));
        }

        // 设置标题
        let title = imp.title_label.get();
        title.set_label(&songlist.name);

        imp.num_label.get().set_label(&gettext!("{} songs", 0));
        self.set_property("like", false);

        imp.songs_list.clear_list();
    }

    pub fn init_songlist(&self, detail: &SongListDetail, likes: &[bool]) {
        let imp = self.imp();
        let songs_list = imp.songs_list.get();

        let sis = detail.sis();

        match detail {
            SongListDetail::Album(detail, dy) => {
                self.set_property("like", dy.is_sub);
                imp.songs_list.set_property("no-act-album", true);
                imp.page_type.replace(Some(DiscoverSubPage::Album));
                let dt = Utc
                    .timestamp_millis_opt(detail.publish_time as i64)
                    .unwrap();
                let dt = dt.format("%Y-%m-%d");
                imp.time_label.set_label(&format!("{}", dt,));

                imp.num_label.set_label(&format!(
                    "{}, {}",
                    gettext!("{} songs", sis.len()),
                    gettext!("{} favs", dy.sub_count)
                ));
            }
            SongListDetail::PlayList(_detail, dy) => {
                self.set_property("like", dy.subscribed);
                imp.songs_list.set_property("no-act-album", false);
                imp.page_type.replace(Some(DiscoverSubPage::SongList));
                imp.num_label.set_label(&format!(
                    "{}, {}",
                    gettext!("{} songs", sis.len()),
                    gettext!("{} favs", dy.booked_count)
                ));
            }
        }

        let sender = imp.sender.get().unwrap();
        songs_list.set_sender(sender.clone());
        songs_list.init_new_list(sis, likes);
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
        #[template_child(id = "time_label")]
        pub time_label: TemplateChild<Label>,
        #[template_child(id = "num_label")]
        pub num_label: TemplateChild<Label>,
        #[template_child(id = "play_button")]
        pub play_button: TemplateChild<Button>,
        #[template_child(id = "like_button")]
        pub like_button: TemplateChild<Button>,

        #[template_child(id = "songs_list")]
        pub songs_list: TemplateChild<SongListView>,

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
            let playlist = self.songs_list.get_songinfo_list();
            if !playlist.is_empty() {
                sender.send(Action::AddPlayList(playlist)).unwrap();
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
                    let s = glib::SendWeakRef::from(self.obj().downgrade());
                    let like = self.like.get();
                    let cb = Arc::new(move |_| {
                        if let Some(s) = s.upgrade() {
                            s.set_property("like", !like);
                        }
                    });
                    match pt {
                        DiscoverSubPage::SongList => sender
                            .send(Action::LikeSongList(songlist.id, !like, Some(cb)))
                            .unwrap(),
                        DiscoverSubPage::Album => sender
                            .send(Action::LikeAlbum(songlist.id, !like, Some(cb)))
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
