//
// data.rs
// Copyright (C) 2019 gmg137 <gmg137@live.com>
// Distributed under terms of the GPLv3 license.
//

use crate::musicapi::{model::*, MusicApi};
use crate::{CACHED_PATH, CONFIG_PATH, DATE_DAY, ISO_WEEK};
use serde::{Deserialize, Serialize};
use sled::{Config, Db};
use std::path::Path;
use std::{fs, io};

// 数据库字段说明
// login_info: 用户登陆信息
// user_song_list: 用户歌单
// song_list_*: 歌单歌曲
// recommend_resource: 每日推荐歌单
// recommend_songs: 每日推荐歌曲
// top_song_list: 热门推荐歌单

// 状态数据
// login: 是否已登录
// day: 数据更新日期, 1-31, 每天只更新一次
// week: 每年中的第几周，每周清除一次缓存
#[derive(Debug, Deserialize, Serialize)]
struct StatusData {
    login: bool,
    day: u32,
    week: u32,
}

// 音乐数据本地缓存
// login: 是否已经登录
pub(crate) struct MusicData {
    musicapi: MusicApi,
    db: Option<Db>,
    pub(crate) login: bool,
}

impl MusicData {
    #[allow(unused)]
    pub(crate) fn new() -> Self {
        // println!("new MusicData!");
        // 加载数据文件
        if let Ok(db) = Config::default()
            .path(format!("{}/db", CONFIG_PATH.to_owned()))
            .cache_capacity(1024 * 1024 * 10)
            .open() {
            // 查询状态数据
            if let Some(status_data) = db.get(b"status_data").unwrap_or(None) {
                if let Ok(status_data) = serde_json::from_slice::<StatusData>(&status_data) {
                    // 每周清理缓存
                    if status_data.week != *ISO_WEEK {
                        clear_cache(Path::new(*CACHED_PATH));
                    }
                    // 对比缓存是否过期
                    if status_data.day == *DATE_DAY {
                        // 直接返回缓存数据
                        return MusicData {
                            musicapi: MusicApi::new(),
                            db: Some(db),
                            login: status_data.login,
                        };
                    } else {
                        db.clear();
                        // 重新查询登陆状态
                        let mut musicapi = MusicApi::new();
                        if let Some(login_info) = musicapi.login_status() {
                            let data = StatusData {
                                login: true,
                                day: *DATE_DAY,
                                week: *ISO_WEEK,
                            };
                            db.insert(b"status_data", serde_json::to_vec(&data).unwrap_or(vec![]));
                            db.insert(b"login_info", serde_json::to_vec(&login_info).unwrap_or(vec![]));
                            db.flush();
                            return MusicData {
                                musicapi,
                                db: Some(db),
                                login: true,
                            };
                        }
                        let data = StatusData {
                            login: false,
                            day: *DATE_DAY,
                            week: *ISO_WEEK,
                        };
                        db.insert(b"status_data", serde_json::to_vec(&data).unwrap_or(vec![]));
                        db.flush();
                        return MusicData {
                            musicapi,
                            db: Some(db),
                            login: false,
                        };
                    }
                }
            } else {
                db.clear();
                let mut musicapi = MusicApi::new();
                if let Some(login_info) = musicapi.login_status() {
                    let data = StatusData {
                        login: true,
                        day: *DATE_DAY,
                        week: *ISO_WEEK,
                    };
                    db.insert(b"status_data", serde_json::to_vec(&data).unwrap_or(vec![]));
                    db.insert(b"login_info", serde_json::to_vec(&login_info).unwrap_or(vec![]));
                    db.flush();
                    return MusicData {
                        musicapi,
                        db: Some(db),
                        login: true,
                    };
                }
                let data = StatusData {
                    login: false,
                    day: *DATE_DAY,
                    week: *ISO_WEEK,
                };
                db.insert(b"status_data", serde_json::to_vec(&data).unwrap_or(vec![]));
                db.flush();
                return MusicData {
                    musicapi,
                    db: Some(db),
                    login: false,
                };
            }
        }
        MusicData {
            musicapi: MusicApi::new(),
            db: None,
            login: false,
        }
    }

    // 登录
    #[allow(unused)]
    pub(crate) fn login(&mut self, username: String, password: String) -> Option<LoginInfo> {
        if let Some(login_info) = self.musicapi.login(username, password) {
            if login_info.code.eq(&200) {
                self.login = true;
                if let Some(db) = &self.db {
                    let data = StatusData {
                        login: true,
                        day: *DATE_DAY,
                        week: *ISO_WEEK,
                    };
                    db.insert(b"status_data", serde_json::to_vec(&data).unwrap_or(vec![]));
                    db.insert(b"login_info", serde_json::to_vec(&login_info).unwrap_or(vec![]));
                    db.flush();
                    return Some(login_info);
                }
            }
        }
        None
    }

    // 获取登陆信息
    #[allow(unused)]
    pub(crate) fn login_info(&mut self) -> Option<LoginInfo> {
        if self.login {
            if let Some(db) = &self.db {
                if let Some(login_info_vec) = db.get(b"login_info").unwrap_or(None) {
                    if let Ok(login_info) = serde_json::from_slice::<LoginInfo>(&login_info_vec) {
                        return Some(login_info);
                    }
                }
            }
        }
        None
    }

    // 退出
    #[allow(unused)]
    pub(crate) fn logout(&mut self) {
        if self.login {
            if let Some(db) = &self.db {
                let data = StatusData {
                    login: false,
                    day: *DATE_DAY,
                    week: *ISO_WEEK,
                };
                db.insert(b"status_data", serde_json::to_vec(&data).unwrap_or(vec![]));
                db.remove(b"user_song_list");
                db.remove(b"recommend_resource");
                db.remove(b"login_info");
                db.flush();
            }
        }
        self.login = false;
        let cookie_path = format!("{}/cookie", CONFIG_PATH.to_owned());
        std::fs::write(&cookie_path, "").unwrap_or(());
    }

    // 每日签到
    #[allow(unused)]
    pub(crate) fn daily_task(&mut self) -> Option<Msg> {
        self.musicapi.daily_task()
    }

    // 用户歌单
    // uid: 用户id
    // offset: 列表起点号
    // limit: 列表长度
    #[allow(unused)]
    pub(crate) fn user_song_list(&mut self, uid: u32, offset: u8, limit: u8) -> Option<Vec<SongList>> {
        if self.login {
            if let Some(db) = &self.db {
                // 查询缓存
                if let Some(user_song_list_vec) = db.get(b"user_song_list").unwrap_or(None) {
                    if let Ok(user_song_list) = serde_json::from_slice::<Vec<SongList>>(&user_song_list_vec) {
                        return Some(user_song_list);
                    }
                }
                // 查询 api
                if let Some(usl) = self.musicapi.user_song_list(uid, offset, limit) {
                    if !usl.is_empty() {
                        db.insert(b"user_song_list", serde_json::to_vec(&usl).unwrap_or(vec![]));
                        db.flush();
                    }
                    return Some(usl);
                }
            }
        }
        None
    }

    // 歌单详情
    // songlist_id: 歌单 id
    #[allow(unused)]
    pub(crate) fn song_list_detail(&mut self, songlist_id: u32, refresh: bool) -> Option<Vec<SongInfo>> {
        if let Some(db) = &self.db {
            let key = format!("song_list_{}", songlist_id);
            if !refresh {
                if let Some(song_list_detail_vec) = db.get(key.as_bytes()).unwrap_or(None) {
                    if let Ok(song_list_detail) = serde_json::from_slice::<Vec<SongInfo>>(&song_list_detail_vec) {
                        return Some(song_list_detail);
                    }
                }
            }
            if let Some(sld) = self.musicapi.song_list_detail(songlist_id) {
                if !sld.is_empty() {
                    db.insert(key.as_bytes(), serde_json::to_vec(&sld).unwrap_or(vec![]));
                }
                db.flush();
                return Some(sld);
            }
        }
        None
    }

    // 歌曲详情
    // ids: 歌曲 id 列表
    #[allow(unused)]
    pub(crate) fn songs_detail(&mut self, ids: &[u32]) -> Option<Vec<SongInfo>> {
        self.musicapi.songs_detail(ids)
    }

    // 歌曲 URL
    // ids: 歌曲列表
    // rate: 320: 320K,
    //       192: 192k
    //       128: 128k
    #[allow(unused)]
    pub(crate) fn songs_url(&mut self, ids: &[u32], rate: u32) -> Option<Vec<SongUrl>> {
        self.musicapi.songs_url(ids, rate)
    }

    // 每日推荐歌单
    #[allow(unused)]
    pub(crate) fn recommend_resource(&mut self) -> Option<Vec<SongList>> {
        if self.login {
            if let Some(db) = &self.db {
                if let Some(recommend_resource_vec) = db.get(b"recommend_resource").unwrap_or(None) {
                    if let Ok(song_list) = serde_json::from_slice::<Vec<SongList>>(&recommend_resource_vec) {
                        return Some(song_list);
                    }
                }
                if let Some(rr) = self.musicapi.recommend_resource() {
                    if !rr.is_empty() {
                        db.insert(b"recommend_resource", serde_json::to_vec(&rr).unwrap_or(vec![]));
                    }
                    db.flush();
                    return Some(rr);
                }
            }
        }
        None
    }

    // 每日推荐歌曲
    #[allow(unused)]
    pub(crate) fn recommend_songs(&mut self, refresh: bool) -> Option<Vec<SongInfo>> {
        if self.login {
            if let Some(db) = &self.db {
                if !refresh {
                    if let Some(recommend_songs_vec) = db.get(b"recommend_songs").unwrap_or(None) {
                        if let Ok(songs) = serde_json::from_slice::<Vec<SongInfo>>(&recommend_songs_vec) {
                            return Some(songs);
                        }
                    }
                }
                if let Some(rs) = self.musicapi.recommend_songs() {
                    if !rs.is_empty() {
                        db.insert(b"recommend_songs", serde_json::to_vec(&rs).unwrap_or(vec![]));
                    }
                    db.flush();
                    return Some(rs);
                }
            }
        }
        None
    }

    // 私人FM
    #[allow(unused)]
    pub(crate) fn personal_fm(&mut self) -> Option<Vec<SongInfo>> {
        self.musicapi.personal_fm()
    }

    // 收藏/喜欢
    // songid: 歌曲id
    // like: true 收藏，false 取消
    #[allow(unused)]
    pub(crate) fn like(&mut self, like: bool, songid: u32) -> bool {
        if self.musicapi.like(like, songid) {
            if let Some(login_info) = self.login_info() {
                if let Some(usl) = &self.user_song_list(login_info.uid, 0, 50) {
                    let row_id = 0; // 假定喜欢的音乐歌单总排在第一位
                    self.song_list_detail(usl[row_id].id, true);
                    return true;
                }
            }
        }
        false
    }

    // FM 不喜欢
    // songid: 歌曲id
    #[allow(unused)]
    pub(crate) fn fm_trash(&mut self, songid: u32) -> bool {
        self.musicapi.fm_trash(songid)
    }

    // 搜索
    // keywords: 关键词
    // types: 单曲(1)，歌手(100)，专辑(10)，歌单(1000)，用户(1002) *(type)*
    // offset: 起始点
    // limit: 数量
    #[allow(unused)]
    pub(crate) fn search(&mut self, keywords: String, types: u32, offset: u16, limit: u16) -> Option<String> {
        self.musicapi.search(keywords, types, offset, limit)
    }

    // 新碟上架
    // offset: 起始点
    // limit: 数量
    #[allow(unused)]
    pub(crate) fn new_albums(&mut self, offset: u8, limit: u8) -> Option<Vec<SongList>> {
        if let Some(db) = &self.db {
            if let Some(new_albums_vec) = db.get(b"new_albums").unwrap_or(None) {
                if let Ok(song_list) = serde_json::from_slice::<Vec<SongList>>(&new_albums_vec) {
                    return Some(song_list);
                }
            }
            if let Some(na) = self.musicapi.new_albums(offset, limit) {
                if !na.is_empty() {
                    db.insert(b"new_albums", serde_json::to_vec(&na).unwrap_or(vec![]));
                }
                db.flush();
                return Some(na);
            }
        }
        None
    }

    // 专辑
    // album_id: 专辑 id
    #[allow(unused)]
    pub(crate) fn album(&mut self, album_id: u32) -> Option<Vec<SongInfo>> {
        if let Some(db) = &self.db {
            if let Some(album_vec) = db.get(b"album").unwrap_or(None) {
                if let Ok(album) = serde_json::from_slice::<Vec<SongInfo>>(&album_vec) {
                    return Some(album);
                }
            }
            if let Some(a) = self.musicapi.album(album_id) {
                if !a.is_empty() {
                    db.insert(b"album", serde_json::to_vec(&a).unwrap_or(vec![]));
                }
                db.flush();
                return Some(a);
            }
        }
        None
    }

    // 热门推荐歌单
    // offset: 起始点
    // limit: 数量
    // order: 排序方式:
    //	      "hot": 热门，
    //        "new": 最新
    #[allow(unused)]
    pub(crate) fn top_song_list(&mut self, order: &str, offset: u8, limit: u8) -> Option<Vec<SongList>> {
        if let Some(db) = &self.db {
            if let Some(top_song_list_vec) = db.get(b"top_song_list").unwrap_or(None) {
                if let Ok(top_song_list) = serde_json::from_slice::<Vec<SongList>>(&top_song_list_vec) {
                    return Some(top_song_list);
                }
            }
            if let Some(tsl) = self.musicapi.top_song_list(order, offset, limit) {
                if !tsl.is_empty() {
                    db.insert(b"top_song_list", serde_json::to_vec(&tsl).unwrap_or(vec![]));
                }
                db.flush();
                return Some(tsl);
            }
        }
        None
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
    pub(crate) fn top_songs(&mut self, list_id: u32) -> Option<Vec<SongInfo>> {
        self.song_list_detail(list_id, true)
    }

    // 查询歌词
    // music_id: 歌曲id
    #[allow(unused)]
    pub(crate) fn song_lyric(&mut self, music_id: u32) -> Option<Vec<String>> {
        if let Some(db) = &self.db {
            let key = format!("song_lyric_{}", music_id);
            if let Some(song_lyric_vec) = db.get(key.as_bytes()).unwrap_or(None) {
                if let Ok(song_lyric) = serde_json::from_slice::<Vec<String>>(&song_lyric_vec) {
                    return Some(song_lyric);
                }
            }
            if let Some(sl) = self.musicapi.song_lyric(music_id) {
                if !sl.is_empty() {
                    db.insert(key.as_bytes(), serde_json::to_vec(&sl).unwrap_or(vec![]));
                }
                db.flush();
                return Some(sl);
            }
        }
        None
    }

    // 删除指定键
    #[allow(unused)]
    pub(crate) fn del<K: AsRef<[u8]>>(&self, key: K) {
        if let Some(db) = &self.db {
            db.remove(key).unwrap_or(None);
            db.flush();
        }
    }

    // 收藏/取消收藏歌单
    // like: true 收藏，false 取消
    // id: 歌单 id
    #[allow(unused)]
    pub(crate) fn song_list_like(&mut self, like: bool, id: u32) -> bool {
        self.musicapi.song_list_like(like, id)
    }
}

// 删除上周缓存的图片文件
fn clear_cache(dir: &Path) -> io::Result<()> {
    if dir.is_dir() {
        for file in fs::read_dir(dir)? {
            let file = file?;
            let path = file.path();
            fs::remove_file(&path)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_db() {
        //let mut data = MusicData::new();
        //dbg!(data.login("".to_owned(), "".to_owned()));
        //data.logout();
        //dbg!(data.login_info());
        //dbg!(data.user_song_list(2740524, 0, 100));
        //dbg!(data.song_list_detail(3447396));
        //dbg!(data.songs_detail(&[334916]));
        //dbg!(data.songs_url(&[2081057], 320));
        //dbg!(data.recommend_resource());
        //dbg!(data.recommend_songs());
        //dbg!(data.like(false, 566442496));
        //dbg!(data.song_lyric(566442496));
        //dbg!(data.top_songs(19723756));
        //dbg!(data.top_song_list("new", 0, 3));
        //dbg!(data.new_albums(0, 5));
        //dbg!(data.album(75889022));
        assert!(true);
    }
}
