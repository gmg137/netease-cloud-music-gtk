//
// system_tray.rs
// Copyright (C) 2022 gmg137 <gmg137 AT live.com>
// Distributed under terms of the GPL-3.0-or-later license.
//

use async_channel::Sender;
use gettextrs::gettext;
use gtk::gdk_pixbuf::{InterpType, Pixbuf};
use ksni::{Icon, MenuItem, Tray, TrayService, menu::StandardItem};
use log::*;
use std::io::Cursor;
use std::sync::{Arc, Mutex};

use crate::application::Action;

const APP_ICON: &[u8] = include_bytes!("../../data/icons/hicolor/512x512@2x.png");

fn pixbuf_to_argb32(pixbuf: &Pixbuf) -> (Vec<u8>, i32, i32) {
    let width = pixbuf.width();
    let height = pixbuf.height();
    let rowstride = pixbuf.rowstride() as usize;
    let n_channels = pixbuf.n_channels() as usize;
    let pixels = unsafe { pixbuf.pixels() };

    let mut argb_data = Vec::with_capacity((width * height * 4) as usize);
    for y in 0..height {
        for x in 0..width {
            let offset = (y as usize) * rowstride + (x as usize) * n_channels;
            let r = pixels[offset];
            let g = pixels[offset + 1];
            let b = pixels[offset + 2];
            let a = if n_channels >= 4 { pixels[offset + 3] } else { 255 };
            argb_data.push(a);
            argb_data.push(r);
            argb_data.push(g);
            argb_data.push(b);
        }
    }
    (argb_data, width, height)
}

fn load_app_icon() -> Vec<Icon> {
    match Pixbuf::from_read(Cursor::new(APP_ICON)) {
        Ok(pixbuf) => {
            let (argb_data, width, height) = pixbuf_to_argb32(&pixbuf);
            vec![Icon { width, height, data: argb_data }]
        }
        Err(e) => {
            warn!("加载托盘图标失败: {}", e);
            vec![]
        }
    }
}

pub fn load_default_cover_pixbuf() -> Option<Pixbuf> {
    const APP_ICON: &[u8] = include_bytes!("../../data/icons/hicolor/512x512@2x.png");
    match Pixbuf::from_read(Cursor::new(APP_ICON)) {
        Ok(pixbuf) => pixbuf.scale_simple(140, 140, InterpType::Bilinear),
        Err(e) => {
            warn!("加载默认封面图标失败: {}", e);
            None
        }
    }
}

fn load_cover_icon_data(album_id: u64) -> Vec<u8> {
    let mut path = crate::path::CACHE.clone();
    path.push(format!("{}-songlist.jpg", album_id));

    let pixbuf = if album_id != 0 && path.exists() {
        Pixbuf::from_file_at_scale(&path, 140, 140, true).ok()
    } else {
        None
    }
    .or_else(load_default_cover_pixbuf);

    pixbuf
        .as_ref()
        .and_then(|pixbuf| pixbuf.save_to_bufferv("png", &[]).ok())
        .unwrap_or_default()
}

fn truncate_text(text: &str, max_chars: usize) -> String {
    let truncated: String = text.chars().take(max_chars).collect();
    if text.chars().count() > max_chars {
        format!("{}...", truncated)
    } else {
        truncated
    }
}

#[derive(Clone)]
pub struct TrayState {
    playing: Arc<Mutex<bool>>,
    song_title: Arc<Mutex<String>>,
    song_artist: Arc<Mutex<String>>,
    cover_icon_data: Arc<Mutex<Vec<u8>>>,
}

impl TrayState {
    pub fn new() -> Self {
        Self {
            playing: Arc::new(Mutex::new(false)),
            song_title: Arc::new(Mutex::new(gettext("NetEase Cloud Music"))),
            song_artist: Arc::new(Mutex::new(String::new())),
            cover_icon_data: Arc::new(Mutex::new(load_cover_icon_data(0))),
        }
    }

    pub fn set_playing(&self, playing: bool) {
        if let Ok(mut p) = self.playing.lock() {
            *p = playing;
        }
    }

    pub fn set_song_title(&self, title: String) {
        if let Ok(mut t) = self.song_title.lock() {
            *t = title;
        }
    }

    pub fn set_song_artist(&self, artist: String) {
        if let Ok(mut a) = self.song_artist.lock() {
            *a = artist;
        }
    }

    pub fn set_cover_icon_data(&self, cover_icon_data: Vec<u8>) {
        if let Ok(mut icon) = self.cover_icon_data.lock() {
            *icon = cover_icon_data;
        }
    }

    #[allow(dead_code)]
    pub fn get_playing(&self) -> bool {
        self.playing.lock().ok().map(|p| *p).unwrap_or(false)
    }

    pub fn get_song_title(&self) -> String {
        self.song_title.lock().ok().map(|t| t.clone()).unwrap_or_else(|| gettext("NetEase Cloud Music"))
    }

    pub fn get_song_artist(&self) -> String {
        self.song_artist.lock().ok().map(|a| a.clone()).unwrap_or_default()
    }

    pub fn get_cover_icon_data(&self) -> Vec<u8> {
        self.cover_icon_data
            .lock()
            .ok()
            .map(|data| data.clone())
            .unwrap_or_default()
    }
}

pub struct SystemTray {
    sender: Sender<Action>,
    state: TrayState,
}

impl SystemTray {
    pub fn new(sender: Sender<Action>, state: TrayState) -> Self {
        Self { sender, state }
    }
}

impl Tray for SystemTray {
    fn id(&self) -> String {
        env!("CARGO_PKG_NAME").to_string()
    }

    fn title(&self) -> String {
        let title = truncate_text(&self.state.get_song_title(), 9);
        let artist = truncate_text(&self.state.get_song_artist(), 9);
        if artist.is_empty() {
            title
        } else {
            format!("{} - {}", title, artist)
        }
    }

    fn icon_name(&self) -> String {
        crate::APP_ID.to_string()
    }

    fn icon_pixmap(&self) -> Vec<Icon> {
        load_app_icon()
    }

    fn menu(&self) -> Vec<MenuItem<Self>> {
        let title = truncate_text(&self.state.get_song_title(), 9);
        let artist = truncate_text(&self.state.get_song_artist(), 9);
        let info_label = if artist.is_empty() {
            title
        } else {
            format!("{} - {}", title, artist)
        };
        let playing = self.state.get_playing();
        let play_label = if playing {
            gettext("Pause")
        } else {
            gettext("Play")
        };
        let play_icon = if playing {
            "media-playback-pause-symbolic"
        } else {
            "media-playback-start-symbolic"
        };

        vec![
            StandardItem {
                label: info_label,
                icon_data: self.state.get_cover_icon_data(),
                activate: Box::new(|tray: &mut Self| {
                    let _ = tray.sender.try_send(Action::ShowMainWindow);
                }),
                ..Default::default()
            }
            .into(),
            MenuItem::Separator,
            StandardItem {
                label: gettext("Previous"),
                icon_name: "media-skip-backward-symbolic".to_string(),
                activate: Box::new(|tray: &mut Self| {
                    let _ = tray.sender.try_send(Action::PlayPreviousSong);
                }),
                ..Default::default()
            }
            .into(),
            StandardItem {
                label: play_label,
                icon_name: play_icon.to_string(),
                activate: Box::new(|tray: &mut Self| {
                    let _ = tray.sender.try_send(Action::TogglePlayPause);
                }),
                ..Default::default()
            }
            .into(),
            StandardItem {
                label: gettext("Next"),
                icon_name: "media-skip-forward-symbolic".to_string(),
                activate: Box::new(|tray: &mut Self| {
                    let _ = tray.sender.try_send(Action::PlayNextSong);
                }),
                ..Default::default()
            }
            .into(),
            MenuItem::Separator,
            StandardItem {
                label: gettext("Quit"),
                activate: Box::new(|tray: &mut Self| {
                    let _ = tray.sender.try_send(Action::Quit);
                }),
                ..Default::default()
            }
            .into(),
        ]
    }

    fn activate(&mut self, _x: i32, _y: i32) {
        let _ = self.sender.try_send(Action::ShowMainWindow);
    }

    fn secondary_activate(&mut self, _x: i32, _y: i32) {
        let _ = self.sender.try_send(Action::ShowMainWindow);
    }
}

pub struct TrayHandle {
    state: Option<TrayState>,
    service: Option<ksni::Handle<SystemTray>>,
}

impl TrayHandle {
    pub fn new() -> Self {
        Self {
            state: None,
            service: None,
        }
    }

    pub fn start(&mut self, sender: Sender<Action>) {
        if self.state.is_some() {
            info!("系统托盘已在运行");
            return;
        }

        info!("启动系统托盘");
        let state = TrayState::new();
        let tray = SystemTray::new(sender, state.clone());
        let service = TrayService::new(tray);
        let handle = service.handle();
        service.spawn();

        self.state = Some(state);
        self.service = Some(handle);
        info!("系统托盘启动成功");
    }

    pub fn stop(&mut self) {
        if let Some(handle) = self.service.take() {
            info!("停止系统托盘");
            drop(handle);
        }
        self.state.take();
        info!("系统托盘已停止");
    }

    pub fn update_playing(&self, playing: bool) {
        if let Some(state) = &self.state {
            state.set_playing(playing);
            self.refresh();
        }
    }

    pub fn update_song_title(&self, title: String, artist: String, album_id: u64) {
        if let Some(state) = &self.state {
            state.set_song_title(title);
            state.set_song_artist(artist);
            state.set_cover_icon_data(load_cover_icon_data(album_id));
            self.refresh();
        }
    }

    fn refresh(&self) {
        if let Some(service) = &self.service {
            service.update(|_| {});
        }
    }

    pub fn is_running(&self) -> bool {
        self.state.is_some()
    }
}

impl Default for TrayHandle {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for TrayHandle {
    fn drop(&mut self) {
        self.stop();
    }
}
