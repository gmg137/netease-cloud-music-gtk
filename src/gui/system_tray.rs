//
// system_tray.rs
// Copyright (C) 2026 gmg137 <gmg137 AT live.com>
// Distributed under terms of the GPL-3.0-or-later license.
//

use async_channel::Sender;
use gettextrs::gettext;
use ksni::{Icon, MenuItem, Tray, TrayService};
use log::*;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

use crate::application::Action;

// 嵌入应用图标（用于托盘显示）
const ICON_DATA: &[u8] = include_bytes!("../../data/icons/hicolor/512x512@2x.png");

// 加载图标像素数据
fn load_icon_pixmap() -> Result<Vec<Icon>, Box<dyn std::error::Error>> {
    // 使用 image crate 解码 PNG
    let img = image::load_from_memory(ICON_DATA)?;
    let rgba = img.to_rgba8();
    let (width, height) = rgba.dimensions();

    // 转换为 ARGB32 格式（StatusNotifierItem 协议要求）
    let mut argb_data = Vec::with_capacity((width * height * 4) as usize);
    for pixel in rgba.pixels() {
        let r = pixel[0];
        let g = pixel[1];
        let b = pixel[2];
        let a = pixel[3];

        // ARGB32 大端序
        argb_data.push(a);
        argb_data.push(r);
        argb_data.push(g);
        argb_data.push(b);
    }

    Ok(vec![Icon {
        width: width as i32,
        height: height as i32,
        data: argb_data,
    }])
}

// 共享的托盘状态
#[derive(Clone)]
pub struct TrayState {
    playing: Arc<Mutex<bool>>,
    song_title: Arc<Mutex<String>>,
}

impl TrayState {
    pub fn new() -> Self {
        Self {
            playing: Arc::new(Mutex::new(false)),
            song_title: Arc::new(Mutex::new(gettext("NetEase Cloud Music"))),
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

    pub fn get_playing(&self) -> bool {
        self.playing.lock().ok().map(|p| *p).unwrap_or(false)
    }

    pub fn get_song_title(&self) -> String {
        self.song_title.lock().ok().map(|t| t.clone()).unwrap_or_else(|| gettext("NetEase Cloud Music"))
    }
}

pub struct SystemTray {
    sender: Sender<Action>,
    state: TrayState,
}

impl SystemTray {
    pub fn new(sender: Sender<Action>, state: TrayState) -> Self {
        Self {
            sender,
            state,
        }
    }
}

impl Tray for SystemTray {
    fn id(&self) -> String {
        env!("CARGO_PKG_NAME").to_string()
    }

    fn title(&self) -> String {
        self.state.get_song_title()
    }

    fn icon_name(&self) -> String {
        "com.gitee.gmg137.NeteaseCloudMusicGtk4".to_string()
    }

    fn icon_pixmap(&self) -> Vec<Icon> {
        // 为 GNOME Status Tray 等插件提供图标像素数据
        // 某些桌面环境无法从图标主题加载图标，需要直接提供像素数据
        match load_icon_pixmap() {
            Ok(icons) => icons,
            Err(e) => {
                warn!("加载托盘图标失败: {}, 回退到图标名称", e);
                vec![]
            }
        }
    }

    fn menu(&self) -> Vec<MenuItem<Self>> {
        let playing = self.state.get_playing();

        vec![
            MenuItem::Standard(ksni::menu::StandardItem {
                label: gettext("Show Main Window"),
                enabled: true,
                visible: true,
                icon_name: "window-restore-symbolic".to_string(),
                icon_data: vec![],
                shortcut: vec![],
                disposition: ksni::menu::Disposition::Normal,
                activate: Box::new(|this: &mut Self| {
                    let _ = this.sender.try_send(Action::ShowMainWindow);
                }),
            }),
            MenuItem::Separator,
            MenuItem::Standard(ksni::menu::StandardItem {
                label: if playing {
                    gettext("Pause")
                } else {
                    gettext("Play")
                },
                enabled: true,
                visible: true,
                icon_name: if playing {
                    "media-playback-pause-symbolic".to_string()
                } else {
                    "media-playback-start-symbolic".to_string()
                },
                icon_data: vec![],
                shortcut: vec![],
                disposition: ksni::menu::Disposition::Normal,
                activate: Box::new(|this: &mut Self| {
                    let current = this.state.get_playing();
                    this.state.set_playing(!current);
                    let _ = this.sender.try_send(Action::TogglePlayPause);
                }),
            }),
            MenuItem::Standard(ksni::menu::StandardItem {
                label: gettext("Previous"),
                enabled: true,
                visible: true,
                icon_name: "media-skip-backward-symbolic".to_string(),
                icon_data: vec![],
                shortcut: vec![],
                disposition: ksni::menu::Disposition::Normal,
                activate: Box::new(|this: &mut Self| {
                    let _ = this.sender.try_send(Action::PlayPreviousSong);
                }),
            }),
            MenuItem::Standard(ksni::menu::StandardItem {
                label: gettext("Next"),
                enabled: true,
                visible: true,
                icon_name: "media-skip-forward-symbolic".to_string(),
                icon_data: vec![],
                shortcut: vec![],
                disposition: ksni::menu::Disposition::Normal,
                activate: Box::new(|this: &mut Self| {
                    let _ = this.sender.try_send(Action::PlayNextSong);
                }),
            }),
            MenuItem::Separator,
            MenuItem::Standard(ksni::menu::StandardItem {
                label: gettext("Quit"),
                enabled: true,
                visible: true,
                icon_name: "application-exit-symbolic".to_string(),
                icon_data: vec![],
                shortcut: vec![],
                disposition: ksni::menu::Disposition::Normal,
                activate: Box::new(|_this: &mut Self| {
                    // 通过 Action 通道发送退出信号，让应用优雅退出
                    // 这样可以确保 GStreamer 和其他资源被正确清理
                    use gtk::prelude::*;
                    if let Some(app) = gtk::gio::Application::default() {
                        app.quit();
                    } else {
                        // 如果无法获取应用实例，尝试通过 sender 发送
                        // 注意：这里不能直接退出，因为会导致 SEGV
                        warn!("Cannot get application instance for quit");
                    }
                }),
            }),
        ]
    }

    fn activate(&mut self, _x: i32, _y: i32) {
        // 左键点击托盘图标时显示/隐藏主窗口
        let _ = self.sender.try_send(Action::ToggleMainWindow);
    }

    fn secondary_activate(&mut self, _x: i32, _y: i32) {
        // 右键点击也显示/隐藏主窗口（菜单会自动显示）
        let _ = self.sender.try_send(Action::ShowMainWindow);
    }
}

pub struct TrayHandle {
    state: Option<TrayState>,
    service: Rc<RefCell<Option<ksni::Handle<SystemTray>>>>,
}

impl TrayHandle {
    pub fn new() -> Self {
        Self {
            state: None,
            service: Rc::new(RefCell::new(None)),
        }
    }

    pub fn start(&mut self, sender: Sender<Action>) -> Result<(), Box<dyn std::error::Error>> {
        info!("尝试启动系统托盘");

        // 如果已有实例，先清理
        if self.state.is_some() {
            warn!("系统托盘已在运行，先停止旧实例");
            self.stop();

            // 等待旧实例完全清理（给 DBus 操作时间完成）
            std::thread::sleep(std::time::Duration::from_millis(150));
            info!("旧实例清理完成，继续启动新实例");
        }

        let state = TrayState::new();
        let tray = SystemTray::new(sender, state.clone());
        let service = TrayService::new(tray);
        let handle = service.handle();
        service.spawn();

        info!("系统托盘启动成功");
        self.state = Some(state);
        *self.service.borrow_mut() = Some(handle);
        Ok(())
    }

    pub fn stop(&mut self) {
        info!("开始停止系统托盘");

        // 先获取服务句柄
        if let Some(handle) = self.service.borrow_mut().take() {
            debug!("正在清理 DBus 连接和托盘服务");

            // 强制隐藏托盘图标，发送 DBus 移除信号
            // 这对于 Linux DBus/SNI 系统很重要，可以确保图标立即从系统托盘中移除
            let _ = handle.update(|_tray| {
                // 触发一次更新，确保 DBus 状态同步
                debug!("触发托盘更新以同步 DBus 状态");
            });

            // 显式 drop 服务，清理 DBus 注册
            drop(handle);
            info!("系统托盘服务已停止，DBus 已清理");

            // 给 DBus 操作时间完成，避免资源泄漏
            std::thread::sleep(std::time::Duration::from_millis(100));
            debug!("DBus 清理延迟完成");
        } else {
            debug!("没有活动的托盘服务需要停止");
        }

        if let Some(_state) = self.state.take() {
            info!("系统托盘状态已清除");
        }

        info!("系统托盘完全停止");
    }

    pub fn update_playing(&self, playing: bool) {
        if let Some(state) = &self.state {
            state.set_playing(playing);
            // 触发菜单更新
            if let Some(service) = self.service.borrow().as_ref() {
                service.update(|_tray| {
                    // 状态已经在 state 中更新，这里只需要触发刷新
                });
            }
        }
    }

    pub fn update_song_title(&self, title: String) {
        if let Some(state) = &self.state {
            state.set_song_title(title);
            // 触发标题更新
            if let Some(service) = self.service.borrow().as_ref() {
                service.update(|_tray| {
                    // 标题已经在 state 中更新，这里只需要触发刷新
                });
            }
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
        // 确保在 TrayHandle 被 drop 时清理资源
        info!("TrayHandle 正在被 drop，执行清理");
        self.stop();
        debug!("TrayHandle drop 完成");
    }
}
