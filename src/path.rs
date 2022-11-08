//
// path.rs
// Copyright (C) 2022 gmg137 <gmg137 AT live.com>
// Distributed under terms of the GPL-3.0-or-later license.
//

use std::path::PathBuf;
use std::{fs, io};

use gtk::glib;
use log::*;
use once_cell::sync::{Lazy, OnceCell};

use crate::config;

static CACHE_SIZE: OnceCell<u64> = OnceCell::new();

pub static DATA: Lazy<PathBuf> = Lazy::new(|| {
    let mut path = glib::user_data_dir();
    path.push(config::GETTEXT_PACKAGE);
    debug!("初始化数据目录: {:?}", path);
    path
});

pub static CONFIG: Lazy<PathBuf> = Lazy::new(|| {
    let mut path = glib::user_config_dir();
    path.push(config::GETTEXT_PACKAGE);
    debug!("初始化配置目录: {:?}", path);
    path
});

pub static CACHE: Lazy<PathBuf> = Lazy::new(|| {
    let mut path = glib::user_cache_dir();
    path.push(config::GETTEXT_PACKAGE);
    debug!("初始化缓存目录: {:?}", path);
    path
});

pub fn init() -> std::io::Result<()> {
    fs::create_dir_all(DATA.to_owned())?;
    fs::create_dir_all(CONFIG.to_owned())?;
    fs::create_dir_all(CACHE.to_owned())?;
    Ok(())
}

pub fn get_music_cache_path(id: u64, rate: u32) -> PathBuf {
    let rate = match rate {
        0 => 128000,
        1 => 192000,
        2 => 320000,
        3 => 999000,
        4 => 1900000,
        _ => 320000,
    };
    CACHE.join(format!("music_{}_{}", id, rate))
}

pub fn get_cache_size() -> (f64, String) {
    let size: u64 = {
        match CACHE_SIZE.get() {
            Some(s) => s.to_owned(),
            None => {
                let s = get_dir_size(CACHE.clone());
                CACHE_SIZE.set(s).unwrap();
                s
            }
        }
    };
    dir_size_with_unit(size)
}

pub fn get_dir_size(path: impl Into<PathBuf>) -> u64 {
    fn dir_size(dir: io::Result<fs::ReadDir>, depth: i32) -> u64 {
        match depth {
            0 => 0,
            _ => match dir {
                Ok(dir) => dir
                    .filter_map(|entry| entry.ok())
                    .filter_map(|entry| entry.metadata().ok().map(|m| (entry, m)))
                    .fold(0, |size, (entry, metadata)| {
                        if metadata.is_dir() {
                            size + dir_size(fs::read_dir(entry.path()), depth - 1)
                        } else {
                            size + metadata.len()
                        }
                    }),
                Err(_) => 0,
            },
        }
    }
    dir_size(fs::read_dir(path.into()), 3)
}

pub fn dir_size_with_unit(dir_size: u64) -> (f64, String) {
    const BYTE_UNITS: &[&str] = &["B", "kB", "MB", "GB", "TB", "PB", "EB", "ZB", "YB"];
    let dir_size = dir_size as f64 + 1.0;
    let exponent = std::cmp::min(dir_size.log(1024.0).floor() as usize, BYTE_UNITS.len() - 1);
    (
        dir_size / 1024usize.pow(exponent as u32) as f64,
        BYTE_UNITS[exponent].to_string(),
    )
}
