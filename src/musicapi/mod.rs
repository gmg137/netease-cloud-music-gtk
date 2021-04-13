//
// mod.rs
// Copyright (C) 2019 gmg137 <gmg137@live.com>
// Distributed under terms of the GPLv3 license.
//
mod encrypt;
pub(crate) mod model;
use crate::model::{Errors, NCMResult};
use encrypt::Crypto;
use isahc::{prelude::*, *};
use lazy_static::lazy_static;
use model::*;
use regex::Regex;
use std::{collections::HashMap, time::Duration};
use urlqstring::QueryParams;

lazy_static! {
    static ref _CSRF: Regex = Regex::new(r"_csrf=(?P<csrf>[^(;|$)]+)").unwrap();
}

static BASE_URL: &str = "https://music.163.com";

const LINUX_USER_AGNET: &str =
    "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/60.0.3112.90 Safari/537.36";

const USER_AGENT_LIST: [&str; 14] = [
    "Mozilla/5.0 (iPhone; CPU iPhone OS 9_1 like Mac OS X) AppleWebKit/601.1.46 (KHTML, like Gecko) Version/9.0 Mobile/13B143 Safari/601.1",
    "Mozilla/5.0 (iPhone; CPU iPhone OS 9_1 like Mac OS X) AppleWebKit/601.1.46 (KHTML, like Gecko) Version/9.0 Mobile/13B143 Safari/601.1",
    "Mozilla/5.0 (Linux; Android 5.0; SM-G900P Build/LRX21T) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/59.0.3071.115 Mobile Safari/537.36",
    "Mozilla/5.0 (Linux; Android 6.0; Nexus 5 Build/MRA58N) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/59.0.3071.115 Mobile Safari/537.36",
    "Mozilla/5.0 (Linux; Android 5.1.1; Nexus 6 Build/LYZ28E) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/59.0.3071.115 Mobile Safari/537.36",
    "Mozilla/5.0 (iPhone; CPU iPhone OS 10_3_2 like Mac OS X) AppleWebKit/603.2.4 (KHTML, like Gecko) Mobile/14F89;GameHelper",
    "Mozilla/5.0 (iPhone; CPU iPhone OS 10_0 like Mac OS X) AppleWebKit/602.1.38 (KHTML, like Gecko) Version/10.0 Mobile/14A300 Safari/602.1",
    "Mozilla/5.0 (iPad; CPU OS 10_0 like Mac OS X) AppleWebKit/602.1.38 (KHTML, like Gecko) Version/10.0 Mobile/14A300 Safari/602.1",
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10.12; rv:46.0) Gecko/20100101 Firefox/46.0",
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_12_5) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/59.0.3071.115 Safari/537.36",
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_12_5) AppleWebKit/603.2.4 (KHTML, like Gecko) Version/10.1.1 Safari/603.2.4",
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:46.0) Gecko/20100101 Firefox/46.0",
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/51.0.2704.103 Safari/537.36",
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/42.0.2311.135 Safari/537.36 Edge/13.1058",
];

pub struct MusicApi {
    client: HttpClient,
    csrf: String,
}

#[allow(unused)]
enum CryptoApi {
    WEAPI,
    LINUXAPI,
}

impl MusicApi {
    #[allow(unused)]
    pub fn new() -> Self {
        let client = HttpClient::builder()
            .timeout(Duration::from_secs(20))
            .cookies()
            .build()
            .expect("初始化网络请求失败!");
        Self {
            client,
            csrf: String::new(),
        }
    }

    // 发送请求
    // method: 请求方法
    // path: 请求路径
    // params: 请求参数
    // cryptoapi: 请求加密方式
    // ua: 要使用的 USER_AGENT_LIST
    async fn request(
        &mut self,
        method: Method,
        path: &str,
        params: HashMap<&str, &str>,
        cryptoapi: CryptoApi,
        ua: &str,
    ) -> NCMResult<String> {
        let mut url = format!("{}{}", BASE_URL, path);
        match method {
            Method::POST => {
                let user_agent = match cryptoapi {
                    CryptoApi::LINUXAPI => LINUX_USER_AGNET.to_string(),
                    CryptoApi::WEAPI => choose_user_agent(ua).to_string(),
                };
                let body = match cryptoapi {
                    CryptoApi::LINUXAPI => {
                        let data = format!(
                            r#"{{"method":"linuxapi","url":"{}","params":{}}}"#,
                            url.replace("weapi", "api"),
                            QueryParams::from_map(params).json()
                        );
                        url = "https://music.163.com/api/linux/forward".to_owned();
                        Crypto::linuxapi(&data)
                    }
                    CryptoApi::WEAPI => {
                        let mut params = params;
                        params.insert("csrf_token", &self.csrf[..]);
                        Crypto::weapi(&QueryParams::from_map(params).json())
                    }
                };

                let request = Request::post(&url)
                    .header("Cookie", "os=pc; appver=2.7.1.198277")
                    .header("Accept", "*/*")
                    .header("Accept-Encoding", "gzip,deflate,br")
                    .header("Accept-Language", "en-US,en;q=0.5")
                    .header("Connection", "keep-alive")
                    .header("Content-Type", "application/x-www-form-urlencoded")
                    .header("Host", "music.163.com")
                    .header("Referer", "https://music.163.com")
                    .header("User-Agent", user_agent)
                    .body(body)
                    .unwrap();
                let mut response = self.client.send_async(request).await.map_err(|_| Errors::NoneError)?;
                if self.csrf.is_empty() {
                    for (k, v) in response.headers() {
                        let v = v.to_str().unwrap_or("");
                        if k.eq("set-cookie") && v.contains("__csrf") {
                            let csrf_token = if let Some(caps) = _CSRF.captures(v) {
                                caps.name("csrf").unwrap().as_str()
                            } else {
                                ""
                            };
                            self.csrf = csrf_token.to_owned();
                        }
                    }
                }
                response.text().await.map_err(|_| Errors::NoneError)
            }
            Method::GET => self
                .client
                .get_async(&url)
                .await
                .map_err(|_| Errors::NoneError)?
                .text()
                .await
                .map_err(|_| Errors::NoneError),
        }
    }

    // 登录
    // username: 用户名(邮箱或手机)
    // password: 密码
    #[allow(unused)]
    pub async fn login(&mut self, username: String, password: String) -> NCMResult<LoginInfo> {
        let mut params = HashMap::new();
        let path;
        if username.len().eq(&11) && username.parse::<u64>().is_ok() {
            path = "/weapi/login/cellphone";
            params.insert("phone", &username[..]);
            params.insert("password", &password[..]);
            params.insert("rememberLogin", "true");
        } else {
            let client_token = "1_jVUMqWEPke0/1/Vu56xCmJpo5vP1grjn_SOVVDzOc78w8OKLVZ2JH7IfkjSXqgfmh";
            path = "/weapi/login";
            params.insert("username", &username[..]);
            params.insert("password", &password[..]);
            params.insert("rememberLogin", "true");
            params.insert("clientToken", client_token);
        }
        let result = self.request(Method::POST, path, params, CryptoApi::WEAPI, "").await?;
        to_login_info(result)
    }

    // 登陆状态
    #[allow(unused)]
    pub async fn login_status(&mut self) -> NCMResult<LoginInfo> {
        let result = self
            .request(Method::GET, "", HashMap::new(), CryptoApi::WEAPI, "")
            .await?;
        let re = regex::Regex::new(
            r#"userId:(?P<id>\d+),nickname:"(?P<nickname>\w+)",avatarUrl.+?(?P<avatar_url>http.+?jpg)""#,
        )?;
        let cap = re.captures(&result).ok_or(Errors::NoneError)?;
        let uid = cap.name("id").ok_or(Errors::NoneError)?.as_str().parse::<u64>()?;
        let nickname = cap.name("nickname").ok_or(Errors::NoneError)?.as_str().to_owned();
        let avatar_url = cap.name("avatar_url").ok_or(Errors::NoneError)?.as_str().to_owned();
        Ok(LoginInfo {
            code: 200,
            uid,
            nickname,
            avatar_url,
            msg: "已登录.".to_owned(),
        })
    }

    // 退出
    #[allow(unused)]
    pub async fn logout(&mut self) {
        let path = "https://music.163.com/weapi/logout";
        self.request(Method::POST, path, HashMap::new(), CryptoApi::WEAPI, "pc")
            .await;
    }

    // 每日签到
    #[allow(unused)]
    pub async fn daily_task(&mut self) -> NCMResult<Msg> {
        let path = "/weapi/point/dailyTask";
        let mut params = HashMap::new();
        params.insert("type", "0");
        let result = self.request(Method::POST, path, params, CryptoApi::WEAPI, "").await?;
        to_msg(result)
    }

    // 用户歌单
    // uid: 用户id
    // offset: 列表起点号
    // limit: 列表长度
    #[allow(unused)]
    pub async fn user_song_list(&mut self, uid: u64, offset: u8, limit: u8) -> NCMResult<Vec<SongList>> {
        let path = "/weapi/user/playlist";
        let mut params = HashMap::new();
        let uid = uid.to_string();
        let offset = offset.to_string();
        let limit = limit.to_string();
        params.insert("uid", uid.as_str());
        params.insert("offset", offset.as_str());
        params.insert("limit", limit.as_str());
        let result = self.request(Method::POST, path, params, CryptoApi::WEAPI, "").await?;
        to_song_list(result, Parse::USL)
    }

    // 用户云盘
    #[allow(unused)]
    pub async fn user_cloud_disk(&mut self) -> NCMResult<Vec<SongInfo>> {
        let path = "/weapi/v1/cloud/get";
        let mut params = HashMap::new();
        params.insert("offset", "0");
        params.insert("limit", "1000");
        let result = self.request(Method::POST, path, params, CryptoApi::WEAPI, "").await?;
        to_song_info(result, Parse::UCD)
    }

    // 歌单详情
    // songlist_id: 歌单 id
    #[allow(unused)]
    pub async fn song_list_detail(&mut self, songlist_id: u64) -> NCMResult<Vec<SongInfo>> {
        let csrf_token = self.csrf.to_owned();
        let path = "/weapi/v6/playlist/detail";
        let mut params = HashMap::new();
        let songlist_id = songlist_id.to_string();
        params.insert("id", songlist_id.as_str());
        params.insert("offset", "0");
        params.insert("total", "true");
        params.insert("limit", "1000");
        params.insert("n", "1000");
        params.insert("csrf_token", &csrf_token);
        let result = self.request(Method::POST, path, params, CryptoApi::WEAPI, "").await?;
        to_song_info(result, Parse::USL)
    }

    // 歌曲详情
    // ids: 歌曲 id 列表
    #[allow(unused)]
    pub async fn songs_detail(&mut self, ids: &[u64]) -> NCMResult<Vec<SongInfo>> {
        let path = "/weapi/v3/song/detail";
        let mut params = HashMap::new();
        let c = format!(
            r#""[{{"id":{}}}]""#,
            ids.iter().map(|i| i.to_string()).collect::<Vec<String>>().join(",")
        );
        let ids = format!(
            r#""[{}]""#,
            ids.iter().map(|i| i.to_string()).collect::<Vec<String>>().join(",")
        );
        params.insert("c", &c[..]);
        params.insert("ids", &ids[..]);
        let result = self.request(Method::POST, path, params, CryptoApi::WEAPI, "").await?;
        to_song_info(result, Parse::USL)
    }

    // 歌曲 URL
    // ids: 歌曲列表
    #[allow(unused)]
    pub async fn songs_url(&mut self, ids: &[u64]) -> NCMResult<Vec<SongUrl>> {
        let csrf_token = self.csrf.to_owned();
        let path = "/weapi/song/enhance/player/url/v1";
        let mut params = HashMap::new();
        let ids = serde_json::to_string(ids)?;
        params.insert("ids", ids.as_str());
        params.insert("level", "standard");
        params.insert("encodeType", "aac");
        params.insert("csrf_token", &csrf_token);
        let result = self.request(Method::POST, path, params, CryptoApi::WEAPI, "").await?;
        to_song_url(result)
    }

    // 每日推荐歌单
    #[allow(unused)]
    pub async fn recommend_resource(&mut self) -> NCMResult<Vec<SongList>> {
        let path = "/weapi/v1/discovery/recommend/resource";
        let result = self
            .request(Method::POST, path, HashMap::new(), CryptoApi::WEAPI, "")
            .await?;
        to_song_list(result, Parse::RMD)
    }

    // 每日推荐歌曲
    #[allow(unused)]
    pub async fn recommend_songs(&mut self) -> NCMResult<Vec<SongInfo>> {
        let path = "/weapi/v2/discovery/recommend/songs";
        let mut params = HashMap::new();
        params.insert("total", "ture");
        let result = self.request(Method::POST, path, params, CryptoApi::WEAPI, "").await?;
        to_song_info(result, Parse::RMDS)
    }

    // 私人FM
    #[allow(unused)]
    pub async fn personal_fm(&mut self) -> NCMResult<Vec<SongInfo>> {
        let path = "/weapi/v1/radio/get";
        let result = self
            .request(Method::POST, path, HashMap::new(), CryptoApi::WEAPI, "")
            .await?;
        to_song_info(result, Parse::RMD)
    }

    // 收藏/取消收藏
    // songid: 歌曲id
    // like: true 收藏，false 取消
    #[allow(unused)]
    pub async fn like(&mut self, like: bool, songid: u64) -> bool {
        let path = "/weapi/radio/like";
        let mut params = HashMap::new();
        let songid = songid.to_string();
        let like = like.to_string();
        params.insert("alg", "itembased");
        params.insert("trackId", songid.as_str());
        params.insert("like", like.as_str());
        params.insert("time", "25");
        if let Ok(result) = self.request(Method::POST, path, params, CryptoApi::WEAPI, "").await {
            return to_msg(result)
                .unwrap_or(Msg {
                    code: 0,
                    msg: "".to_owned(),
                })
                .code
                .eq(&200);
        }
        false
    }

    // FM 不喜欢
    // songid: 歌曲id
    #[allow(unused)]
    pub async fn fm_trash(&mut self, songid: u64) -> bool {
        let path = "/weapi/radio/trash/add";
        let mut params = HashMap::new();
        let songid = songid.to_string();
        params.insert("alg", "RT");
        params.insert("songId", songid.as_str());
        params.insert("time", "25");
        if let Ok(result) = self.request(Method::POST, path, params, CryptoApi::WEAPI, "").await {
            return to_msg(result)
                .unwrap_or(Msg {
                    code: 0,
                    msg: "".to_owned(),
                })
                .code
                .eq(&200);
        }
        false
    }

    // 搜索
    // keywords: 关键词
    // types: 单曲(1)，歌手(100)，专辑(10)，歌单(1000)，用户(1002) *(type)*
    // offset: 起始点
    // limit: 数量
    #[allow(unused)]
    pub async fn search(&mut self, keywords: String, types: u32, offset: u16, limit: u16) -> NCMResult<String> {
        let path = "/weapi/search/get";
        let mut params = HashMap::new();
        let _types = types.to_string();
        let offset = offset.to_string();
        let limit = limit.to_string();
        params.insert("s", &keywords[..]);
        params.insert("type", &_types[..]);
        params.insert("offset", &offset[..]);
        params.insert("limit", &limit[..]);
        let result = self.request(Method::POST, path, params, CryptoApi::WEAPI, "").await?;
        match types {
            1 => to_song_info(result, Parse::SEARCH).and_then(|s| Ok(serde_json::to_string(&s)?)),
            100 => to_singer_info(result).and_then(|s| Ok(serde_json::to_string(&s)?)),
            _ => Err(Errors::NoneError),
        }
    }

    // 新碟上架
    // offset: 起始点
    // limit: 数量
    #[allow(unused)]
    pub async fn new_albums(&mut self, offset: u8, limit: u8) -> NCMResult<Vec<SongList>> {
        let path = "/weapi/album/new";
        let mut params = HashMap::new();
        let offset = offset.to_string();
        let limit = limit.to_string();
        params.insert("area", "ALL");
        params.insert("offset", &offset[..]);
        params.insert("limit", &limit[..]);
        params.insert("total", "true");
        let result = self.request(Method::POST, path, params, CryptoApi::WEAPI, "").await?;
        to_song_list(result, Parse::ALBUM)
    }

    // 专辑
    // album_id: 专辑 id
    #[allow(unused)]
    pub async fn album(&mut self, album_id: u64) -> NCMResult<Vec<SongInfo>> {
        let path = format!("/weapi/v1/album/{}", album_id);
        let result = self
            .request(Method::POST, &path, HashMap::new(), CryptoApi::WEAPI, "")
            .await?;
        to_song_info(result, Parse::ALBUM)
    }

    // 热门推荐歌单
    // offset: 起始点
    // limit: 数量
    // order: 排序方式:
    //	      "hot": 热门，
    //        "new": 最新
    #[allow(unused)]
    pub async fn top_song_list(&mut self, order: &str, offset: u8, limit: u8) -> NCMResult<Vec<SongList>> {
        let path = "/weapi/playlist/list";
        let mut params = HashMap::new();
        let offset = offset.to_string();
        let limit = limit.to_string();
        params.insert("cat", "全部");
        params.insert("order", order);
        params.insert("total", "true");
        params.insert("offset", &offset[..]);
        params.insert("limit", &limit[..]);
        let result = self.request(Method::POST, path, params, CryptoApi::WEAPI, "").await?;
        to_song_list(result, Parse::TOP)
    }

    // 热门歌曲/排行榜
    // list_id:
    // 云音乐飙升榜: 19723756
    // 云音乐新歌榜: 3779629
    // 网易原创歌曲榜: 2884035
    // 云音乐热歌榜: 3778678
    // 云音乐古典音乐榜: 71384707
    // 云音乐ACG音乐榜: 71385702
    // 云音乐韩语榜: 745956260
    // 云音乐国电榜: 10520166
    // 云音乐嘻哈榜: 991319590']
    // 抖音排行榜: 2250011882
    // UK排行榜周榜: 180106
    // 美国Billboard周榜: 60198
    // KTV嗨榜: 21845217
    // iTunes榜: 11641012
    // Hit FM Top榜: 120001
    // 日本Oricon周榜: 60131
    // 台湾Hito排行榜: 112463
    // 香港电台中文歌曲龙虎榜: 10169002
    // 华语金曲榜: 4395559
    #[allow(unused)]
    pub async fn top_songs(&mut self, list_id: u64) -> NCMResult<Vec<SongInfo>> {
        self.song_list_detail(list_id).await
    }

    // 查询歌词
    // music_id: 歌曲id
    #[allow(unused)]
    pub async fn song_lyric(&mut self, music_id: u64) -> NCMResult<Vec<String>> {
        let csrf_token = self.csrf.to_owned();
        let path = "/weapi/song/lyric";
        let mut params = HashMap::new();
        let id = music_id.to_string();
        params.insert("id", &id[..]);
        params.insert("lv", "-1");
        params.insert("tv", "-1");
        params.insert("csrf_token", &csrf_token);
        let result = self.request(Method::POST, path, params, CryptoApi::WEAPI, "").await?;
        to_lyric(result)
    }

    // 收藏/取消收藏歌单
    // like: true 收藏，false 取消
    // id: 歌单 id
    #[allow(unused)]
    pub async fn song_list_like(&mut self, like: bool, id: u64) -> bool {
        let path = if like {
            "/weapi/playlist/subscribe"
        } else {
            "/weapi/playlist/unsubscribe"
        };
        let mut params = HashMap::new();
        let id = id.to_string();
        params.insert("id", &id[..]);
        if let Ok(result) = self.request(Method::POST, path, params, CryptoApi::WEAPI, "").await {
            return to_msg(result)
                .unwrap_or(Msg {
                    code: 0,
                    msg: "".to_owned(),
                })
                .code
                .eq(&200);
        }
        false
    }
}

fn choose_user_agent(ua: &str) -> &str {
    let index = if ua == "mobile" {
        rand::random::<usize>() % 7
    } else if ua == "pc" {
        rand::random::<usize>() % 5 + 8
    } else if !ua.is_empty() {
        return ua;
    } else {
        rand::random::<usize>() % USER_AGENT_LIST.len()
    };
    USER_AGENT_LIST[index]
}
