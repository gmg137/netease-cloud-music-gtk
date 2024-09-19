use glib::{timeout_add_seconds, SourceId};
use gtk::glib;
use std::sync::{Arc, Mutex};
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
