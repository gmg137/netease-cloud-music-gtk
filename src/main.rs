extern crate gstreamer as gst;
extern crate gstreamer_player as gst_player;

use lazy_static::lazy_static;
#[macro_use]
extern crate log;

mod app;
mod data;
mod musicapi;
mod utils;
mod view;
mod widgets;
use crate::app::App;
use std::collections::HashMap;
static APP_VERSION: &str = "0.6.0";

lazy_static! {
    // 配置文件目录
    static ref CONFIG_PATH: &'static str = {
        if let Some(path) = dirs::home_dir() {
            let config_path = format!("{}/.config/NeteaseCloudMusicGtk", path.display());
            if !std::path::Path::new(&config_path).exists() {
                std::fs::create_dir(&config_path).unwrap_or(());
            }
            return Box::leak(Box::new(config_path));
        }
        ".config/NeteaseCloudMusicGtk"
    };
    // 缓存文件目录
    static ref CACHED_PATH: &'static str = {
        if let Some(path) = dirs::home_dir() {
            let cached_path = format!("{}/.cache/NeteaseCloudMusicGtk", path.display());
            if !std::path::Path::new(&cached_path).exists() {
                std::fs::create_dir_all(&cached_path).unwrap_or(());
            }
            return Box::leak(Box::new(cached_path));
        }
        ".cache/NeteaseCloudMusicGtk"
    };
    // 歌词文件目录
    static ref LYRICS_PATH: &'static str = {
        if let Some(path) = dirs::home_dir() {
            let lyrics_path = format!("{}/.lyrics", path.display());
            if !std::path::Path::new(&lyrics_path).exists() {
                std::fs::create_dir_all(&lyrics_path).unwrap_or(());
            }
            return Box::leak(Box::new(lyrics_path));
        }
        ".lyrics"
    };
    // 当前时期-天
    static ref DATE_DAY: u32 = {
        use chrono::prelude::*;
        let date = Local::now();
        date.day()
    };
    // 当前时期-周
    static ref ISO_WEEK: u32 = {
        use chrono::prelude::*;
        let date = Local::now();
        date.iso_week().week()
    };
    // 排行榜 id
    static ref TOP_ID: HashMap<u8,u32>= {
        let mut m = HashMap::new();
        m.insert(0, 19723756);
        m.insert(1, 3779629);
        m.insert(2, 2884035);
        m.insert(3, 3778678);
        m.insert(4, 71384707);
        m.insert(5, 71385702);
        m.insert(6, 745956260);
        m.insert(7, 10520166);
        m.insert(8, 991319590);
        m.insert(9, 2250011882);
        m.insert(10, 180106);
        m.insert(11, 60198);
        m.insert(12, 21845217);
        m.insert(13, 11641012);
        m.insert(14, 120001);
        m.insert(15, 60131);
        m.insert(16, 112463);
        m.insert(17, 4395559);
        m
    };
    // 排行榜名称
    static ref TOP_NAME: HashMap<u8,&'static str>= {
        let mut m = HashMap::new();
        m.insert(0," 云音乐飙升榜");
        m.insert(1," 云音乐新歌榜");
        m.insert(2," 网易原创歌曲榜");
        m.insert(3," 云音乐热歌榜");
        m.insert(4," 云音乐古典音乐榜");
        m.insert(5," 云音乐ACG音乐榜");
        m.insert(6," 云音乐韩语榜");
        m.insert(7," 云音乐国电榜");
        m.insert(8," 云音乐嘻哈榜");
        m.insert(9," 抖音排行榜");
        m.insert(10,"UK排行榜周榜");
        m.insert(11,"美国Billboard周榜");
        m.insert(12,"KTV嗨榜");
        m.insert(13,"iTunes榜");
        m.insert(14,"Hit FM Top榜");
        m.insert(15,"日本Oricon周榜");
        m.insert(16,"台湾Hito排行榜");
        m.insert(17,"华语金曲榜");
        m
    };
}

#[macro_export]
macro_rules! clone {
    (@param _) => ( _ );
    (@param $x:ident) => ( $x );
    ($($n:ident),+ => move || $body:expr) => (
        {
            $( let $n = $n.clone(); )+
            move || $body
        }
    );
    ($($n:ident),+ => move |$($p:tt),+| $body:expr) => (
        {
            $( let $n = $n.clone(); )+
            move |$(clone!(@param $p),)+| $body
        }
    );
}

#[macro_export]
macro_rules! upgrade_weak {
    ($x:ident, $r:expr) => {{
        match $x.upgrade() {
            Some(o) => o,
            None => return $r,
        }
    }};
    ($x:ident) => {
        upgrade_weak!($x, ())
    };
}

fn main() {
    loggerv::init_with_level(log::Level::Info).expect("Error initializing loggerv.");

    gtk::init().expect("Error initializing gtk.");
    gst::init().expect("Error initializing gstreamer");

    App::run();
}
