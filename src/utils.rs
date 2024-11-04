use gettextrs::gettext;
use glib::{timeout_add_seconds, SourceId};
use gtk::glib;
use std::sync::{Arc, Mutex};

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
