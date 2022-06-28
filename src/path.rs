//
// path.rs
// Copyright (C) 2022 gmg137 <gmg137 AT live.com>
// Distributed under terms of the GPL-3.0-or-later license.
//

use std::fs;
use std::path::PathBuf;

use gtk::glib;
use once_cell::sync::Lazy;

use crate::config;

pub static DATA: Lazy<PathBuf> = Lazy::new(|| {
    let mut path = glib::user_data_dir();
    path.push(config::GETTEXT_PACKAGE);
    path
});

pub static CONFIG: Lazy<PathBuf> = Lazy::new(|| {
    let mut path = glib::user_config_dir();
    path.push(config::GETTEXT_PACKAGE);
    path
});

pub static CACHE: Lazy<PathBuf> = Lazy::new(|| {
    let mut path = glib::user_cache_dir();
    path.push(config::GETTEXT_PACKAGE);
    path
});

pub fn init() -> std::io::Result<()> {
    fs::create_dir_all(DATA.to_owned())?;
    fs::create_dir_all(CONFIG.to_owned())?;
    fs::create_dir_all(CACHE.to_owned())?;
    Ok(())
}
