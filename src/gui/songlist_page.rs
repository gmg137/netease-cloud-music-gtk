//
// songlist_page.rs
// Copyright (C) 2022 gmg137 <gmg137 AT live.com>
// Distributed under terms of the GPL-3.0-or-later license.
//
use gettextrs::gettext;
use glib::Sender;
pub(crate) use gtk::{glib, prelude::*, subclass::prelude::*, CompositeTemplate, *};
use ncm_api::{SongInfo, SongList};
use once_cell::sync::OnceCell;

use crate::{application::Action, model::DiscoverSubPage, ncmapi::COOKIE_JAR, path::CACHE};
use std::{cell::RefCell, rc::Rc};

use super::SonglistRow;

glib::wrapper! {
    pub struct SonglistPage(ObjectSubclass<imp::SonglistPage>)
        @extends gtk::Widget, gtk::Box,
        @implements gtk::Accessible, gtk::Buildable,gtk::ConstraintTarget, gtk::Orientable;
}

impl SonglistPage {
    pub fn new() -> Self {
        let songlist_page: SonglistPage =
            glib::Object::new(&[]).expect("Failed to create SonglistPage");
        songlist_page
    }

    pub fn set_sender(&self, sender: Sender<Action>) {
        self.imp().sender.set(sender).unwrap();
    }

    pub fn init_songlist_info(&self, songlist: &SongList) {
        let imp = self.imp();
        imp.songlist.replace(Some(songlist.to_owned()));

        // 判断是否显示收藏按钮
        let like_button = imp.like_button.get();
        if COOKIE_JAR.get().is_some() {
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

        // 删除旧内容
        let listbox = self.imp().listbox.get();
        while let Some(child) = listbox.last_child() {
            listbox.remove(&child);
        }
    }

    pub fn init_songlist(&self, sis: Vec<SongInfo>, page_type: DiscoverSubPage) {
        let imp = self.imp();
        imp.page_type.replace(Some(page_type));
        imp.playlist.replace(sis.clone());
        imp.num_label
            .get()
            .set_label(&gettext!("{} songs", sis.len()));
        let sender = imp.sender.get().unwrap();
        let listbox = imp.listbox.get();
        sis.into_iter().for_each(|si| {
            let row = SonglistRow::new();
            row.set_tooltip_text(Some(&si.name));

            row.set_name(&si.name);
            row.set_singer(&si.singer);
            row.set_album(&si.album);
            row.set_duration(&si.duration);

            let sender = sender.clone();
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
                        DiscoverSubPage::SongList => {
                            sender.send(Action::LikeSongList(songlist.id)).unwrap()
                        }
                        DiscoverSubPage::Album => {
                            sender.send(Action::LikeAlbum(songlist.id)).unwrap()
                        }
                    }
                }
            }
        }
    }

    impl ObjectImpl for SonglistPage {
        fn constructed(&self, obj: &Self::Type) {
            self.parent_constructed(obj);

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
        }
    }
    impl WidgetImpl for SonglistPage {}
    impl BoxImpl for SonglistPage {}
}
