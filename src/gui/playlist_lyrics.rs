//
// playlist_lyrics.rs
// Copyright (C) 2022 gmg137 <gmg137 AT live.com>
// Distributed under terms of the GPL-3.0-or-later license.
//
use super::SonglistRow;
use adw::subclass::prelude::BinImpl;
use gettextrs::gettext;
use glib::{ParamFlags, ParamSpec, ParamSpecInt, Sender, Value};
use gtk::{glib, prelude::*, subclass::prelude::*, CompositeTemplate, *};
use ncm_api::SongInfo;
use once_cell::sync::Lazy;
use once_cell::sync::OnceCell;
use std::cell::Cell;
use std::cell::RefCell;
use std::rc::Rc;

use crate::application::Action;

glib::wrapper! {
    pub struct PlayListLyricsPage(ObjectSubclass<imp::PlayListLyricsPage>)
        @extends Widget, Paned,
        @implements Accessible, Orientable, ConstraintTarget,Buildable;
}

impl PlayListLyricsPage {
    pub fn new() -> Self {
        glib::Object::new(&[]).expect("Failed to create PlayListLyricsPage")
    }

    pub fn set_sender(&self, sender: Sender<Action>) {
        self.imp().sender.set(sender).unwrap();
    }

    pub fn init_page(&self, sis: Vec<SongInfo>, si: SongInfo) {
        let imp = self.imp();
        // 删除旧内容
        let playlist_box = imp.playlist_box.get();
        while let Some(child) = playlist_box.last_child() {
            playlist_box.remove(&child);
        }
        self.update_playlist(sis, si);

        let lyrics_text_view = imp.lyrics_text_view.get();
        let buffer = lyrics_text_view.buffer();
        buffer.set_text(&gettext("Loading lyrics..."));
        lyrics_text_view.set_buffer(Some(&buffer));
    }

    pub fn update_playlist(&self, sis: Vec<SongInfo>, current_song: SongInfo) {
        let imp = self.imp();
        imp.playlist.replace(sis.clone());
        let sender = imp.sender.get().unwrap();
        let listbox = imp.playlist_box.get();
        let mut index = 0;
        sis.into_iter().for_each(|si| {
            let row = SonglistRow::new();
            row.set_tooltip_text(Some(&si.name));

            row.set_name(&si.name);
            row.set_singer(&si.singer);
            row.set_album(&si.album);
            row.set_duration(&si.duration);

            if current_song.id == si.id {
                row.switch_image(true);
                self.set_property("select-row", index);
            }

            let sender = sender.clone();
            row.connect_activate(move |row| {
                row.switch_image(true);
                sender.send(Action::AddPlay(si.clone())).unwrap();
                sender.send(Action::GetLyrics(si.clone())).unwrap();
            });
            listbox.append(&row);
            index += 1;
        });
    }

    pub fn update_lyrics(&self, lyrics: String) {
        let lyrics_text_view = self.imp().lyrics_text_view.get();
        let buffer = lyrics_text_view.buffer();
        buffer.set_text(&lyrics);
        lyrics_text_view.set_buffer(Some(&buffer));
    }

    pub fn switch_row(&self, index: i32) {
        let imp = self.imp();
        let listbox = imp.playlist_box.get();
        let current_row_index: i32 = self.property("select-row");
        if let Some(row) = listbox.row_at_index(current_row_index) {
            let row = row.downcast::<SonglistRow>().unwrap();
            row.switch_image(false);
        }

        self.set_property("select-row", index);
        if let Some(row) = listbox.row_at_index(index) {
            let row = row.downcast::<SonglistRow>().unwrap();
            row.switch_image(true);
        }
    }
}

impl Default for PlayListLyricsPage {
    fn default() -> Self {
        Self::new()
    }
}

mod imp {

    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/gitee/gmg137/NeteaseCloudMusicGtk4/gtk/playlist-lyrics-page.ui")]
    pub struct PlayListLyricsPage {
        #[template_child]
        pub playlist_box: TemplateChild<ListBox>,
        #[template_child]
        pub lyrics_text_view: TemplateChild<TextView>,
        pub playlist: Rc<RefCell<Vec<SongInfo>>>,
        pub sender: OnceCell<Sender<Action>>,
        select_row: Cell<i32>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for PlayListLyricsPage {
        const NAME: &'static str = "PlayListLyricsPage";
        type Type = super::PlayListLyricsPage;
        type ParentType = adw::Bin;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for PlayListLyricsPage {
        fn constructed(&self, obj: &Self::Type) {
            self.parent_constructed(obj);

            self.playlist_box.connect_row_activated(
                glib::clone!(@weak obj as s => move |list, row| {
                    let index = s.property("select-row");
                    if index != -1 && index != row.index() {
                        s.set_property("select-row", row.index());
                        if let Some(row) = list.row_at_index(index) {
                            let row = row.downcast::<SonglistRow>().unwrap();
                            row.switch_image(false);
                        }
                    } else {
                        s.set_property("select-row", row.index());
                    }
                }),
            );
        }

        fn properties() -> &'static [ParamSpec] {
            static PROPERTIES: Lazy<Vec<ParamSpec>> = Lazy::new(|| {
                vec![ParamSpecInt::new(
                    // Name
                    "select-row",
                    // Nickname
                    "select-row",
                    // Short description
                    "Current select row index",
                    // Minimum value
                    i32::MIN,
                    // Maximum value
                    i32::MAX,
                    // Default value
                    -1,
                    // The property can be read and written to
                    ParamFlags::READWRITE,
                )]
            });
            PROPERTIES.as_ref()
        }

        fn set_property(&self, _obj: &Self::Type, _id: usize, value: &Value, pspec: &ParamSpec) {
            match pspec.name() {
                "select-row" => {
                    let input_number = value.get().expect("The value needs to be of type `i32`.");
                    self.select_row.replace(input_number);
                }
                _ => unimplemented!(),
            }
        }

        fn property(&self, _obj: &Self::Type, _id: usize, pspec: &ParamSpec) -> Value {
            match pspec.name() {
                "select-row" => self.select_row.get().to_value(),
                _ => unimplemented!(),
            }
        }
    }
    impl WidgetImpl for PlayListLyricsPage {}
    impl BinImpl for PlayListLyricsPage {}
}
