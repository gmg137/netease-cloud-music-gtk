//
// model.rs
// Copyright (C) 2019 gmg137 <gmg137@live.com>
// Distributed under terms of the GPLv3 license.
//
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[allow(unused)]
pub fn to_lyric(json: String) -> Option<Vec<String>> {
    if let Ok(value) = serde_json::from_str::<Value>(&json) {
        if value.get("code").unwrap_or(&json!(0)).eq(&200) {
            let mut vec: Vec<String> = Vec::new();
            let lyric = value
                .get("lrc")
                .unwrap_or(&json!(null))
                .get("lyric")
                .unwrap_or(&json!(""))
                .as_str()
                .unwrap_or("")
                .to_owned();
            vec = lyric
                .split("\n")
                .collect::<Vec<&str>>()
                .iter()
                .map(|s| s.to_string())
                .filter(|s| !s.is_empty())
                .collect::<Vec<String>>();
            if vec.is_empty() {
                return None;
            }
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
    if let Ok(value) = serde_json::from_str::<Value>(&json) {
        if value.get("code").unwrap_or(&json!(0)).eq(&200) {
            let mut vec: Vec<SingerInfo> = Vec::new();
            let list = json!([]);
            let array = value
                .get("result")
                .unwrap_or(&json!(null))
                .get("artists")
                .unwrap_or(&list)
                .as_array()
                .unwrap();
            array.iter().for_each(|v| {
                vec.push(SingerInfo {
                    id: v.get("id").unwrap_or(&json!(0)).as_u64().unwrap() as u32,
                    name: v.get("name").unwrap_or(&json!("")).as_str().unwrap_or("").to_owned(),
                    pic_url: v.get("picUrl").unwrap_or(&json!("")).as_str().unwrap_or("").to_owned(),
                });
            });
            if vec.is_empty() {
                return None;
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
    if let Ok(value) = serde_json::from_str::<Value>(&json) {
        if value.get("code").unwrap_or(&json!(0)).eq(&200) {
            let mut vec: Vec<SongUrl> = Vec::new();
            let list = json!([]);
            let array = value.get("data").unwrap_or(&list).as_array().unwrap();
            array.iter().for_each(|v| {
                let url = v.get("url").unwrap_or(&json!("")).as_str().unwrap_or("").to_owned();
                if !url.is_empty() {
                    vec.push(SongUrl {
                        id: v.get("id").unwrap_or(&json!(0)).as_u64().unwrap() as u32,
                        url,
                        rate: v.get("br").unwrap_or(&json!(0)).as_u64().unwrap() as u32,
                    });
                }
            });
            if vec.is_empty() {
                return None;
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
    if let Ok(value) = serde_json::from_str::<Value>(&json) {
        if value.get("code").unwrap_or(&json!(0)).eq(&200) {
            let mut vec: Vec<SongInfo> = Vec::new();
            let list = json!([]);
            match parse {
                Parse::USL => {
                    let mut array = value.get("songs").unwrap_or(&list).as_array().unwrap();
                    if array.is_empty() {
                        array = value
                            .get("playlist")
                            .unwrap_or(&json!(null))
                            .get("tracks")
                            .unwrap_or(&list)
                            .as_array()
                            .unwrap();
                    }
                    array.iter().for_each(|v| {
                        let duration = v.get("dt").unwrap_or(&json!(0)).as_u64().unwrap() as u32;
                        vec.push(SongInfo {
                            id: v.get("id").unwrap_or(&json!(0)).as_u64().unwrap() as u32,
                            name: v.get("name").unwrap_or(&json!("")).as_str().unwrap_or("").to_owned(),
                            singer: v
                                .get("ar")
                                .unwrap_or(&json!(&list))
                                .get(0)
                                .unwrap_or(&json!(null))
                                .get("name")
                                .unwrap_or(&json!(""))
                                .as_str()
                                .unwrap_or("")
                                .to_owned(),
                            album: v
                                .get("al")
                                .unwrap_or(&json!(null))
                                .get("name")
                                .unwrap_or(&json!(""))
                                .as_str()
                                .unwrap_or("")
                                .to_owned(),
                            pic_url: v
                                .get("al")
                                .unwrap_or(&json!(null))
                                .get("picUrl")
                                .unwrap_or(&json!(""))
                                .as_str()
                                .unwrap_or("")
                                .to_owned(),
                            duration: format!("{:0>2}:{:0>2}", duration / 1000 / 60, duration / 1000 % 60),
                            song_url: String::new(),
                        });
                    });
                }
                Parse::RMD => {
                    let array = value.get("data").unwrap_or(&list).as_array().unwrap();
                    array.iter().for_each(|v| {
                        let duration = v.get("duration").unwrap_or(&json!(0)).as_u64().unwrap() as u32;
                        vec.push(SongInfo {
                            id: v.get("id").unwrap_or(&json!(0)).as_u64().unwrap() as u32,
                            name: v.get("name").unwrap_or(&json!("")).as_str().unwrap_or("").to_owned(),
                            singer: v
                                .get("artists")
                                .unwrap_or(&json!(&list))
                                .get(0)
                                .unwrap_or(&json!(null))
                                .get("name")
                                .unwrap_or(&json!(""))
                                .as_str()
                                .unwrap_or("")
                                .to_owned(),
                            album: v
                                .get("album")
                                .unwrap_or(&json!(null))
                                .get("name")
                                .unwrap_or(&json!(""))
                                .as_str()
                                .unwrap_or("")
                                .to_owned(),
                            pic_url: v
                                .get("album")
                                .unwrap_or(&json!(null))
                                .get("picUrl")
                                .unwrap_or(&json!(""))
                                .as_str()
                                .unwrap_or("")
                                .to_owned(),
                            duration: format!("{:0>2}:{:0>2}", duration / 1000 / 60, duration / 1000 % 60),
                            song_url: String::new(),
                        });
                    });
                }
                Parse::RMDS => {
                    let array = value
                        .get("data")
                        .unwrap_or(&json!(null))
                        .as_object()
                        .unwrap()
                        .get("dailySongs")
                        .unwrap_or(&list)
                        .as_array()
                        .unwrap();
                    array.iter().for_each(|v| {
                        let duration = v.get("duration").unwrap_or(&json!(0)).as_u64().unwrap() as u32;
                        vec.push(SongInfo {
                            id: v.get("id").unwrap_or(&json!(0)).as_u64().unwrap() as u32,
                            name: v.get("name").unwrap_or(&json!("")).as_str().unwrap_or("").to_owned(),
                            singer: v
                                .get("artists")
                                .unwrap_or(&json!(&list))
                                .get(0)
                                .unwrap_or(&json!(null))
                                .get("name")
                                .unwrap_or(&json!(""))
                                .as_str()
                                .unwrap_or("")
                                .to_owned(),
                            album: v
                                .get("album")
                                .unwrap_or(&json!(null))
                                .get("name")
                                .unwrap_or(&json!(""))
                                .as_str()
                                .unwrap_or("")
                                .to_owned(),
                            pic_url: v
                                .get("album")
                                .unwrap_or(&json!(null))
                                .get("picUrl")
                                .unwrap_or(&json!(""))
                                .as_str()
                                .unwrap_or("")
                                .to_owned(),
                            duration: format!("{:0>2}:{:0>2}", duration / 1000 / 60, duration / 1000 % 60),
                            song_url: String::new(),
                        });
                    });
                }
                Parse::SEARCH => {
                    let array = value
                        .get("result")
                        .unwrap_or(&json!(null))
                        .as_object()
                        .unwrap()
                        .get("songs")
                        .unwrap_or(&list)
                        .as_array()
                        .unwrap();
                    array.iter().for_each(|v| {
                        let duration = v.get("dt").unwrap_or(&json!(0)).as_u64().unwrap() as u32;
                        vec.push(SongInfo {
                            id: v.get("id").unwrap_or(&json!(0)).as_u64().unwrap() as u32,
                            name: v.get("name").unwrap_or(&json!("")).as_str().unwrap_or("").to_owned(),
                            singer: v
                                .get("ar")
                                .unwrap_or(&json!(&list))
                                .get(0)
                                .unwrap_or(&json!(null))
                                .get("name")
                                .unwrap_or(&json!(""))
                                .as_str()
                                .unwrap_or("")
                                .to_owned(),
                            album: v
                                .get("al")
                                .unwrap_or(&json!(null))
                                .get("name")
                                .unwrap_or(&json!(""))
                                .as_str()
                                .unwrap_or("")
                                .to_owned(),
                            pic_url: v
                                .get("al")
                                .unwrap_or(&json!(null))
                                .get("picUrl")
                                .unwrap_or(&json!(""))
                                .as_str()
                                .unwrap_or("")
                                .to_owned(),
                            duration: format!("{:0>2}:{:0>2}", duration / 1000 / 60, duration / 1000 % 60),
                            song_url: String::new(),
                        });
                    });
                }
                Parse::ALBUM => {
                    let array = value.get("songs").unwrap_or(&list).as_array().unwrap();
                    array.iter().for_each(|v| {
                        let duration = v.get("dt").unwrap_or(&json!(0)).as_u64().unwrap() as u32;
                        vec.push(SongInfo {
                            id: v.get("id").unwrap_or(&json!(0)).as_u64().unwrap() as u32,
                            name: v.get("name").unwrap_or(&json!("")).as_str().unwrap_or("").to_owned(),
                            singer: v
                                .get("ar")
                                .unwrap_or(&json!(&list))
                                .get(0)
                                .unwrap_or(&json!(null))
                                .get("name")
                                .unwrap_or(&json!(""))
                                .as_str()
                                .unwrap()
                                .to_owned(),
                            album: value
                                .get("album")
                                .unwrap_or(&json!(null))
                                .get("name")
                                .unwrap_or(&json!(""))
                                .as_str()
                                .unwrap_or("")
                                .to_owned(),
                            pic_url: value
                                .get("album")
                                .unwrap_or(&json!(null))
                                .get("picUrl")
                                .unwrap_or(&json!(""))
                                .as_str()
                                .unwrap_or("")
                                .to_owned(),
                            duration: format!("{:0>2}:{:0>2}", duration / 1000 / 60, duration / 1000 % 60),
                            song_url: String::new(),
                        });
                    });
                }
                _ => {}
            }
            if vec.is_empty() {
                return None;
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
    if let Ok(value) = serde_json::from_str::<Value>(&json) {
        if value.get("code").unwrap_or(&json!(0)).eq(&200) {
            let mut vec: Vec<SongList> = Vec::new();
            let list = json!([]);
            match parse {
                Parse::USL => {
                    let array = value.get("playlist").unwrap_or(&list).as_array().unwrap();
                    array.iter().for_each(|v| {
                        vec.push(SongList {
                            id: v.get("id").unwrap_or(&json!(0)).as_u64().unwrap() as u32,
                            name: v.get("name").unwrap_or(&json!("")).as_str().unwrap_or("").to_owned(),
                            cover_img_url: v
                                .get("coverImgUrl")
                                .unwrap_or(&json!(""))
                                .as_str()
                                .unwrap_or("")
                                .to_owned(),
                        });
                    });
                }
                Parse::RMD => {
                    let array = value.get("recommend").unwrap_or(&list).as_array().unwrap();
                    array.iter().for_each(|v| {
                        vec.push(SongList {
                            id: v.get("id").unwrap_or(&json!(0)).as_u64().unwrap() as u32,
                            name: v.get("name").unwrap_or(&json!("")).as_str().unwrap_or("").to_owned(),
                            cover_img_url: v.get("picUrl").unwrap_or(&json!("")).as_str().unwrap_or("").to_owned(),
                        });
                    });
                }
                Parse::ALBUM => {
                    let array = value.get("albums").unwrap_or(&list).as_array().unwrap();
                    array.iter().for_each(|v| {
                        vec.push(SongList {
                            id: v.get("id").unwrap_or(&json!(0)).as_u64().unwrap() as u32,
                            name: v.get("name").unwrap_or(&json!("")).as_str().unwrap_or("").to_owned(),
                            cover_img_url: v.get("picUrl").unwrap_or(&json!("")).as_str().unwrap_or("").to_owned(),
                        });
                    });
                }
                Parse::TOP => {
                    let array = value.get("playlists").unwrap_or(&list).as_array().unwrap();
                    array.iter().for_each(|v| {
                        vec.push(SongList {
                            id: v.get("id").unwrap_or(&json!(0)).as_u64().unwrap() as u32,
                            name: v.get("name").unwrap_or(&json!("")).as_str().unwrap_or("").to_owned(),
                            cover_img_url: v
                                .get("coverImgUrl")
                                .unwrap_or(&json!(""))
                                .as_str()
                                .unwrap_or("")
                                .to_owned(),
                        });
                    });
                }
                _ => {}
            }
            if vec.is_empty() {
                return None;
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
    if let Ok(value) = serde_json::from_str::<Value>(&json) {
        let code = value.get("code").unwrap_or(&json!(0)).as_i64().unwrap() as i32;
        if code.eq(&200) {
            Some(Msg {
                code: 200,
                msg: "".to_owned(),
            })
        } else {
            let msg = value.get("msg").unwrap_or(&json!("")).as_str().unwrap_or("").to_owned();
            Some(Msg { code, msg })
        }
    } else {
        None
    }
}

// 登陆信息
#[derive(Debug, Clone, Deserialize, Serialize)]
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
    if let Ok(value) = serde_json::from_str::<Value>(&json) {
        let code = value.get("code").unwrap_or(&json!(0)).as_i64().unwrap() as i32;
        if code.eq(&200) {
            let profile = value.get("profile").unwrap_or(&json!(null)).as_object().unwrap();
            Some(LoginInfo {
                code,
                uid: profile.get("userId").unwrap_or(&json!(0)).as_u64().unwrap() as u32,
                nickname: profile
                    .get("nickname")
                    .unwrap_or(&json!(""))
                    .as_str()
                    .unwrap_or("")
                    .to_owned(),
                avatar_url: profile
                    .get("avatarUrl")
                    .unwrap_or(&json!(""))
                    .as_str()
                    .unwrap_or("")
                    .to_owned(),
                msg: "".to_owned(),
            })
        } else {
            let msg = value.get("msg").unwrap_or(&json!("")).as_str().unwrap_or("").to_owned();
            Some(LoginInfo {
                code,
                uid: 0,
                nickname: "".to_owned(),
                avatar_url: "".to_owned(),
                msg,
            })
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
