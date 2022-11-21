//
// playlist_lyrics.rs
// Copyright (C) 2022 gmg137 <gmg137 AT live.com>
// Distributed under terms of the GPL-3.0-or-later license.
//
use adw::subclass::prelude::BinImpl;
use gettextrs::gettext;
use glib::{closure_local, ParamSpec, Sender, Value};
use gtk::{glib, prelude::*, subclass::prelude::*, CompositeTemplate, *};
use ncm_api::SongInfo;
use once_cell::sync::Lazy;
use once_cell::sync::OnceCell;
use std::cell::RefCell;
use std::rc::Rc;

use crate::application::Action;
use crate::gui::{songlist_row::SonglistRow, songlist_view::SongListView};

glib::wrapper! {
    pub struct PlayListLyricsPage(ObjectSubclass<imp::PlayListLyricsPage>)
        @extends Widget, Paned,
        @implements Accessible, Orientable, ConstraintTarget,Buildable;
}

impl PlayListLyricsPage {
    pub fn new() -> Self {
        glib::Object::new(&[])
    }

    pub fn set_sender(&self, sender_: Sender<Action>) {
        let sender = sender_.clone();
        self.imp().sender.set(sender).unwrap();

        let sender = sender_;
        self.imp()
            .songs_list
            .get()
            .connect_row_activated(closure_local!(move |_: SongListView, row: SonglistRow| {
                if let Some(si) = row.get_song_info() {
                    sender.send(Action::UpdateLyrics(si)).unwrap();
                }
            }));
    }

    pub fn init_page(&self, sis: &[SongInfo], si: SongInfo, likes: &[bool]) {
        let imp = self.imp();
        // 删除旧内容
        let songs_list = imp.songs_list.get();
        songs_list.clear_list();
        self.update_playlist(sis, si, likes);

        let lyrics_text_view = imp.lyrics_text_view.get();
        let buffer = lyrics_text_view.buffer();
        buffer.set_text(&gettext("Loading lyrics..."));
        lyrics_text_view.set_buffer(Some(&buffer));
    }

    pub fn update_playlist(&self, sis: &[SongInfo], current_song: SongInfo, likes: &[bool]) {
        let imp = self.imp();
        imp.playlist.replace(Clone::clone(&sis).to_vec());
        let sender = imp.sender.get().unwrap();
        let songs_list = imp.songs_list.get();
        songs_list.set_sender(sender.clone());
        songs_list.init_new_list(sis, likes);

        let i: i32 = {
            let mut i: i32 = 0;
            match sis.iter().find(|si| {
                i += 1;
                si.id == current_song.id
            }) {
                Some(_) => i - 1,
                _ => -1,
            }
        };
        self.switch_row(i);
    }

    pub fn update_lyrics(&self, lyrics: String) {
        let lyrics_text_view = self.imp().lyrics_text_view.get();
        let buffer = lyrics_text_view.buffer();
        buffer.set_text(&lyrics);
        lyrics_text_view.set_buffer(Some(&buffer));
    }

    pub fn switch_row(&self, index: i32) {
        self.imp().songs_list.mark_new_row_playing(index, false);
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
        pub songs_list: TemplateChild<SongListView>,
        #[template_child]
        pub lyrics_text_view: TemplateChild<TextView>,
        pub playlist: Rc<RefCell<Vec<SongInfo>>>,
        pub sender: OnceCell<Sender<Action>>,
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
        fn constructed(&self) {
            let _obj = self.obj();
            self.parent_constructed();
        }

        fn properties() -> &'static [ParamSpec] {
            static PROPERTIES: Lazy<Vec<ParamSpec>> = Lazy::new(std::vec::Vec::new);
            PROPERTIES.as_ref()
        }

        fn set_property(&self, _id: usize, _value: &Value, pspec: &ParamSpec) {
            pspec.name();
            unimplemented!()
        }

        fn property(&self, _id: usize, pspec: &ParamSpec) -> Value {
            pspec.name();
            unimplemented!()
        }
    }
    impl WidgetImpl for PlayListLyricsPage {}
    impl BinImpl for PlayListLyricsPage {}
}
