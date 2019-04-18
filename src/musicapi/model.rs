//
// model.rs
// Copyright (C) 2019 gmg137 <gmg137@live.com>
// Distributed under terms of the GPLv3 license.
//
use regex::Regex;
use serde::{Deserialize, Serialize};

#[allow(unused)]
pub fn to_lyric(json: String) -> Option<Vec<String>> {
    let mut re = Regex::new(r#""code":(?P<code>-{0,1}\d+)"#).unwrap();
    if let Some(code) = re.captures(&json) {
        let code = code
            .name("code")
            .unwrap()
            .as_str()
            .parse::<i32>()
            .unwrap_or(0);
        if code.eq(&200) {
            let mut vec: Vec<String> = Vec::new();
            let mut text = "";
            re = Regex::new(r#""lyric":"(?P<lyric>.+?)"\},"#).unwrap();
            if let Some(cap) = re.captures(&json) {
                text = cap.name("lyric").unwrap().as_str();
            }
            vec = text
                .split("\\n")
                .collect::<Vec<&str>>()
                .iter()
                .map(|s| s.to_string())
                .filter(|s| !s.is_empty())
                .collect::<Vec<String>>();
            return Some(vec);
        }
        None
    } else {
        None
    }
}

// 歌手信息
#[derive(Debug, Deserialize, Serialize)]
pub struct SingerInfo {
    // 歌手 id
    pub id: u32,
    // 歌手姓名
    pub name: String,
    // 歌手照片
    pub pic_url: String,
}

#[allow(unused)]
pub fn to_singer_info(json: String) -> Option<Vec<SingerInfo>> {
    let mut re = Regex::new(r#""code":(?P<code>-{0,1}\d+)"#).unwrap();
    if let Some(code) = re.captures(&json) {
        let code = code
            .name("code")
            .unwrap()
            .as_str()
            .parse::<i32>()
            .unwrap_or(0);
        if code.eq(&200) {
            let mut vec: Vec<SingerInfo> = Vec::new();
            re = Regex::new(
                r#""id":(?P<id>\d+),"name":"(?P<name>.{1,50})","picUrl":"(?P<pic_url>.+?.jpg)""#,
            )
            .unwrap();
            for cap in re.captures_iter(&json) {
                vec.push(SingerInfo {
                    id: cap.name("id").unwrap().as_str().parse::<u32>().unwrap_or(0),
                    name: cap.name("name").unwrap().as_str().to_owned(),
                    pic_url: cap.name("pic_url").unwrap().as_str().to_owned(),
                })
            }
            return Some(vec);
        }
        None
    } else {
        None
    }
}

// 歌曲 URL
#[derive(Debug, Deserialize, Serialize)]
pub struct SongUrl {
    // 歌曲 id
    pub id: u32,
    // 歌曲 URL
    pub url: String,
    // 码率
    pub rate: u32,
}

#[allow(unused)]
pub fn to_song_url(json: String) -> Option<Vec<SongUrl>> {
    let re = Regex::new(r#""code":(?P<code>-{0,1}\d+)"#).unwrap();
    if let Some(code) = re.captures(&json) {
        let code = code
            .name("code")
            .unwrap()
            .as_str()
            .parse::<i32>()
            .unwrap_or(0);
        if code.eq(&200) {
            let mut vec = Vec::new();
            let re =
                Regex::new(r#""id":(?P<id>\d+),"url":"(?P<url>.+?)","br":(?P<rate>\d+)"#).unwrap();
            for cap in re.captures_iter(&json) {
                vec.push(SongUrl {
                    id: cap.name("id").unwrap().as_str().parse::<u32>().unwrap_or(0),
                    url: cap.name("url").unwrap().as_str().to_owned(),
                    rate: cap
                        .name("rate")
                        .unwrap()
                        .as_str()
                        .parse::<u32>()
                        .unwrap_or(0),
                })
            }
            return Some(vec);
        }
        None
    } else {
        None
    }
}

// 歌曲信息
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct SongInfo {
    // 歌曲 id
    pub id: u32,
    // 歌名
    pub name: String,
    // 歌手
    pub singer: String,
    // 专辑
    pub album: String,
    // 封面图
    pub pic_url: String,
    // 歌曲时长
    pub duration: String,
    // 歌曲链接
    pub song_url: String,
}

// parse: 解析方式
#[allow(unused)]
pub fn to_song_info(json: String, parse: Parse) -> Option<Vec<SongInfo>> {
    let mut re = Regex::new(r#""code":(?P<code>-{0,1}\d+)"#).unwrap();
    if let Some(code) = re.captures(&json) {
        let code = code
            .name("code")
            .unwrap()
            .as_str()
            .parse::<i32>()
            .unwrap_or(0);
        if code.eq(&200) {
            let mut vec: Vec<SongInfo> = Vec::new();
            match parse {
                Parse::USL => {
                    re = Regex::new(
                r#""name":"(?P<name>.{1,50})","id":(?P<id>\d+).+?"name":"(?P<singer>.+?)".+?"al":.+?"name":"(?P<album>.+?)","picUrl":"(?P<pic_url>.+?.jpg)".+?"dt":(?P<duration>\d+)"#,
            )
            .unwrap();
                }
                Parse::RMD => {
                    re = Regex::new(
                r#""name":"(?P<name>.{1,50})","id":(?P<id>\d+),"position.+?"name":"(?P<singer>.+?)".+?"picUrl":"(?P<pic_url>.+?.jpg)".+?"name":"(?P<album>.+?)",.+?"duration":(?P<duration>\d+)"#,
            )
            .unwrap();
                }
                Parse::RMDS => {
                    re = Regex::new(
                r#""name":"(?P<name>.{1,50})","id":(?P<id>\d+),"position.+?"name":"(?P<singer>.+?)".+?"name":"(?P<album>.+?)".+?\d+,"picUrl":"(?P<pic_url>.+?.jpg)","publishTime.+?"duration":(?P<duration>\d+)"#,
            )
            .unwrap();
                }
                Parse::SEARCH => {
                    re = Regex::new(
                r#""name":"(?P<name>.{1,50})","id":(?P<id>\d+),.+?"name":"(?P<singer>.+?)".+?"name":"(?P<album>.{1,50}?)","picUrl":"(?P<pic_url>.+?.jpg)".+?"dt":(?P<duration>\d+)"#,
            )
            .unwrap();
                }
                Parse::SD => {
                    re = Regex::new(
                r#""name":"(?P<name>.{1,50})","id":(?P<id>\d+),"pst.+?"name":"(?P<singer>.+?)".+?"name":"(?P<album>.+?)".+?"picUrl":"(?P<pic_url>.+?.jpg)",.+?"dt":(?P<duration>\d+)"#,
            )
            .unwrap();
                }
                Parse::ALBUM => {
                    re = Regex::new(
                r#""dt":(?P<duration>\d+),.+?"name":"(?P<name>.{1,50})","id":(?P<id>\d+).+?(?P<singer>0).+?(?P<album>0).+?(?P<pic_url>0)"#,
            )
            .unwrap();
                }
                _ => {}
            }
            for cap in re.captures_iter(&json) {
                let duration = cap
                    .name("duration")
                    .unwrap()
                    .as_str()
                    .parse::<u32>()
                    .unwrap_or(0);
                vec.push(SongInfo {
                    id: cap.name("id").unwrap().as_str().parse::<u32>().unwrap_or(0),
                    name: cap.name("name").unwrap().as_str().to_owned(),
                    singer: cap.name("singer").unwrap().as_str().to_owned(),
                    album: cap.name("album").unwrap().as_str().to_owned(),
                    pic_url: cap.name("pic_url").unwrap().as_str().to_owned(),
                    duration: format!("{:0>2}:{:0>2}", duration / 1000 / 60, duration / 1000 % 60),
                    song_url: String::new(),
                })
            }
            return Some(vec);
        }
        None
    } else {
        None
    }
}

// 歌单信息
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SongList {
    // 歌单 id
    pub id: u32,
    // 歌单名
    pub name: String,
    // 歌单封面
    pub cover_img_url: String,
}

// parse: 解析方式
#[allow(unused)]
pub fn to_song_list(json: String, parse: Parse) -> Option<Vec<SongList>> {
    let mut re = Regex::new(r#""code":(?P<code>-{0,1}\d+)"#).unwrap();
    if let Some(code) = re.captures(&json) {
        let code = code
            .name("code")
            .unwrap()
            .as_str()
            .parse::<i32>()
            .unwrap_or(0);
        if code.eq(&200) {
            let mut vec = Vec::new();
            match parse {
                Parse::USL => {
                    re = Regex::new(r#""coverImgUrl":"(?P<cover_img_url>.+?jpg).+?"name":"(?P<name>.+?)","id":(?P<id>\d+)"#).unwrap();
                }
                Parse::RMD => {
                    re = Regex::new(
                    r#""id":(?P<id>\d+),.+?"name":"(?P<name>.+?)".+?"picUrl":"(?P<cover_img_url>.+?jpg)""#,
                ).unwrap();
                }
                Parse::ALBUM => {
                    re = Regex::new(r#"publishTime":\d+,.+?"picUrl":"(?P<cover_img_url>.+?jpg)".+?"name":"(?P<name>.+?)","id":(?P<id>\d+),"#).unwrap();
                }
                Parse::TOP => {
                    re = Regex::new(r#""name":"(?P<name>.{1,30})","id":(?P<id>\d+),.+?"coverImgUrl":"(?P<cover_img_url>.+?jpg)""#).unwrap();
                }
                _ => {}
            }
            for cap in re.captures_iter(&json) {
                vec.push(SongList {
                    id: cap.name("id").unwrap().as_str().parse::<u32>().unwrap_or(0),
                    name: cap.name("name").unwrap().as_str().to_owned(),
                    cover_img_url: cap.name("cover_img_url").unwrap().as_str().to_owned(),
                })
            }
            return Some(vec);
        }
        None
    } else {
        None
    }
}

// 消息
#[derive(Debug, Deserialize, Serialize)]
pub struct Msg {
    pub code: i32,
    pub msg: String,
}

#[allow(unused)]
pub fn to_msg(json: String) -> Option<Msg> {
    let re = Regex::new(r#""code":(?P<code>-{0,1}\d+)"#).unwrap();
    if let Some(code) = re.captures(&json) {
        let code = code
            .name("code")
            .unwrap()
            .as_str()
            .parse::<i32>()
            .unwrap_or(0);
        if code.eq(&200) {
            Some(Msg {
                code,
                msg: "".to_owned(),
            })
        } else {
            let re = Regex::new(r#""msg":"(?P<msg>\w+)"#).unwrap();
            if let Some(msg) = re.captures(&json) {
                Some(Msg {
                    code,
                    msg: msg.name("msg").unwrap().as_str().to_owned(),
                })
            } else {
                None
            }
        }
    } else {
        None
    }
}

// 登陆信息
#[derive(Debug, Deserialize, Serialize)]
pub struct LoginInfo {
    // 登陆状态码
    pub code: i32,
    // 用户 id
    pub uid: u32,
    // 用户昵称
    pub nickname: String,
    // 用户头像
    pub avatar_url: String,
    // 状态消息
    pub msg: String,
}

#[allow(unused)]
pub fn to_login_info(json: String) -> Option<LoginInfo> {
    let re = Regex::new(r#""code":(?P<code>-{0,1}\d+)"#).unwrap();
    if let Some(code) = re.captures(&json) {
        let code = code
            .name("code")
            .unwrap()
            .as_str()
            .parse::<i32>()
            .unwrap_or(0);
        if code.eq(&200) {
            let re = Regex::new(r#""id":(?P<id>\d+).+"avatarUrl":"(?P<avatar_url>.+?jpg)".+"nickname":"(?P<nickname>.+?)""#).unwrap();
            if let Some(cap) = re.captures(&json) {
                let uid = cap.name("id").unwrap().as_str().parse::<u32>().unwrap_or(0);
                let nickname = cap.name("nickname").unwrap().as_str().to_owned();
                let avatar_url = cap.name("avatar_url").unwrap().as_str().to_owned();
                Some(LoginInfo {
                    code,
                    uid,
                    nickname,
                    avatar_url,
                    msg: "".to_owned(),
                })
            } else {
                let re = Regex::new(r#""id":(?P<id>\d+).+"nickname":"(?P<nickname>.+?)".+"avatarUrl":"(?P<avatar_url>.+?jpg)"#).unwrap();
                if let Some(cap) = re.captures(&json) {
                    let uid = cap.name("id").unwrap().as_str().parse::<u32>().unwrap_or(0);
                    let nickname = cap.name("nickname").unwrap().as_str().to_owned();
                    let avatar_url = cap.name("avatar_url").unwrap().as_str().to_owned();
                    Some(LoginInfo {
                        code,
                        uid,
                        nickname,
                        avatar_url,
                        msg: "".to_owned(),
                    })
                } else {
                    None
                }
            }
        } else {
            let re = Regex::new(r#""msg":"(?P<msg>\w+)"#).unwrap();
            if let Some(msg) = re.captures(&json) {
                Some(LoginInfo {
                    code,
                    uid: 0,
                    nickname: "".to_owned(),
                    avatar_url: "".to_owned(),
                    msg: msg.name("msg").unwrap().as_str().to_owned(),
                })
            } else {
                None
            }
        }
    } else {
        None
    }
}

// 请求方式
#[allow(unused)]
#[derive(Debug)]
pub enum Method {
    POST,
    GET,
}

// 解析方式
// USL: 用户
// RMD: 推荐
// RMDS: 推荐歌曲
// SEARCH: 搜索
// SD: 单曲详情
// ALBUM: 专辑
// TOP: 热门
#[allow(unused)]
#[derive(Debug)]
pub enum Parse {
    USL,
    RMD,
    RMDS,
    SEARCH,
    SD,
    ALBUM,
    TOP,
}
