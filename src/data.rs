//
// data.rs
// Copyright (C) 2019 gmg137 <gmg137@live.com>
// Distributed under terms of the GPLv3 license.
//

use crate::musicapi::{model::*, MusicApi};
use crate::{CONFIG_PATH, DATE_DAY};
use byteorder::{BigEndian, ReadBytesExt};
use sled::Db;
use std::io::Cursor;

// 数据库字段说明
// login: 登陆状态
//      0 未登陆
//      1 已登陆
// date: 数据更新日期, 1-31, 每天只更新一次
// user_song_list: 用户歌单
// song_list_*: 歌单歌曲
// recommend_resource: 每日推荐歌单
// recommend_songs: 每日推荐歌曲
// top_song_list: 热门推荐歌单

// 音乐数据本地缓存
// login: 是否已经登陆
// update: 数据是否需要更新
pub(crate) struct MusicData {
    musicapi: MusicApi,
    db: Option<Db>,
    pub(crate) login: bool,
    pub(crate) update: bool,
}

impl MusicData {
    #[allow(unused)]
    pub(crate) fn new() -> Self {
        // 加载数据文件
        if let Ok(db) = Db::start_default(format!("{}/db", CONFIG_PATH.to_owned())) {
            // 查询数据更新日期
            if let Some(date) = db.get(b"date").unwrap_or(None) {
                let mut v = Cursor::new(&date[..]);
                // 对比缓存是否过期
                if v.read_u32::<BigEndian>().unwrap_or(0) == *DATE_DAY {
                    // 直接返回缓存数据
                    if let Some(login) = db.get(b"login").unwrap_or(None) {
                        if let Some(update) = db.get(b"update").unwrap_or(None) {
                            return MusicData {
                                musicapi: MusicApi::new(),
                                db: Some(db),
                                login: login == [1],
                                update: update == [1],
                            };
                        }
                        return MusicData {
                            musicapi: MusicApi::new(),
                            db: Some(db),
                            login: login == [1],
                            update: true,
                        };
                    }
                } else {
                    db.clear();
                    // 重新查询登陆状态
                    let mut musicapi = MusicApi::new();
                    if let Some(login_info) = musicapi.login_status() {
                        db.set(b"login", vec![1]);
                        db.set(b"date", DATE_DAY.to_be_bytes().to_vec());
                        db.set(
                            b"login_info",
                            serde_json::to_vec(&login_info).unwrap_or(vec![]),
                        );
                        db.flush();
                        return MusicData {
                            musicapi,
                            db: Some(db),
                            login: true,
                            update: true,
                        };
                    }
                    db.set(b"login", vec![0]);
                    db.set(b"date", DATE_DAY.to_be_bytes().to_vec());
                    db.flush();
                    return MusicData {
                        musicapi,
                        db: Some(db),
                        login: false,
                        update: true,
                    };
                }
            } else {
                db.clear();
                let mut musicapi = MusicApi::new();
                if let Some(login_info) = musicapi.login_status() {
                    db.set(b"login", vec![1]);
                    db.set(b"date", DATE_DAY.to_be_bytes().to_vec());
                    db.set(
                        b"login_info",
                        serde_json::to_vec(&login_info).unwrap_or(vec![]),
                    );
                    db.flush();
                    return MusicData {
                        musicapi,
                        db: Some(db),
                        login: true,
                        update: true,
                    };
                }
                db.set(b"login", vec![0]);
                db.set(b"date", DATE_DAY.to_be_bytes().to_vec());
                db.flush();
                return MusicData {
                    musicapi,
                    db: Some(db),
                    login: false,
                    update: true,
                };
            }
        }
        MusicData {
            musicapi: MusicApi::new(),
            db: None,
            login: false,
            update: false,
        }
    }

    // 登陆
    #[allow(unused)]
    pub(crate) fn login(&mut self, username: String, password: String) -> Option<LoginInfo> {
        if let Some(login_info) = self.musicapi.login(username, password) {
            if login_info.code.eq(&200) {
                self.login = true;
                if let Some(db) = &self.db {
                    db.set(b"login", vec![1]);
                    db.set(b"date", DATE_DAY.to_be_bytes().to_vec());
                    db.set(
                        b"login_info",
                        serde_json::to_vec(&login_info).unwrap_or(vec![]),
                    );
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
                db.set(b"login", vec![0]);
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
    pub(crate) fn user_song_list(
        &mut self,
        uid: u32,
        offset: u8,
        limit: u8,
    ) -> Option<Vec<SongList>> {
        if self.login {
            if let Some(db) = &self.db {
                // 查询缓存
                if let Some(user_song_list_vec) = db.get(b"user_song_list").unwrap_or(None) {
                    if let Ok(user_song_list) =
                        serde_json::from_slice::<Vec<SongList>>(&user_song_list_vec)
                    {
                        return Some(user_song_list);
                    }
                }
                // 查询 api
                if let Some(usl) = self.musicapi.user_song_list(uid, offset, limit) {
                    db.set(
                        b"user_song_list",
                        serde_json::to_vec(&usl).unwrap_or(vec![]),
                    );
                    db.flush();
                    return Some(usl);
                }
            }
        }
        None
    }

    // 歌单详情
    // songlist_id: 歌单 id
    #[allow(unused)]
    pub(crate) fn song_list_detail(&mut self, songlist_id: u32) -> Option<Vec<SongInfo>> {
        if let Some(db) = &self.db {
            let key = format!("song_list_{}", songlist_id);
            // 有更新且要查询的歌单为我喜欢的歌曲时才更新数据
            if !self.update || songlist_id != 3447396 {
                if let Some(song_list_detail_vec) = db.get(key.as_bytes()).unwrap_or(None) {
                    if let Ok(song_list_detail) =
                        serde_json::from_slice::<Vec<SongInfo>>(&song_list_detail_vec)
                    {
                        return Some(song_list_detail);
                    }
                }
            }
            if let Some(sld) = self.musicapi.song_list_detail(songlist_id) {
                db.set(key.as_bytes(), serde_json::to_vec(&sld).unwrap_or(vec![]));
                if songlist_id == 3447396 {
                    db.set(b"update", vec![0]);
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
                if let Some(recommend_resource_vec) = db.get(b"recommend_resource").unwrap_or(None)
                {
                    if let Ok(song_list) =
                        serde_json::from_slice::<Vec<SongList>>(&recommend_resource_vec)
                    {
                        return Some(song_list);
                    }
                }
                if let Some(rr) = self.musicapi.recommend_resource() {
                    db.set(
                        b"recommend_resource",
                        serde_json::to_vec(&rr).unwrap_or(vec![]),
                    );
                    db.flush();
                    return Some(rr);
                }
            }
        }
        None
    }

    // 每日推荐歌曲
    #[allow(unused)]
    pub(crate) fn recommend_songs(&mut self) -> Option<Vec<SongInfo>> {
        if self.login {
            if let Some(db) = &self.db {
                if let Some(recommend_songs_vec) = db.get(b"recommend_songs").unwrap_or(None) {
                    if let Ok(songs) = serde_json::from_slice::<Vec<SongInfo>>(&recommend_songs_vec)
                    {
                        return Some(songs);
                    }
                }
                if let Some(rs) = self.musicapi.recommend_songs() {
                    db.set(
                        b"recommend_songs",
                        serde_json::to_vec(&rs).unwrap_or(vec![]),
                    );
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
            if let Some(db) = &self.db {
                db.set(b"update", vec![1]);
                db.flush();
            }
            self.update = true;
            return true;
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
    pub(crate) fn search(
        &mut self,
        keywords: String,
        types: u32,
        offset: u16,
        limit: u16,
    ) -> Option<String> {
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
                db.set(b"new_albums", serde_json::to_vec(&na).unwrap_or(vec![]));
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
                db.set(b"album", serde_json::to_vec(&a).unwrap_or(vec![]));
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
    pub(crate) fn top_song_list(
        &mut self,
        order: &str,
        offset: u8,
        limit: u8,
    ) -> Option<Vec<SongList>> {
        if let Some(db) = &self.db {
            if let Some(top_song_list_vec) = db.get(b"top_song_list").unwrap_or(None) {
                if let Ok(top_song_list) =
                    serde_json::from_slice::<Vec<SongList>>(&top_song_list_vec)
                {
                    return Some(top_song_list);
                }
            }
            if let Some(tsl) = self.musicapi.top_song_list(order, offset, limit) {
                db.set(b"top_song_list", serde_json::to_vec(&tsl).unwrap_or(vec![]));
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
        self.song_list_detail(list_id)
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
                db.set(key.as_bytes(), serde_json::to_vec(&sl).unwrap_or(vec![]));
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
            db.del(key).unwrap_or(None);
            db.flush();
        }
    }
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
