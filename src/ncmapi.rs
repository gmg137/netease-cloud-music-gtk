//
// ncmapi.rs
// Copyright (C) 2022 gmg137 <gmg137 AT live.com>
// Distributed under terms of the GPL-3.0-or-later license.
//
use anyhow::Result;
use ncm_api::{CookieJar, MusicApi, SongUrl};
use once_cell::sync::OnceCell;

use crate::path::CACHE;
use std::{fs, path::PathBuf};

pub static COOKIE_JAR: OnceCell<CookieJar> = OnceCell::new();
pub static UID: OnceCell<u64> = OnceCell::new();

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

    pub fn set_cookie_jar(&self) {
        if COOKIE_JAR.get().is_none() {
            if let Some(cookie_jar) = self.client.cookie_jar() {
                COOKIE_JAR.set(cookie_jar.to_owned()).unwrap();
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
