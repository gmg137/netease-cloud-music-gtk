//
// model.rs
// Copyright (C) 2020 gmg137 <gmg137@live.com>
// Distributed under terms of the MIT license.
//
use async_std::io;
use chrono::prelude::*;
use custom_error::custom_error;
use lazy_static::lazy_static;
use std::{collections::HashMap, path::PathBuf};

pub(crate) type NCMResult<T> = Result<T, Errors>;

lazy_static! {
    pub(crate) static ref NCM_XDG: xdg::BaseDirectories = {
        xdg::BaseDirectories::with_prefix("netease-cloud-music-gtk").unwrap()
    };

    // 数据目录
    pub(crate) static ref NCM_DATA: PathBuf = {
        NCM_XDG.create_data_directory(NCM_XDG.get_data_home()).unwrap()
    };

    // 配置目录
    pub(crate) static ref NCM_CONFIG: PathBuf = {
        NCM_XDG.create_config_directory(NCM_XDG.get_config_home()).unwrap()
    };

    // 缓存目录
    pub(crate) static ref NCM_CACHE: PathBuf = {
        NCM_XDG.create_cache_directory(NCM_XDG.get_cache_home()).unwrap()
    };

    // 歌词文件目录
    pub(crate) static ref LYRICS_PATH: &'static str = {
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
    pub(crate) static ref DATE_DAY: u32 = {
        let date = Local::now();
        date.day()
    };

    // 当前时期-周
    pub(crate) static ref ISO_WEEK: u32 = {
        let date = Local::now();
        date.iso_week().week()
    };

    // 当前时期-月
    pub(crate) static ref DATE_MONTH: u32 = {
        let date = Local::now();
        date.month()
    };

    // 排行榜 id
    pub(crate) static ref TOP_ID: HashMap<u8,u64>= {
        let mut m = HashMap::new();
        m.insert(0, 19_723_756);
        m.insert(1, 3_779_629);
        m.insert(2, 2_884_035);
        m.insert(3, 3_778_678);
        m.insert(4, 71_384_707);
        m.insert(5, 71_385_702);
        m.insert(6, 745_956_260);
        m.insert(7, 10_520_166);
        m.insert(8, 991_319_590);
        m.insert(9, 2_250_011_882);
        m.insert(10, 180_106);
        m.insert(11, 60198);
        m.insert(12, 21_845_217);
        m.insert(13, 11_641_012);
        m.insert(14, 120_001);
        m.insert(15, 60131);
        m.insert(16, 112_463);
        m.insert(17, 4_395_559);
        m
    };

    // 排行榜名称
    pub(crate) static ref TOP_NAME: HashMap<u8,&'static str>= {
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

custom_error! { pub Errors
    OpenSSLError{ source: openssl::error::ErrorStack } = "openSSL Error",
    CurlError{ source: curl::Error } = "curl Error",
    RegexError{ source: regex::Error } = "regex Error",
    SerdeJsonError{ source: serde_json::error::Error } = "serde json Error",
    SerdeUrlEncodeError{ source: serde_urlencoded::ser::Error } = "serde url encode Error",
    ParseError{ source: std::num::ParseIntError } = "parse Error",
    AsyncIoError{ source: io::Error } = "async io Error",
    NoneError = "None Error",
}
