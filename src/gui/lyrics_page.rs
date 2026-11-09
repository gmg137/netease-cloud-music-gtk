//
// lyrics_page.rs
// Copyright (C) 2022 gmg137 <gmg137 AT live.com>
// Distributed under terms of the GPL-3.0-or-later license.
//
// 官方客户端风格的大封面歌词页：点击播放栏封面进入。
//
use adw::subclass::prelude::BinImpl;
use async_channel::Sender;
use gettextrs::gettext;
use gtk::gdk_pixbuf::{InterpType, Pixbuf};
use gtk::{
    gdk,
    glib::{self, ParamSpec, Value},
    prelude::*,
    subclass::prelude::*,
    *,
};
use log::*;
use ncm_api::{SongInfo, SongList};
use once_cell::sync::{Lazy, OnceCell};
use std::cell::{Cell, RefCell};
use std::path::Path;
use std::sync::{Arc, RwLock};

use crate::{application::Action, path::CACHE};

glib::wrapper! {
    pub struct LyricsPage(ObjectSubclass<imp::LyricsPage>)
        @extends adw::Bin, Widget,
        @implements Accessible, Buildable, ConstraintTarget;
}

impl LyricsPage {
    pub fn new() -> Self {
        glib::Object::new()
    }

    pub fn set_sender(&self, sender: Sender<Action>) {
        let _ = self.imp().sender.set(sender);
    }

    /// 更新左侧封面与歌曲信息；同一首歌重复调用不会重复下载封面。
    pub fn set_song(&self, si: &SongInfo) {
        let imp = self.imp();
        if imp.song_id.get() == si.id && si.id != 0 {
            return;
        }
        // 换歌后先清掉旧歌词，等新歌词返回
        if imp.song_id.get() != si.id {
            self.update_lyrics_text(&gettext("Loading lyrics..."));
        }
        imp.song_id.set(si.id);
        imp.album_id.set(si.album_id);
        imp.pic_url.replace(si.pic_url.clone());
        imp.title_label.get().set_label(&si.name);
        imp.artist_label.get().set_label(&si.singer);
        imp.album_label.get().set_label(&si.album);

        let mut path = CACHE.clone();
        path.push(format!("{}-songlist.jpg", si.album_id));
        if path.exists() {
            self.apply_cover(&path);
        } else {
            imp.cover_picture
                .get()
                .set_paintable(None::<&gdk::Paintable>);
            imp.bg_picture
                .get()
                .set_paintable(None::<&gdk::Paintable>);
            if si.pic_url.is_empty() {
                return;
            }
            let Some(sender) = imp.sender.get().cloned() else {
                return;
            };
            let page = glib::SendWeakRef::from(self.downgrade());
            let cover_path = path.clone();
            let _ = sender.try_send(Action::DownloadImage(
                si.pic_url.clone(),
                path,
                600,
                600,
                Some(Arc::new(move |_| {
                    if let Some(page) = page.upgrade() {
                        page.apply_cover(&cover_path);
                    }
                })),
            ));
        }
    }

    /// 元数据补全后强制重画（同一首歌也要重新找封面）
    pub fn force_set_song(&self, si: &SongInfo) {
        self.imp().song_id.set(0);
        self.set_song(si);
    }

    pub fn update_lyrics_text(&self, text: &str) {
        let imp = self.imp();
        imp.buffer.get().set_text(text);
        imp.current_lyrics.write().unwrap().clear();
    }

    pub fn has_lyrics(&self) -> bool {
        !self.imp().current_lyrics.read().unwrap().is_empty()
    }

    pub fn update_lyrics(&self, lyrics: Vec<(u64, String)>) {
        let imp = self.imp();
        // 每条歌词独占一行，保证「缓冲行号 == 歌词下标」，点击跳转才能对得上
        let text = lyrics
            .iter()
            .map(|(_, line)| format!("{}\n", line.trim_end_matches('\n')))
            .collect::<String>();
        imp.buffer.get().set_text(&text);
        *imp.current_lyrics.write().unwrap() = lyrics;
    }

    pub fn update_lyrics_highlight(&self, time: u64) {
        let imp = self.imp();
        let lyrics = imp.current_lyrics.read().unwrap().clone();
        let Some((start, end)) = crate::gui::get_playing_indexes(lyrics, time) else {
            return;
        };
        let Some(mark) = self.set_lyrics_highlight(start as i32, end as i32) else {
            return;
        };
        // 与官方一致：当前行停在靠中上位置，且用户手动滚动后暂停自动滚动
        if *(imp.scrolled.lock().unwrap()) == 0 {
            imp.lyrics_text_view
                .get()
                .scroll_to_mark(&mark, 0.0, true, 0.0, 0.45);
        }
    }

    fn apply_cover(&self, path: &Path) {
        let imp = self.imp();
        let pixbuf = match Pixbuf::from_file_at_scale(path, 600, 600, true) {
            Ok(pixbuf) => pixbuf,
            Err(e) => {
                warn!("歌词页加载封面失败 {:?}: {}", path, e);
                return;
            }
        };

        imp.cover_picture
            .get()
            .set_paintable(Some(&gdk::Texture::for_pixbuf(&pixbuf)));

        // 背景：先缩到 32x32 再放大，用双线性插值得到柔和的模糊封面
        if let Some(small) = pixbuf.scale_simple(32, 32, InterpType::Bilinear) {
            if let Some(blurred) = small.scale_simple(900, 900, InterpType::Bilinear) {
                imp.bg_picture
                    .get()
                    .set_paintable(Some(&gdk::Texture::for_pixbuf(&blurred)));
            }
        }
    }

    fn set_lyrics_highlight(&self, line_start: i32, line_end: i32) -> Option<TextMark> {
        let imp = self.imp();
        let buffer = imp.buffer.get();
        let highlight_text_tag = imp.highlight_text_tag.get();
        let uttered_text_tag = imp.uttered_text_tag.get();

        buffer.remove_tag(
            &highlight_text_tag,
            &buffer.start_iter(),
            &buffer.end_iter(),
        );
        buffer.remove_tag(
            &uttered_text_tag,
            &buffer.start_iter(),
            &buffer.end_iter(),
        );

        let mut mark_to_return = None;
        for i in 0..line_start {
            let Some(start) = buffer.iter_at_line(i) else {
                continue;
            };
            let mut end = start;
            if !start.ends_line() {
                end.forward_to_line_end();
            }
            buffer.apply_tag(&uttered_text_tag, &start, &end);
        }
        for i in line_start..=line_end {
            let Some(start) = buffer.iter_at_line(i) else {
                continue;
            };
            if mark_to_return.is_none() {
                mark_to_return = Some(buffer.create_mark(None, &start, true));
            }
            let mut end = start;
            if !start.ends_line() {
                end.forward_to_line_end();
            }
            buffer.apply_tag(&highlight_text_tag, &start, &end);
        }

        mark_to_return
    }
}

impl Default for LyricsPage {
    fn default() -> Self {
        Self::new()
    }
}

#[gtk::template_callbacks]
impl LyricsPage {
    // 点击封面回到专辑页
    #[template_callback]
    fn cover_clicked_cb(&self) {
        let imp = self.imp();
        if imp.song_id.get() == 0 || imp.album_id.get() == 0 {
            return;
        }
        let Some(sender) = imp.sender.get().cloned() else {
            return;
        };
        let songlist = SongList {
            id: imp.album_id.get(),
            name: imp.album_label.get().label().to_string(),
            cover_img_url: imp.pic_url.borrow().clone(),
            author: String::new(),
            special_type: 0,
        };
        let _ = sender.try_send(Action::ToAlbumPage(songlist));
    }

    // 点击某行歌词跳转到对应时间
    #[template_callback]
    fn lyrics_clicked_cb(&self, _n_press: i32, x: f64, y: f64) {
        let imp = self.imp();
        let text_view = imp.lyrics_text_view.get();
        let Some(iter) = text_view.iter_at_location(x as i32, y as i32) else {
            return;
        };
        let buffer = imp.buffer.get();
        let before = buffer.text(&buffer.start_iter(), &iter, true);
        let line = before.chars().filter(|c| *c == '\n').count();
        let time = match imp.current_lyrics.read().unwrap().get(line) {
            Some((time, _)) => *time,
            None => return,
        };
        if let Some(sender) = imp.sender.get().cloned() {
            // 歌词时间戳单位是毫秒，播放器 seek 使用微秒
            let _ = sender.try_send(Action::SeekTo(time * 1000));
        }
    }
}

mod imp {
    use super::*;
    use std::sync::Mutex;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/gitee/gmg137/NeteaseCloudMusicGtk4/gtk/lyrics-page.ui")]
    pub struct LyricsPage {
        #[template_child]
        pub bg_picture: TemplateChild<Picture>,
        #[template_child]
        pub cover_picture: TemplateChild<Picture>,
        #[template_child]
        pub title_label: TemplateChild<Label>,
        #[template_child]
        pub artist_label: TemplateChild<Label>,
        #[template_child]
        pub album_label: TemplateChild<Label>,
        #[template_child]
        pub scroll_lyrics_win: TemplateChild<ScrolledWindow>,
        #[template_child]
        pub lyrics_text_view: TemplateChild<TextView>,
        #[template_child]
        pub buffer: TemplateChild<TextBuffer>,
        #[template_child]
        pub highlight_text_tag: TemplateChild<TextTag>,
        #[template_child]
        pub uttered_text_tag: TemplateChild<TextTag>,

        pub current_lyrics: Arc<RwLock<Vec<(u64, String)>>>,
        pub scrolled: Arc<Mutex<usize>>,
        pub sender: OnceCell<Sender<Action>>,
        pub song_id: Cell<u64>,
        pub album_id: Cell<u64>,
        pub pic_url: RefCell<String>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for LyricsPage {
        const NAME: &'static str = "LyricsPage";
        type Type = super::LyricsPage;
        type ParentType = adw::Bin;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_instance_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for LyricsPage {
        fn constructed(&self) {
            self.parent_constructed();
            load_css();

            // 用户手动滚动歌词后 3 秒内不自动跟随
            let scrolled = self.scrolled.clone();
            let scroll_controller =
                EventControllerScroll::new(EventControllerScrollFlags::VERTICAL);
            scroll_controller.connect_scroll(move |_, _, _| {
                *scrolled.lock().unwrap() += 1;
                let scrolled = scrolled.clone();
                glib::timeout_add_seconds(3, move || {
                    let mut val = scrolled.lock().unwrap();
                    *val = val.saturating_sub(1);
                    glib::ControlFlow::Break
                });
                glib::Propagation::Proceed
            });
            self.scroll_lyrics_win.add_controller(scroll_controller);
        }

        fn properties() -> &'static [ParamSpec] {
            static PROPERTIES: Lazy<Vec<ParamSpec>> = Lazy::new(Vec::new);
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

    impl WidgetImpl for LyricsPage {}
    impl BinImpl for LyricsPage {}
}

fn load_css() {
    let provider = CssProvider::new();
    provider.load_from_resource("/com/gitee/gmg137/NeteaseCloudMusicGtk4/themes/lyrics.css");
    style_context_add_provider_for_display(
        &gdk::Display::default().expect("Could not connect to a display."),
        &provider,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
}
