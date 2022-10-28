//
// ncmapi.rs
// Copyright (C) 2022 gmg137 <gmg137 AT live.com>
// Distributed under terms of the GPL-3.0-or-later license.
//
use anyhow::Result;
use cookie_store::CookieStore;
use isahc::cookies;
use ncm_api::{CookieJar, MusicApi, SongUrl};
use once_cell::sync::OnceCell;

use crate::path::CACHE;
use log::error;
use std::{fs, io, path::PathBuf};

const COOKIE_FILE: &str = "cookies.json";
static COOKIE_JAR: OnceCell<CookieJar> = OnceCell::new();

pub const BASE_URL: &str = "https://music.163.com";

pub struct NcmClient {
    pub client: MusicApi,
    rate: u32,
}

impl NcmClient {
    pub fn new() -> Self {
        if let Some(cookie_jar) = COOKIE_JAR.get() {
            let client = MusicApi::from_cookie_jar(cookie_jar.to_owned());
            return Self {
                client,
                rate: 320000,
            };
        }
        Self {
            client: MusicApi::new(),
            rate: 320000,
        }
    }

    pub fn set_proxy(&mut self, proxy: String) -> Result<()> {
        self.client.set_proxy(&proxy)
    }

    pub fn set_rate(&mut self, item: u32) {
        let rate = match item {
            0 => 128000,
            1 => 192000,
            2 => 320000,
            3 => 999000,
            4 => 1900000,
            _ => 320000,
        };
        self.rate = rate;
    }

    pub fn set_cookie_jar_to_global(&self) {
        if let Some(cookie_jar) = self.client.cookie_jar() {
            match COOKIE_JAR.get() {
                Some(global_jar) => {
                    let url = BASE_URL.parse().unwrap();
                    cookie_jar.get_for_uri(&url).into_iter().for_each(|c| {
                        global_jar.set(c, &url).unwrap();
                    });
                }
                None => {
                    COOKIE_JAR.set(cookie_jar.to_owned()).unwrap();
                }
            }
        }
    }

    pub fn cookie_file_path() -> PathBuf {
        crate::path::DATA.clone().join(COOKIE_FILE)
    }

    pub fn try_load_cookie_jar_from_file() -> bool {
        if COOKIE_JAR.get().is_none() {
            match fs::File::open(&Self::cookie_file_path()) {
                Err(err) => match err.kind() {
                    io::ErrorKind::NotFound => (),
                    other => error!("{:?}", other),
                },
                Ok(file) => match CookieStore::load_json(io::BufReader::new(file)) {
                    Err(err) => error!("{:?}", err),
                    Ok(cookie_store) => {
                        let cookie_jar = CookieJar::default();
                        let url = BASE_URL.parse().unwrap();

                        for c in cookie_store.matches(&url) {
                            let cookie = cookies::CookieBuilder::new(c.name(), c.value())
                                .build()
                                .unwrap();
                            cookie_jar.set(cookie, &BASE_URL.parse().unwrap()).unwrap();
                        }
                        COOKIE_JAR.set(cookie_jar).unwrap();
                        return true;
                    }
                },
            };
        }
        return false;
    }

    pub fn save_cookie_jar_to_file() {
        if let Some(cookie_jar) = COOKIE_JAR.get() {
            match fs::File::create(&Self::cookie_file_path()) {
                Err(err) => error!("{:?}", err),
                Ok(mut file) => {
                    let mut cookie_store = CookieStore::default();
                    let url = &BASE_URL.parse().unwrap();
                    for c in cookie_jar.get_for_uri(&BASE_URL.parse().unwrap()) {
                        let cookie = cookie_store::Cookie::parse(
                            format!("{}={}; Max-Age=31536000", c.name(), c.value()),
                            &url,
                        )
                        .unwrap();
                        cookie_store.insert(cookie, &url).unwrap();
                    }
                    cookie_store.save_json(&mut file).unwrap();
                }
            }
        }
    }

    pub fn clean_cookie_jar_and_file() {
        if let Some(cookie_jar) = COOKIE_JAR.get() {
            cookie_jar.clear();
            match fs::remove_file(&crate::path::DATA.clone().join(COOKIE_FILE)) {
                Err(err) => match err.kind() {
                    io::ErrorKind::NotFound => (),
                    other => error!("{:?}", other),
                },
                Ok(_) => (),
            }
        }
    }

    pub async fn create_qrcode(&self) -> Result<(PathBuf, String)> {
        let qrinfo = self.client.login_qr_create().await?;
        let mut path = CACHE.clone();
        path.push("qrimage.png");
        qrcode_generator::to_png_to_file(qrinfo.0, qrcode_generator::QrCodeEcc::Low, 140, &path)?;
        Ok((path, qrinfo.1))
    }

    pub async fn songs_url(&self, ids: &[u64]) -> Result<Vec<SongUrl>> {
        self.client.songs_url(ids, &self.rate.to_string()).await
    }

    pub async fn get_lyrics(&self, id: u64) -> Result<String> {
        let mut path = CACHE.clone();
        path.push(format!("{}.lrc", id));
        if !path.exists() {
            if let Ok(lyr) = self.client.song_lyric(id).await {
                let re = regex::Regex::new(r"\[\d+:\d+.\d+\]").unwrap();
                let lrc = lyr
                    .into_iter()
                    .map(|v| re.replace_all(&v, "").to_string())
                    .collect::<Vec<String>>()
                    .join("\n");
                fs::write(&path, &lrc)?;
                Ok(lrc)
            } else {
                Ok(gettextrs::gettext("No lyrics found!".to_owned()))
            }
        } else {
            let lrc = fs::read_to_string(&path)?;
            Ok(lrc)
        }
    }
}
