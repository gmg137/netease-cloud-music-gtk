use gettextrs::gettext;
use glib::{SourceId, timeout_add_seconds};
use gtk::{Label, glib, prelude::*};
use ncm_api::{SongCopyright, SongInfo, SongQuality, SongQualityState};
use std::sync::{Arc, Mutex};

/// 音质的可读名称
pub fn quality_name(quality: SongQuality) -> String {
    match quality {
        SongQuality::Standard => gettext("Standard"),
        SongQuality::Higher => gettext("Higher"),
        SongQuality::Extreme => gettext("Extreme"),
        SongQuality::Lossless => gettext("Lossless"),
        SongQuality::HiRes => gettext("Hi-Res"),
        SongQuality::Surround => gettext("Surround"),
        SongQuality::AudioVivid => gettext("Audio Vivid"),
        SongQuality::Master => gettext("Master"),
    }
}

/// 歌曲当前实际音质（没取到实际值时退回到选中音质）
pub fn current_quality(quality: &SongQualityState) -> Option<SongQuality> {
    quality.actual.or(quality.selected)
}

/// 可用音质列表文本，如 `Standard / Higher / Extreme`
pub fn quality_list_text(quality: &SongQualityState) -> String {
    quality
        .available
        .iter()
        .map(|q| quality_name(*q))
        .collect::<Vec<String>>()
        .join(" / ")
}

/// Like `gettext`, but replaces named variables with the given dictionary.
///
/// The expected format to replace is `{name}`, where `name` is the first string
/// in the dictionary entry tuple.
/// 使用 xtr 生成 pot 文件时需添加参数: xtr -k gettext_f -k gettext -o NAME.pot src/main.rs
pub fn gettext_f(msgid: &str, args: &[(&str, &str)]) -> String {
    let s = gettext(msgid);
    freplace(s, args)
}

/// Replace variables in the given string with the given dictionary.
///
/// The expected format to replace is `{name}`, where `name` is the first string
/// in the dictionary entry tuple.
pub fn freplace(s: String, args: &[(&str, &str)]) -> String {
    let mut s = s;

    for (k, v) in args {
        s = s.replace(&format!("{{{k}}}"), v);
    }

    s
}

#[derive(Debug)]
pub struct Debounce {
    timer_id: Arc<Mutex<Option<SourceId>>>,
}

impl Debounce {
    pub fn new() -> Self {
        Self {
            timer_id: Arc::new(Mutex::new(None)),
        }
    }
    pub fn debounce<F>(&self, delay: u32, callback: F)
    where
        F: Fn() + 'static + Send,
    {
        let timer_id_clone = self.timer_id.clone();

        if let Some(source_id) = timer_id_clone.lock().unwrap().take() {
            source_id.remove();
        }

        let timer_id_closure = timer_id_clone.clone();
        let new_timer_id = timeout_add_seconds(delay, move || {
            callback();
            timer_id_closure.lock().unwrap().take();
            glib::ControlFlow::Break
        });

        let mut guard = timer_id_clone.lock().unwrap();
        *guard = Some(new_timer_id);
    }
}

impl Default for Debounce {
    fn default() -> Self {
        Self::new()
    }
}

pub fn empty_song_info() -> SongInfo {
    SongInfo {
        id: 0,
        name: String::new(),
        singer: String::new(),
        album: String::new(),
        album_id: 0,
        pic_url: String::new(),
        duration: 0,
        song_url: String::new(),
        quality: SongQualityState::default(),
        copyright: SongCopyright::Unknown,
    }
}

/// 仅当 `Label` 文本被省略号截断时，悬停才显示完整文本。
///
/// 用于列表行或页面顶部标题等可能被 `ellipsize` 截断的标签。
pub fn setup_ellipsis_tooltip(label: &Label) {
    label.set_has_tooltip(true);
    label.connect_query_tooltip(|label, _x, _y, _kb, tooltip| {
        if label.layout().is_ellipsized() {
            tooltip.set_text(Some(label.label().as_str()));
            true
        } else {
            false
        }
    });
}
