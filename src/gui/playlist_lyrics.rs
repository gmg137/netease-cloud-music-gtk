//
// playlist_lyrics.rs
// Copyright (C) 2022 gmg137 <gmg137 AT live.com>
// Distributed under terms of the GPL-3.0-or-later license.
//
use adw::subclass::prelude::BinImpl;
use async_channel::Sender;
use glib::{closure_local, ParamSpec, Value};
use gtk::{glib, prelude::*, subclass::prelude::*, CompositeTemplate, *};
use log::warn;
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
        @extends adw::Bin, Widget, Paned,
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
        self.setup_scroll_controller();
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

    pub fn update_lyrics_text(&self, text: &str) {
        let buffer = self.imp().buffer.get();
        buffer.set_text(text);
    }

    pub fn update_lyrics(&self, lyrics: Vec<(u64, String)>) {
        let buffer = self.imp().buffer.get();
        buffer.set_text(
            &lyrics
                .iter()
                .map(|(_, x)| x.to_owned())
                .collect::<Vec<_>>()
                .join(""),
        );
        let mut current_lyrics = self.imp().current_lyrics.write().unwrap();
        *current_lyrics = lyrics;
    }

    pub fn update_lyrics_highlight(&self, time: u64) {
        let lyrics_text_view = self.imp().lyrics_text_view.get();
        let lyrics = self.imp().current_lyrics.read().unwrap().clone();
        let playing_indexes = get_playing_indexes(lyrics, time);
        if playing_indexes.is_none() {
            // 没有行需要高亮
            return;
        }
        let (start, end) = playing_indexes.unwrap();
        let center_mark = self.set_lyrics_highlight(start as i32, end as i32);

        if let Some(mark) = center_mark {
            if *(self.imp().scrolled.lock().unwrap()) == 0 {
                lyrics_text_view.scroll_to_mark(&mark, 0.0, true, 0.0, 0.5);
            }
        }
    }

    fn set_lyrics_highlight(&self, line_start: i32, line_end: i32) -> Option<TextMark> {
        let highlight_text_tag = self.imp().highlight_text_tag.get();
        let buffer = self.imp().buffer.get();

        let mut mark_to_return = None;
        buffer.remove_tag(
            &highlight_text_tag,
            &buffer.start_iter(),
            &buffer.end_iter(),
        );
        // gtk doesn't seem to be happy to apply tags to a multi-line TextIter region after an immediate `remove_tag``, so we apply tags line by line
        for i in line_start..=line_end {
            let start = buffer.iter_at_line(i);
            if start.is_none() {
                continue;
            }
            let start = start.unwrap();
            if mark_to_return.is_none() {
                mark_to_return = Some(buffer.create_mark(None, &start, true))
            }
            let mut end = start;
            if !start.ends_line() {
                end.forward_to_line_end();
            }
            buffer.apply_tag(&highlight_text_tag, &start, &end);
        }

        mark_to_return
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

    use std::sync::{Arc, Mutex, RwLock};

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
        #[template_child]
        pub buffer: TemplateChild<TextBuffer>,
        #[template_child]
        pub highlight_text_tag: TemplateChild<TextTag>,
        pub(crate) scrolled: Arc<Mutex<usize>>,
        pub playlist: Rc<RefCell<Vec<SongInfo>>>,
        pub sender: OnceCell<Sender<Action>>,
        pub current_lyrics: Arc<RwLock<Vec<(u64, String)>>>,
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

fn get_playing_indexes(mut lyrics: Vec<(u64, String)>, time: u64) -> Option<(usize, usize)> {
    // 填充空白行，以使用window(3)方法时可以到达最后一行
    lyrics.push((3600000000, "".to_string()));
    lyrics.push((3600000000, "".to_string()));
    for (i, lyr) in lyrics.windows(3).enumerate() {
        if (time >= lyr[0].0 && time < lyr[1].0)
            || lyr[0].0 == lyr[1].0 && time >= lyr[0].0 && time < lyr[2].0
        {
            if lyr[0].0 == lyr[1].0 {
                // 也包含翻译行
                warn!("also has translation {}", i);
                return Some((i, i + 1));
            } else {
                warn!("no translation {}", i);
                // 不包含翻译行
                return Some((i, i));
            }
        }
    }
    None
}
