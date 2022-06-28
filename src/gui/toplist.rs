//
// toplist.rs
// Copyright (C) 2022 gmg137 <gmg137 AT live.com>
// Distributed under terms of the GPL-3.0-or-later license.
//
use adw::subclass::prelude::BinImpl;
use adw::traits::ActionRowExt;
use adw::ActionRow;
use gettextrs::gettext;
use glib::Sender;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::{glib, CompositeTemplate, *};
use ncm_api::{SongInfo, TopList};
use once_cell::sync::OnceCell;
use std::cell::RefCell;
use std::rc::Rc;

use crate::{application::Action, gui::songlist_row::SonglistRow, path::CACHE};

glib::wrapper! {
    pub struct TopListView(ObjectSubclass<imp::TopListView>)
        @extends Widget, Paned,
        @implements Accessible, Orientable, ConstraintTarget,Buildable;
}

impl Default for TopListView {
    fn default() -> Self {
        Self::new()
    }
}

impl TopListView {
    pub fn new() -> Self {
        glib::Object::new(&[]).expect("Failed to create TopListView")
    }

    pub fn set_sender(&self, sender: Sender<Action>) {
        self.imp().sender.set(sender).unwrap();
    }

    pub fn init_sidebar(&self, list: Vec<TopList>) {
        let sidebar = self.imp().sidebar.get();
        let mut select = false;
        for t in &list {
            let action = ActionRow::builder()
                .activatable(true)
                .title(&t.name)
                .subtitle(&t.update)
                .build();
            let mut path = CACHE.clone();
            path.push(format!("{}-toplist.jpg", t.id));
            let image = Image::from_file(path);
            image.set_pixel_size(40);
            action.add_prefix(&image);
            sidebar.append(&action);
            if !select {
                sidebar.select_row(Some(&action));
                select = true;
            }
        }
        self.imp().data.set(list).unwrap();
        self.imp().update_toplist_info(0);
    }

    pub fn update_songs_list(&self, sis: Vec<SongInfo>) {
        let imp = self.imp();
        imp.playlist.replace(sis.clone());
        imp.num_label
            .get()
            .set_label(&gettext!("{} songs", sis.len()));
        let sender = imp.sender.get().unwrap();
        let songs_list = imp.songs_list.get();
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
            songs_list.append(&row);
        });
    }
}

mod imp {

    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/gitee/gmg137/NeteaseCloudMusicGtk4/gtk/toplist.ui")]
    pub struct TopListView {
        #[template_child]
        pub sidebar: TemplateChild<ListBox>,
        #[template_child]
        pub cover_image: TemplateChild<Image>,
        #[template_child]
        pub title_label: TemplateChild<Label>,
        #[template_child]
        pub num_label: TemplateChild<Label>,
        #[template_child]
        pub play_button: TemplateChild<Button>,
        #[template_child]
        pub songs_list: TemplateChild<ListBox>,

        pub playlist: Rc<RefCell<Vec<SongInfo>>>,
        pub data: OnceCell<Vec<TopList>>,
        pub sender: OnceCell<Sender<Action>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for TopListView {
        const NAME: &'static str = "TopListView";
        type Type = super::TopListView;
        type ParentType = adw::Bin;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
            klass.bind_template_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[gtk::template_callbacks]
    impl TopListView {
        #[template_callback]
        fn sidebar_cb(&self, row: &ListBoxRow) {
            let index = row.index();
            self.update_toplist_info(index);
        }

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

        pub fn update_toplist_info(&self, index: i32) {
            let songs_list = self.songs_list.get();
            while let Some(child) = songs_list.last_child() {
                songs_list.remove(&child);
            }
            let data = self.data.get().unwrap();
            if let Some(info) = data.get(index as usize) {
                self.sender
                    .get()
                    .unwrap()
                    .send(Action::GetToplistSongsList(info.id))
                    .unwrap();
                let mut path = CACHE.clone();

                path.push(format!("{}-toplist.jpg", info.id));
                self.cover_image.set_from_file(Some(path));

                let title = self.title_label.get();
                title.set_label(&info.name);
            }
        }
    }

    impl ObjectImpl for TopListView {
        fn constructed(&self, obj: &Self::Type) {
            self.parent_constructed(obj);
            let select_row = Rc::new(RefCell::new(-1));
            self.songs_list.connect_row_activated(move |list, row| {
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
    impl WidgetImpl for TopListView {}
    impl BinImpl for TopListView {}
}
