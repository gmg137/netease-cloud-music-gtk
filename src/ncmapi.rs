//
// ncmapi.rs
// Copyright (C) 2022 gmg137 <gmg137 AT live.com>
// Distributed under terms of the GPL-3.0-or-later license.
//
use anyhow::Result;
use cookie_store::CookieStore;
use ncm_api::{CookieBuilder, CookieJar, MusicApi, SongUrl};

use crate::path::CACHE;
use log::error;
use std::{fs, io, path::PathBuf};

const COOKIE_FILE: &str = "cookies.json";
const MAX_CONS: usize = 32;

pub const BASE_URL_LIST: [&str; 12] = [
    "https://music.163.com/",
    "https://music.163.com/eapi/clientlog",
    "https://music.163.com/eapi/feedback",
    "https://music.163.com/api/clientlog",
    "https://music.163.com/api/feedback",
    "https://music.163.com/neapi/clientlog",
    "https://music.163.com/neapi/feedback",
    "https://music.163.com/weapi/clientlog",
    "https://music.163.com/weapi/feedback",
    "https://music.163.com/wapi/clientlog",
    "https://music.163.com/wapi/feedback",
    "https://music.163.com/openapi/clientlog",
];

#[derive(Clone)]
pub struct NcmClient {
    pub client: MusicApi,
}

impl NcmClient {
    pub fn new() -> Self {
        Self {
            client: MusicApi::new(MAX_CONS),
        }
    }

    pub fn from_cookie_jar(cookie_jar: CookieJar) -> Self {
        Self {
            client: MusicApi::from_cookie_jar(cookie_jar, MAX_CONS),
        }
    }

    pub fn set_proxy(&mut self, proxy: String) -> Result<()> {
        self.client.set_proxy(&proxy)
    }

    pub fn get_api_rate(item: u32) -> u32 {
        match item {
            0 => 128000,
            1 => 192000,
            2 => 320000,
            3 => 999000,
            4 => 1900000,
            _ => 320000,
        }
    }

    /*
    pub fn set_cookie_jar_to_global(&self) {
        if let Some(cookie_jar) = self.client.cookie_jar() {
            match COOKIE_JAR.get() {
                Some(global_jar) => {
                    for base_url in BASE_URL_LIST {
                        let url = base_url.parse().unwrap();
                        cookie_jar.get_for_uri(&url).into_iter().for_each(|c| {
                            global_jar.set(c, &url).unwrap();
                        });
                    }
                }
                None => {
                    COOKIE_JAR.set(cookie_jar.to_owned()).unwrap();
                }
            }
        }
    }
    */

    pub fn cookie_file_path() -> PathBuf {
        crate::path::DATA.clone().join(COOKIE_FILE)
    }

    pub fn load_cookie_jar_from_file() -> Option<CookieJar> {
        match fs::File::open(&Self::cookie_file_path()) {
            Err(err) => match err.kind() {
                io::ErrorKind::NotFound => (),
                other => error!("{:?}", other),
            },
            Ok(file) => match CookieStore::load_json(io::BufReader::new(file)) {
                Err(err) => error!("{:?}", err),
                Ok(cookie_store) => {
                    let cookie_jar = CookieJar::default();
                    for base_url in BASE_URL_LIST {
                        let url = base_url.parse().unwrap();
                        for c in cookie_store.matches(&url) {
                            let cookie = CookieBuilder::new(c.name(), c.value())
                                .domain("music.163.com")
                                .path(c.path().unwrap_or("/"))
                                .build()
                                .unwrap();
                            cookie_jar.set(cookie, &base_url.parse().unwrap()).unwrap();
                        }
                    }
                    return Some(cookie_jar);
                }
            },
        };
        None
    }

    pub fn save_cookie_jar_to_file(&self) {
        if let Some(cookie_jar) = self.client.cookie_jar() {
            match fs::File::create(&Self::cookie_file_path()) {
                Err(err) => error!("{:?}", err),
                Ok(mut file) => {
                    let mut cookie_store = CookieStore::default();
                    for base_url in BASE_URL_LIST {
                        let uri = &base_url.parse().unwrap();
                        let url = &base_url.parse().unwrap();
                        for c in cookie_jar.get_for_uri(url) {
                            let cookie = cookie_store::Cookie::parse(
                                format!(
                                    "{}={}; Path={}; Domain=music.163.com; Max-Age=31536000",
                                    c.name(),
                                    c.value(),
                                    url.path()
                                ),
                                uri,
                            )
                            .unwrap();
                            cookie_store.insert(cookie, uri).unwrap();
                        }
                    }
                    cookie_store.save_json(&mut file).unwrap();
                }
            }
        }
    }

    pub fn clean_cookie_file() {
        if let Err(err) = fs::remove_file(&crate::path::DATA.clone().join(COOKIE_FILE)) {
            match err.kind() {
                io::ErrorKind::NotFound => (),
                other => error!("{:?}", other),
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

    pub async fn songs_url(&self, ids: &[u64], rate: u32) -> Result<Vec<SongUrl>> {
        self.client
            .songs_url(ids, &Self::get_api_rate(rate).to_string())
            .await
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

impl Default for NcmClient {
    fn default() -> Self {
        Self::new()
    }
}
