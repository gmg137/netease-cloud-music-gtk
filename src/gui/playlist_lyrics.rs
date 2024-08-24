//
// playlist_lyrics.rs
// Copyright (C) 2022 gmg137 <gmg137 AT live.com>
// Distributed under terms of the GPL-3.0-or-later license.
//
use adw::subclass::prelude::BinImpl;
use async_channel::Sender;
use gettextrs::gettext;
use glib::{closure_local, ParamSpec, Value};
use gtk::{glib, prelude::*, subclass::prelude::*, CompositeTemplate, *};
use ncm_api::SongInfo;
use once_cell::sync::Lazy;
use once_cell::sync::OnceCell;
use std::cell::RefCell;
use std::rc::Rc;

use crate::{
    application::Action,
    gui::{songlist_row::SonglistRow, songlist_view::SongListView},
};

glib::wrapper! {
    pub struct PlayListLyricsPage(ObjectSubclass<imp::PlayListLyricsPage>)
        @extends Widget, Paned,
        @implements Accessible, Orientable, ConstraintTarget,Buildable;
}

impl PlayListLyricsPage {
    pub fn new() -> Self {
        glib::Object::new()
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
                    sender.send_blocking(Action::UpdateLyrics(si, 0)).unwrap();
                }
            }));
    }

    pub fn init_page(&self, sis: &[SongInfo], si: SongInfo, likes: &[bool]) {
        let imp = self.imp();
        // 删除旧内容
        let songs_list = imp.songs_list.get();
        songs_list.clear_list();
        self.update_playlist(sis, si, likes);
        self.update_font_size();
        self.setup_scroll_controller();

        let lyrics_text_view = imp.lyrics_text_view.get();
        let buffer = lyrics_text_view.buffer();
        buffer.set_text(&gettext("Loading lyrics..."));
        lyrics_text_view.set_buffer(Some(&buffer));
    }

    fn setup_scroll_controller(&self) {
        let scroll_win = self.imp().scroll_lyrics_win.get();
        let scrolled = self.imp().scrolled.clone();

        let scroll_controller = EventControllerScroll::new(EventControllerScrollFlags::VERTICAL);
        scroll_controller.connect_scroll(move |_, _, _| {
            {
                let mut val = scrolled.lock().unwrap();
                *val += 1;
            }
            let scrolled = scrolled.clone();
            glib::timeout_add_seconds(3, move || {
                let mut val = scrolled.lock().unwrap();
                *val -= 1;
                glib::ControlFlow::Break
            });
            glib::Propagation::Proceed
        });
        scroll_win.add_controller(scroll_controller);
    }

    fn update_font_size(&self) {
        let imp = self.imp();
        let lyrics_text_view = imp.lyrics_text_view.get();
        let pango_context = lyrics_text_view.pango_context();
        let font_description = pango_context
            .font_description()
            .expect("expect font description");
        let font_size = font_description.size();

        let font_size_in_pixels = if font_description.is_size_absolute() {
            font_size as f64 / pango::SCALE as f64
        } else {
            font_size as f64 / pango::SCALE as f64
        };

        imp.font_size.replace(font_size_in_pixels);
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

    pub fn update_lyrics(&self, lyrics: Vec<(u64, String)>, time: u64) {
        let mut lyrics = lyrics;
        // 填充空白行，以使用window(3)方法时可以到达最后一行
        lyrics.push((3600000000, "".to_string()));
        lyrics.push((3600000000, "".to_string()));
        let lyrics_text_view = self.imp().lyrics_text_view.get();
        let scroll_win = self.imp().scroll_lyrics_win.get();
        let adjustment = scroll_win.vadjustment();
        let height = scroll_win.allocated_height();

        // the default value of line-height is normal, so the line-height in pixel is
        // font-size * 1.2(normal)
        let line_height = self.imp().font_size.get() * 1.2;

        let buffer = lyrics_text_view.buffer();
        buffer.set_text("");
        let mut iter = buffer.start_iter();
        let mut playing_index = 0;
        for (i, lyr) in lyrics.windows(3).enumerate() {
            if (time >= lyr[0].0 && time < lyr[1].0)
                || lyr[0].0 == lyr[1].0 && time >= lyr[0].0 && time < lyr[2].0
            {
                if playing_index == 0 {
                    playing_index = i;
                }
                buffer.insert_markup(
                    &mut iter,
                    &format!(
                        r#"<span size="large" weight="bold" color="red">{}</span>"#,
                        lyr[0].1.replace("&", "&amp;")
                    ),
                );
            } else {
                buffer.insert(&mut iter, &lyr[0].1);
            }
        }
        if *(self.imp().scrolled.lock().unwrap()) == 0 {
            let offset = line_height * playing_index as f64 - height as f64 / 2.0 - line_height / 2.0
                    + 18.0 // text top-margin
                    + 10.0; // text view margin-top
            adjustment.set_value(offset.max(0f64));
        }
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

    use std::{
        cell::Cell,
        sync::{Arc, Mutex},
    };

    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/gitee/gmg137/NeteaseCloudMusicGtk4/gtk/playlist-lyrics-page.ui")]
    pub struct PlayListLyricsPage {
        #[template_child]
        pub songs_list: TemplateChild<SongListView>,
        #[template_child]
        pub scroll_lyrics_win: TemplateChild<ScrolledWindow>,
        #[template_child]
        pub lyrics_text_view: TemplateChild<TextView>,
        pub(crate) font_size: Cell<f64>,
        pub(crate) scrolled: Arc<Mutex<usize>>,
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
