//
// ncmapi.rs
// Copyright (C) 2022 gmg137 <gmg137 AT live.com>
// Distributed under terms of the GPL-3.0-or-later license.
//
use anyhow::Result;
use cookie_store::{CookieStore, serde};
use isahc::prelude::*;
use ncm_api::{
    CookieBuilder, CookieJar, MusicApi, SongCopyright, SongInfo, SongQuality, SongQualityState,
    SongUrl,
};

use crate::path::{CACHE, LYRICS};
use log::{debug, error};
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
            5 => 804505,
            6 => 2695684,
            7 => 4532511,
            _ => 320000,
        }
    }

    pub fn get_quality(item: u32) -> SongQuality {
        match item {
            0 => SongQuality::Standard,
            1 => SongQuality::Higher,
            2 => SongQuality::Extreme,
            3 => SongQuality::Lossless,
            4 => SongQuality::HiRes,
            5 => SongQuality::Surround,
            6 => SongQuality::AudioVivid,
            7 => SongQuality::Master,
            _ => SongQuality::Extreme,
        }
    }

    pub fn get_quality_index(quality: SongQuality) -> u32 {
        match quality {
            SongQuality::Standard => 0,
            SongQuality::Higher => 1,
            SongQuality::Extreme => 2,
            SongQuality::Lossless => 3,
            SongQuality::HiRes => 4,
            SongQuality::Surround => 5,
            SongQuality::AudioVivid => 6,
            SongQuality::Master => 7,
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
        match fs::File::open(Self::cookie_file_path()) {
            Err(err) => match err.kind() {
                io::ErrorKind::NotFound => (),
                other => error!("{:?}", other),
            },
            Ok(file) => match serde::json::load(io::BufReader::new(file)) {
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
            match fs::File::create(Self::cookie_file_path()) {
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
                    serde::json::save(&cookie_store, &mut file).unwrap();
                }
            }
        }
    }

    pub fn clean_cookie_file() {
        if let Err(err) = fs::remove_file(crate::path::DATA.clone().join(COOKIE_FILE)) {
            match err.kind() {
                io::ErrorKind::NotFound => (),
                other => error!("{:?}", other),
            }
        }
    }

    pub async fn create_qrcode(&self) -> Result<(PathBuf, String)> {
        let (qr_url, unikey) = self.client.login_qr_create().await?;
        let mut path = CACHE.clone();
        path.push("qrimage.png");
        qrcode_generator::to_png_to_file(qr_url, qrcode_generator::QrCodeEcc::Low, 140, &path)?;
        Ok((path, unikey))
    }

    pub async fn songs_url(&self, ids: &[u64], rate: u32) -> Result<Vec<SongUrl>> {
        self.client
            .songs_url_v1(ids, Self::get_quality(rate))
            .await
    }

    /// 歌曲搜索（供本地歌曲匹配用）。
    ///
    /// ncm-api 的 `search_song` 走 `/weapi/cloudsearch/get/web` 加密链路，在当前环境里
    /// 服务端一直返回非 200 的响应体，被上游解析成 `Err("none")`，导致本地补全全军覆没。
    /// 匹配只需要「歌名 + 歌手 + 专辑 + 时长」，不需要登录态，所以直接走公开的
    /// `/api/search/get/web`，绕开 weapi。
    ///
    /// 该接口有 IP 级频率限制（返回 code=405「操作频繁」），命中后必须长退避，
    /// 否则后面的歌会被一路连带判成「匹配不到」。
    async fn search_with_retry(&self, keywords: String) -> Option<Vec<ncm_api::SongInfo>> {
        use std::time::{Duration, Instant};
        for attempt in 0..3u32 {
            // 全局节流：相邻两次请求至少隔 SEARCH_INTERVAL，别撞频率限制
            let wait = Duration::from_millis(SEARCH_INTERVAL_MS)
                .saturating_sub(SEARCH_GATE.lock().unwrap().elapsed());
            if !wait.is_zero() {
                crate::glib::timeout_future(wait).await;
            }
            *SEARCH_GATE.lock().unwrap() = Instant::now();
            match search_web(&keywords, 10).await {
                WebSearch::Songs(list) if !list.is_empty() => return Some(list),
                // 接口正常但确实没这首歌：换下一组候选，别在同一组上空转
                WebSearch::Songs(_) => {
                    eprintln!("NCM_SEARCH_ERR [{}] 接口返回空结果", keywords);
                    return None;
                }
                WebSearch::RateLimited => {
                    // 5s / 10s / 15s 指数退避，限流是会累积的，急不来
                    let backoff = 5000 * (u64::from(attempt) + 1);
                    eprintln!(
                        "NCM_SEARCH_ERR [{}] 第{}次被限流，退避 {}ms",
                        keywords,
                        attempt + 1,
                        backoff
                    );
                    crate::glib::timeout_future(Duration::from_millis(backoff)).await;
                }
                WebSearch::Failed => {
                    eprintln!("NCM_SEARCH_ERR [{}] 第{}次请求失败", keywords, attempt + 1);
                    crate::glib::timeout_future(Duration::from_millis(
                        1000 * (u64::from(attempt) + 1),
                    ))
                    .await;
                }
            }
        }
        None
    }

    /// 用车载信息 + 文件名在网易云里匹配曲目，用于本地歌曲补全歌词与封面。
    ///
    /// 本地文件的标签经常把歌名/歌手写反，文件名又常是「歌手 - 歌名」，
    /// 所以把这些组合都当候选试一遍；评分也不区分名字/歌手朝向。
    /// 匹配不可信时返回 None（宁可不匹配，也不要贴错歌词）。
    pub async fn match_song(
        &self,
        name: &str,
        singer: &str,
        file_stem: Option<&str>,
    ) -> Option<SongInfo> {
        let mut pairs: Vec<(String, String)> = Vec::new();
        let unknown_singer = singer.trim().is_empty() || singer.contains("未知");
        // 联想接口对「歌名 + 歌手」这种长关键词经常直接返回空，纯歌名命中率更高，
        // 所以纯歌名候选排最前；能搜到之后再由下面的评分逻辑核对歌手。
        push_pair(&mut pairs, &simplify_title(name), "");
        if let Some(stem) = file_stem {
            push_pair(&mut pairs, &simplify_title(&strip_track_prefix(stem)), "");
        }
        push_pair(&mut pairs, name, if unknown_singer { "" } else { singer });
        // 标签写反：拿歌手当歌名再试一次
        if !unknown_singer {
            push_pair(&mut pairs, singer, name);
        }
        if let Some(stem) = file_stem {
            let stem = strip_track_prefix(stem);
            if let Some((left, right)) = stem.split_once(" - ") {
                push_pair(&mut pairs, right, left);
                push_pair(&mut pairs, left, right);
            }
            push_pair(&mut pairs, &stem, "");
        }
        // 只按歌名搜（歌手对不上、或歌手本身是垃圾标签时靠它兜底）
        push_pair(&mut pairs, &simplify_title(name), "");
        if let Some(stem) = file_stem {
            let stem = simplify_title(&strip_track_prefix(stem));
            push_pair(&mut pairs, &stem, "");
            // 文件名/歌名里没有空格的「歌手-歌名」写法
            if let Some((left, right)) = stem.split_once('-') {
                if !left.trim().is_empty() && !right.trim().is_empty() {
                    push_pair(&mut pairs, right, left);
                }
            }
        }

        let mut best: Option<(i32, SongInfo)> = None;
        // 最多试 4 组候选；拿到高置信度结果就提前收工，避免刷接口
        for (title, artist) in pairs.into_iter().take(4) {
            let keywords = format!("{} {}", title, artist).trim().to_string();
            // 搜索偶尔被限流（非 200 响应），串行 + 退避重试，别把整首歌判成“匹配不到”
            let Some(list) = self.search_with_retry(keywords).await else {
                continue;
            };
            let title_lower = title.to_lowercase();
            let artist_lower = artist.to_lowercase();
            for song in list {
                let song_name = song.name.to_lowercase();
                let song_singer = song.singer.to_lowercase();
                let hit = |wanted_title: &str, wanted_artist: &str| -> i32 {
                    let mut score = 0;
                    if song_name == wanted_title {
                        score += 6;
                    } else if !wanted_title.is_empty()
                        && (song_name.contains(wanted_title) || wanted_title.contains(&song_name))
                    {
                        score += 3;
                    }
                    if !wanted_artist.is_empty()
                        && (song_singer.contains(wanted_artist)
                            || wanted_artist.contains(&song_singer))
                    {
                        score += 4;
                    }
                    score
                };
                let mut score = hit(&title_lower, &artist_lower).max(hit(&artist_lower, &title_lower));
                if !song.pic_url.is_empty() {
                    score += 1;
                }
                if best.as_ref().map(|(s, _)| score > *s).unwrap_or(true) {
                    best = Some((score, song));
                }
            }
            if best.as_ref().map(|(s, _)| *s >= 9).unwrap_or(false) {
                break;
            }
        }

        let (score, mut song) = match best {
            Some(best) => best,
            None => {
                eprintln!("NCM_MATCH_MISS 无搜索结果");
                return None;
            }
        };
        // 至少歌名要对得上（3 分），避免给本地歌曲贴一个毫不相干的结果
        if score < 3 {
            eprintln!(
                "NCM_MATCH_MISS 分数不足({}) 最佳结果: {} - {}",
                score, song.name, song.singer
            );
            return None;
        }
        // 搜索接口常常不带封面图，用详情接口补齐（本地歌曲的封面靠它）
        if song.pic_url.is_empty() || song.album.is_empty() || song.album_id == 0 {
            if let Ok(list) = self.client.songs_detail(&[song.id]).await {
                if let Some(detail) = list.into_iter().next() {
                    if song.pic_url.is_empty() {
                        song.pic_url = detail.pic_url;
                    }
                    if song.album.is_empty() {
                        song.album = detail.album;
                    }
                    if song.album_id == 0 {
                        song.album_id = detail.album_id;
                    }
                }
            }
        }
        Some(song)
    }

    pub async fn get_lyrics(&self, si: SongInfo) -> Result<Vec<(u64, String)>> {
        // 歌词文件位置
        let mut lyric_path = LYRICS.clone();
        lyric_path.push(format!(
            "{}-{}-{}.lrc",
            si.name.replace('/', "／"),
            si.singer,
            si.album
        ));
        // 翻译歌词文件位置
        let mut tlyric_path = LYRICS.clone();
        tlyric_path.push(format!("{}.tlrc", si.id));
        // 替换歌词时间
        let re = regex::Regex::new(r"\[\d+:\d+.\d+\]").unwrap();
        // 修正不正常的时间戳 [00:11:22]
        let re_abnormal_ts = regex::Regex::new(r"^\[(\d+):(\d+):(\d+)\]").unwrap();
        if !lyric_path.exists() {
            if let Ok(lyr) = self.client.song_lyric(si.id).await {
                debug!("歌词: {:?}", lyr);
                // 添加歌词翻译
                let mut lt = Vec::new();
                for l in lyr.lyric.iter() {
                    let mut time = 0;
                    if l.len() >= 10 && re.is_match(l) {
                        // 时间戳可能是 [00:11:22] 这类异常格式，直接 unwrap 会 panic
                        let ts = |r: std::ops::Range<usize>| {
                            l.get(r)
                                .and_then(|s| s.parse::<u64>().ok())
                                .unwrap_or(0)
                        };
                        time = (ts(1..3) * 60 + ts(4..6)) * 1000 + ts(7..9) * 10;
                        let mut nl = re.replace_all(l, "").to_string();
                        nl.push('\n');
                        lt.push((time, nl));
                    }
                    for t in lyr.tlyric.iter() {
                        if t.len() >= 10 && l.len() >= 10 && t.starts_with(&l[0..10]) {
                            let mut nt = re.replace_all(t, "").to_string();
                            nt.push('\n');
                            lt.push((time, nt));
                        }
                    }
                }
                // 保存歌词文件
                let lyric = lyr
                    .lyric
                    .into_iter()
                    .map(|x| re_abnormal_ts.replace_all(&x, "[$1:$2.$3]").to_string())
                    .collect::<Vec<String>>()
                    .join("\n");
                fs::write(&lyric_path, lyric)?;
                if !lyr.tlyric.is_empty() {
                    // 保存翻译歌词文件
                    let tlyric = lyr
                        .tlyric
                        .into_iter()
                        .map(|x| re_abnormal_ts.replace_all(&x, "[$1:$2.$3]").to_string())
                        .collect::<Vec<String>>()
                        .join("\n");
                    fs::write(&tlyric_path, tlyric)?;
                }
                // 组织歌词+翻译
                Ok(lt)
            } else {
                anyhow::bail!("No lyrics found!")
            }
        } else {
            let lyric = fs::read_to_string(&lyric_path)?;
            let lyrics: Vec<String> = lyric
                .split('\n')
                .collect::<Vec<&str>>()
                .iter()
                .map(|s| s.to_string())
                .collect();
            let mut tlyrics = vec![];
            if tlyric_path.exists() {
                let tlyric = fs::read_to_string(&tlyric_path)?;
                tlyrics = tlyric
                    .split('\n')
                    .collect::<Vec<&str>>()
                    .iter()
                    .map(|s| s.to_string())
                    .collect();
            }
            // 添加歌词翻译
            let mut lt = Vec::new();
            for l in lyrics.iter() {
                let mut time = 0;
                if l.len() >= 10 && re.is_match(l) {
                    time = (l[1..3].parse::<u64>().unwrap() * 60 + l[4..6].parse::<u64>().unwrap())
                        * 1000
                        + l[7..9].parse::<u64>().unwrap_or(0) * 10;
                    let mut nl = re.replace_all(l, "").to_string();
                    nl.push('\n');
                    lt.push((time, nl));
                }
                for t in tlyrics.iter() {
                    if t.len() >= 10 && l.len() >= 10 && t.starts_with(&l[0..10]) {
                        let mut nt = re.replace_all(t, "").to_string();
                        nt.push('\n');
                        lt.push((time, nt));
                    }
                }
            }
            // 组织歌词+翻译
            Ok(lt)
        }
    }
}

impl Default for NcmClient {
    fn default() -> Self {
        Self::new()
    }
}

// 搜索节流：记录上一次真正发出请求的时间
static SEARCH_GATE: once_cell::sync::Lazy<std::sync::Mutex<std::time::Instant>> =
    once_cell::sync::Lazy::new(|| std::sync::Mutex::new(std::time::Instant::now()));

// 相邻两次搜索的最小间隔。联想接口实测 400ms 连发也不限流，这里留一倍余量
const SEARCH_INTERVAL_MS: u64 = 800;

// 公开搜索接口必须带浏览器 UA 与 Referer，否则返回 403
const WEB_UA: &str =
    "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) \
     Chrome/120.0.0.0 Safari/537.36";

// 独立的小客户端：不复用 ncm-api 内部的 client（拿不到），10 秒超时防挂住补全任务
static WEB_CLIENT: once_cell::sync::Lazy<isahc::HttpClient> = once_cell::sync::Lazy::new(|| {
    isahc::HttpClient::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .expect("创建搜索用 HTTP 客户端失败")
});

/// 搜索结果：必须把「确实没这首歌」和「被限流」分开，
/// 否则限流会被当成匹配失败，一路连带后面的歌全部放弃。
enum WebSearch {
    Songs(Vec<SongInfo>),
    RateLimited,
    Failed,
}

/// 搜索接口：`/api/search/get/web`（正式搜索）有很紧的 IP 频控（返回 code=405「操作频繁」，
/// 惩罚窗口长达十几分钟），而网页搜索框用的联想接口 `/api/search/suggest/web` 配额宽松得多、
/// 返回结构一致。本地补全只需要「歌名 → 歌曲 id」，用联想接口足够，且不会被限到不可用。
async fn search_web(keywords: &str, limit: u16) -> WebSearch {
    let body = format!("s={}&limit={}", url_encode(keywords), limit);
    let Ok(request) = isahc::Request::post("https://music.163.com/api/search/suggest/web")
        .header("User-Agent", WEB_UA)
        .header("Referer", "https://music.163.com/")
        .header(
            "Content-Type",
            "application/x-www-form-urlencoded;charset=utf-8",
        )
        .body(body)
    else {
        return WebSearch::Failed;
    };
    let mut response = match WEB_CLIENT.send_async(request).await {
        Ok(response) => response,
        Err(err) => {
            eprintln!("NCM_SEARCH_ERR [{}] {:?}", keywords, err);
            return WebSearch::Failed;
        }
    };
    let status = response.status();
    let text = match response.text().await {
        Ok(text) => text,
        Err(err) => {
            eprintln!("NCM_SEARCH_ERR [{}] 读取响应失败 {:?}", keywords, err);
            return WebSearch::Failed;
        }
    };
    if status.as_u16() == 429 {
        return WebSearch::RateLimited;
    }
    if !status.is_success() {
        eprintln!("NCM_SEARCH_ERR [{}] HTTP {}", keywords, status.as_u16());
        return WebSearch::Failed;
    }
    parse_web_search(&text)
}

/// 解析 /api/search/get/web 的返回体（结构与 weapi 版基本一致）
fn parse_web_search(text: &str) -> WebSearch {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(text) else {
        error!(
            "搜索响应不是 JSON：{}",
            text.chars().take(120).collect::<String>()
        );
        return WebSearch::Failed;
    };
    let code = value.get("code").and_then(|v| v.as_i64()).unwrap_or(0);
    // 405 = 操作频繁（IP 级频控），429 = 标准限流，两者都该退避重试
    if code == 405 || code == 429 {
        return WebSearch::RateLimited;
    }
    if code != 200 {
        error!(
            "搜索响应 code = {}：{}",
            code,
            text.chars().take(120).collect::<String>()
        );
        return WebSearch::Failed;
    }
    let Some(songs) = value
        .get("result")
        .and_then(|r| r.get("songs"))
        .and_then(|s| s.as_array())
    else {
        return WebSearch::Songs(Vec::new());
    };
    let mut list = Vec::with_capacity(songs.len());
    for song in songs {
        let Some(id) = song.get("id").and_then(|v| v.as_u64()) else {
            continue;
        };
        let name = song
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();
        let singer = song
            .get("artists")
            .and_then(|v| v.as_array())
            .map(|artists| {
                artists
                    .iter()
                    .filter_map(|a| a.get("name").and_then(|n| n.as_str()))
                    .collect::<Vec<&str>>()
                    .join("/")
            })
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "unknown".to_string());
        let album = song.get("album");
        let album_name = album
            .and_then(|a| a.get("name"))
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();
        let album_id = album
            .and_then(|a| a.get("id"))
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        // 老接口常常只有 picId 没有 picUrl，交给 match_song 用 songs_detail 补
        let pic_url = album
            .and_then(|a| a.get("picUrl"))
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();
        let duration = song.get("duration").and_then(|v| v.as_u64()).unwrap_or(0);
        list.push(SongInfo {
            id,
            name,
            singer,
            album: album_name,
            album_id,
            pic_url,
            duration,
            song_url: String::new(),
            quality: SongQualityState {
                available: vec![SongQuality::Standard],
                selected: None,
                actual: None,
            },
            copyright: SongCopyright::Unknown,
        });
    }
    WebSearch::Songs(list)
}

/// URL 百分号编码（只用于拼搜索关键字，避免为此引入 url crate）
fn url_encode(value: &str) -> String {
    let mut out = String::with_capacity(value.len() * 3);
    for byte in value.as_bytes() {
        match *byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(*byte as char)
            }
            b' ' => out.push('+'),
            other => out.push_str(&format!("%{other:02X}")),
        }
    }
    out
}

/// 往候选列表里加一组 (歌名, 歌手)，去重、跳过空歌名
fn push_pair(pairs: &mut Vec<(String, String)>, title: &str, artist: &str) {
    let title = title.trim();
    let artist = artist.trim();
    if title.is_empty() {
        return;
    }
    if pairs.iter().any(|(t, a)| t == title && a == artist) {
        return;
    }
    pairs.push((title.to_string(), artist.to_string()));
}

/// 去掉文件名里的曲目序号（"01." / "01 - " / "01、" / "01_ "）
fn strip_track_prefix(stem: &str) -> String {
    let s = stem.trim();
    let digits = s.chars().take_while(|c| c.is_ascii_digit()).count();
    if digits == 0 || digits > 3 {
        return s.to_string();
    }
    let rest = &s[digits..];
    let trimmed = rest.trim_start_matches(|c: char| matches!(c, '.' | '-' | '、' | '_' | ' '));
    if trimmed.len() == rest.len() || trimmed.is_empty() {
        return s.to_string();
    }
    trimmed.trim().to_string()
}

/// 去掉歌名里的括号补充说明（"（《电视原声》主题曲）"、" (Live)"、"[Remix]"），只留主标题
fn simplify_title(title: &str) -> String {
    let mut out = String::with_capacity(title.len());
    let mut depth = 0i32;
    for c in title.chars() {
        match c {
            '(' | '（' | '[' | '【' | '{' => depth += 1,
            ')' | '）' | ']' | '】' | '}' => {
                if depth > 0 {
                    depth -= 1;
                }
            }
            _ if depth == 0 => out.push(c),
            _ => {}
        }
    }
    let out = out.trim().trim_end_matches('-').trim();
    if out.is_empty() {
        title.trim().to_string()
    } else {
        out.to_string()
    }
}
