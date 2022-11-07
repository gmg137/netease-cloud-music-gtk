//
// my_page.rs
// Copyright (C) 2022 gmg137 <gmg137 AT live.com>
// Distributed under terms of the GPL-3.0-or-later license.
//

use glib::Sender;
use gtk::{glib, prelude::*, subclass::prelude::*, CompositeTemplate};
use ncm_api::SongList;
use once_cell::sync::OnceCell;

use crate::{application::Action, gui::SongListGridItem};

glib::wrapper! {
    pub struct MyPage(ObjectSubclass<imp::MyPage>)
        @extends gtk::Widget, gtk::Box,
        @implements gtk::Accessible, gtk::Buildable,gtk::ConstraintTarget, gtk::Orientable;
}

impl MyPage {
    pub fn new() -> Self {
        glib::Object::new(&[])
    }

    pub fn set_sender(&self, sender: Sender<Action>) {
        self.imp().sender.set(sender).unwrap();
    }

    pub fn init_page(&self, song_list: Vec<SongList>) {
        let imp = self.imp();
        let rec_grid = imp.rec_grid.get();
        SongListGridItem::box_clear(rec_grid);
        self.setup_rec_grid(song_list);
    }

    fn setup_rec_grid(&self, song_list: Vec<SongList>) {
        let sender = self.imp().sender.get().unwrap().clone();
        let top_picks = self.imp().rec_grid.get();

        SongListGridItem::box_update_songlist(top_picks.clone(), &song_list, 140, false, &sender);

        top_picks.connect_child_activated(move |_, child| {
            let index = child.index() as usize;
            if let Some(sl) = song_list.get(index) {
                sender.send(Action::ToSongListPage(sl.clone())).unwrap();
            }
        });
    }
}

impl Default for MyPage {
    fn default() -> Self {
        Self::new()
    }
}

mod imp {

    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/gitee/gmg137/NeteaseCloudMusicGtk4/gtk/my-page.ui")]
    pub struct MyPage {
        #[template_child]
        pub rec_grid: TemplateChild<gtk::FlowBox>,
        #[template_child]
        pub daily_rec_avatar: TemplateChild<adw::Avatar>,

        pub sender: OnceCell<Sender<Action>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for MyPage {
        const NAME: &'static str = "MyPage";
        type Type = super::MyPage;
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
    impl MyPage {
        #[template_callback]
        fn daily_rec_cb(&self) {
            let sender = self.sender.get().unwrap();
            sender.send(Action::ToMyPageDailyRec).unwrap();
        }

        #[template_callback]
        fn heartbeat_cb(&self) {
            let sender = self.sender.get().unwrap();
            sender.send(Action::ToMyPageHeartbeat).unwrap();
        }

        #[template_callback]
        fn fm_cb(&self) {
            let sender = self.sender.get().unwrap();
            sender.send(Action::ToMyPageFm).unwrap();
        }

        #[template_callback]
        fn cloud_disk_cb(&self) {
            let sender = self.sender.get().unwrap();
            sender.send(Action::ToMyPageCloudDisk).unwrap();
        }

        #[template_callback]
        fn collection_album_cb(&self) {
            let sender = self.sender.get().unwrap();
            sender.send(Action::ToMyPageAlbums).unwrap();
        }

        #[template_callback]
        fn collection_songlist_cb(&self) {
            let sender = self.sender.get().unwrap();
            sender.send(Action::ToMyPageSonglist).unwrap();
        }
    }

    impl ObjectImpl for MyPage {
        fn constructed(&self) {
            self.parent_constructed();
            if let Ok(datetime) = glib::DateTime::now_local() {
                self.daily_rec_avatar.set_show_initials(true);
                self.daily_rec_avatar.set_text(Some(&format!(
                    "{} {}",
                    datetime.day_of_month() / 10,
                    datetime.day_of_month() % 10
                )));
            }
        }
    }
    impl WidgetImpl for MyPage {}
    impl BoxImpl for MyPage {}
}
