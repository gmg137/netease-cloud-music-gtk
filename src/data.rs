//
// data.rs
// Copyright (C) 2019 gmg137 <gmg137@live.com>
// Distributed under terms of the GPLv3 license.
//

use crate::{
    model::{Errors, NCMResult, DATE_DAY, NCM_CONFIG, NCM_DATA},
    musicapi::{model::*, MusicApi},
};
use async_std::{fs, prelude::*};
use openssl::hash::{hash, MessageDigest};
use serde::{Deserialize, Serialize};
use std::path::Path;

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
#[derive(Debug, Deserialize, Serialize)]
struct StatusData {
    login: bool,
    day: u32,
}

// 音乐数据本地缓存
// login: 是否已经登录
pub(crate) struct MusicData {
    musicapi: MusicApi,
    pub(crate) login: bool,
}

#[derive(Debug, Deserialize, Serialize)]
struct LoginKey {
    username: String,
    password: String,
}

impl MusicData {
    #[allow(unused)]
    pub(crate) async fn new() -> NCMResult<Self> {
        if let Ok(buffer) = fs::read(format!("{}status_data.db", NCM_CONFIG.to_string_lossy())).await {
            let mut status_data: StatusData = bincode::deserialize(&buffer).map_err(|_| Errors::NoneError)?;
            // 对比缓存数据是否过期
            if status_data.day != *DATE_DAY {
                // 清理缓存数据
                clear_cache(&NCM_DATA).await;
                // 更新日期
                status_data.day = *DATE_DAY;
                fs::write(
                    format!("{}status_data.db", NCM_CONFIG.to_string_lossy()),
                    bincode::serialize(&status_data).map_err(|_| Errors::NoneError)?,
                )
                .await?;
            }
            return Ok(MusicData {
                musicapi: MusicApi::new()?,
                login: status_data.login,
            });
        }
        let data = StatusData {
            login: false,
            day: *DATE_DAY,
        };
        fs::write(
            format!("{}status_data.db", NCM_CONFIG.to_string_lossy()),
            bincode::serialize(&data).map_err(|_| Errors::NoneError)?,
        )
        .await?;
        Ok(MusicData {
            musicapi: MusicApi::new()?,
            login: false,
        })
    }

    // 登录
    #[allow(unused)]
    pub(crate) async fn login(&mut self, username: String, password: String) -> NCMResult<LoginInfo> {
        let password = hex::encode(hash(MessageDigest::md5(), &password.as_bytes())?);
        let login_info = self.musicapi.login(username.to_owned(), password.to_owned())?;
        if login_info.code.eq(&200) {
            self.login = true;
            let data = StatusData {
                login: true,
                day: *DATE_DAY,
            };
            fs::write(
                format!("{}login_key.db", NCM_CONFIG.to_string_lossy()),
                bincode::serialize(&LoginKey { username, password }).map_err(|_| Errors::NoneError)?,
            )
            .await?;
            fs::write(
                format!("{}status_data.db", NCM_CONFIG.to_string_lossy()),
                bincode::serialize(&data).map_err(|_| Errors::NoneError)?,
            )
            .await?;
            fs::write(
                format!("{}login_info.db", NCM_DATA.to_string_lossy()),
                bincode::serialize(&login_info).map_err(|_| Errors::NoneError)?,
            )
            .await?;
            return Ok(login_info);
        }
        Ok(login_info)
    }

    // 重新登录
    #[allow(unused)]
    async fn re_login(&mut self) -> NCMResult<LoginInfo> {
        if let Ok(buffer) = fs::read(format!("{}login_key.db", NCM_CONFIG.to_string_lossy())).await {
            let LoginKey { username, password } = bincode::deserialize(&buffer).map_err(|_| Errors::NoneError)?;
            let login_info = self.musicapi.login(username, password)?;
            if login_info.code.eq(&200) {
                self.login = true;
                let data = StatusData {
                    login: true,
                    day: *DATE_DAY,
                };
                fs::write(
                    format!("{}status_data.db", NCM_CONFIG.to_string_lossy()),
                    bincode::serialize(&data).map_err(|_| Errors::NoneError)?,
                )
                .await?;
                fs::write(
                    format!("{}login_info.db", NCM_DATA.to_string_lossy()),
                    bincode::serialize(&login_info).map_err(|_| Errors::NoneError)?,
                )
                .await?;
                return Ok(login_info);
            }
        }
        Err(Errors::NoneError)
    }

    // 获取登陆信息
    #[allow(unused)]
    pub(crate) async fn login_info(&mut self) -> NCMResult<LoginInfo> {
        if self.login {
            if let Ok(buffer) = fs::read(format!("{}login_info.db", NCM_DATA.to_string_lossy())).await {
                let login_info: LoginInfo = bincode::deserialize(&buffer).map_err(|_| Errors::NoneError)?;
                return Ok(login_info);
            }
            if let Ok(login_info) = self.re_login().await {
                return Ok(login_info);
            }
            self.login = false;
            let data = StatusData {
                login: false,
                day: *DATE_DAY,
            };
            fs::write(
                format!("{}status_data.db", NCM_CONFIG.to_string_lossy()),
                bincode::serialize(&data).map_err(|_| Errors::NoneError)?,
            )
            .await?;
        }
        Err(Errors::NoneError)
    }

    // 退出
    #[allow(unused)]
    pub(crate) async fn logout(&mut self) -> NCMResult<()> {
        if self.login {
            let data = StatusData {
                login: false,
                day: *DATE_DAY,
            };
            fs::write(
                format!("{}status_data.db", NCM_CONFIG.to_string_lossy()),
                bincode::serialize(&data).map_err(|_| Errors::NoneError)?,
            )
            .await?;
            fs::remove_file(format!("{}user_song_list.db", NCM_DATA.to_string_lossy())).await?;
            fs::remove_file(format!("{}recommend_resource.db", NCM_DATA.to_string_lossy())).await?;
            fs::remove_file(format!("{}recommend_songs.db", NCM_DATA.to_string_lossy())).await?;
            fs::remove_file(format!("{}login_info.db", NCM_DATA.to_string_lossy())).await?;
        }
        self.login = false;
        let cookie_path = format!("{}cookie", NCM_CONFIG.to_string_lossy());
        fs::write(&cookie_path, "").await?;
        Ok(())
    }

    // 每日签到
    #[allow(unused)]
    pub(crate) async fn daily_task(&mut self) -> NCMResult<Msg> {
        self.musicapi.daily_task()
    }

    // 用户歌单
    // uid: 用户id
    // offset: 列表起点号
    // limit: 列表长度
    #[allow(unused)]
    pub(crate) async fn user_song_list(&mut self, uid: u64, offset: u8, limit: u8) -> NCMResult<Vec<SongList>> {
        if self.login {
            let path = format!("{}user_song_list.db", NCM_DATA.to_string_lossy());
            // 查询缓存
            if let Ok(buffer) = fs::read(&path).await {
                if let Ok(login_info) = bincode::deserialize::<Vec<SongList>>(&buffer) {
                    return Ok(login_info);
                }
            }
            // 查询 api
            let usl = self.musicapi.user_song_list(uid, offset, limit)?;
            if !usl.is_empty() {
                fs::write(path, bincode::serialize(&usl).map_err(|_| Errors::NoneError)?).await?;
                return Ok(usl);
            }
        }
        Err(Errors::NoneError)
    }

    // 歌单详情
    // songlist_id: 歌单 id
    #[allow(unused)]
    pub(crate) async fn song_list_detail(&mut self, songlist_id: u64, refresh: bool) -> NCMResult<Vec<SongInfo>> {
        let path = format!("{}song_list_{}.db", NCM_DATA.to_string_lossy(), songlist_id);
        if !refresh {
            if let Ok(buffer) = fs::read(&path).await {
                if let Ok(song_list_detail) = bincode::deserialize::<Vec<SongInfo>>(&buffer) {
                    return Ok(song_list_detail);
                }
            }
        }
        let sld = self.musicapi.song_list_detail(songlist_id)?;
        if !sld.is_empty() {
            fs::write(path, bincode::serialize(&sld).map_err(|_| Errors::NoneError)?).await?;
            return Ok(sld);
        }
        Err(Errors::NoneError)
    }

    // 歌曲详情
    // ids: 歌曲 id 列表
    #[allow(unused)]
    pub(crate) async fn songs_detail(&mut self, ids: &[u64]) -> NCMResult<Vec<SongInfo>> {
        self.musicapi.songs_detail(ids)
    }

    // 歌曲 URL
    // ids: 歌曲列表
    // rate: 320: 320K,
    //       192: 192k
    //       128: 128k
    #[allow(unused)]
    pub(crate) async fn songs_url(&mut self, ids: &[u64], rate: u32) -> NCMResult<Vec<SongUrl>> {
        self.musicapi.songs_url(ids, rate)
    }

    // 每日推荐歌单
    #[allow(unused)]
    pub(crate) async fn recommend_resource(&mut self) -> NCMResult<Vec<SongList>> {
        if self.login {
            let path = format!("{}recommend_resource.db", NCM_DATA.to_string_lossy());
            if let Ok(buffer) = fs::read(&path).await {
                if let Ok(song_list) = bincode::deserialize::<Vec<SongList>>(&buffer) {
                    return Ok(song_list);
                }
            }
            if let Ok(rr) = self.musicapi.recommend_resource() {
                if !rr.is_empty() {
                    fs::write(path, bincode::serialize(&rr).map_err(|_| Errors::NoneError)?).await?;
                    return Ok(rr);
                }
            }
        }
        Err(Errors::NoneError)
    }

    // 每日推荐歌曲
    #[allow(unused)]
    pub(crate) async fn recommend_songs(&mut self) -> NCMResult<Vec<SongInfo>> {
        if self.login {
            let path = format!("{}recommend_songs.db", NCM_DATA.to_string_lossy());
            if let Ok(buffer) = fs::read(&path).await {
                if let Ok(songs) = bincode::deserialize::<Vec<SongInfo>>(&buffer) {
                    return Ok(songs);
                }
            }
            if let Ok(rs) = self.musicapi.recommend_songs() {
                if !rs.is_empty() {
                    fs::write(path, bincode::serialize(&rs).map_err(|_| Errors::NoneError)?).await?;
                    return Ok(rs);
                }
            }
        }
        Err(Errors::NoneError)
    }

    // 音乐云盘
    #[allow(unused)]
    pub(crate) async fn cloud_disk(&mut self, refresh: bool) -> NCMResult<Vec<SongInfo>> {
        if self.login {
            let path = format!("{}cloud_disk.db", NCM_DATA.to_string_lossy());
            if !refresh {
                if let Ok(buffer) = fs::read(&path).await {
                    if let Ok(songs) = bincode::deserialize::<Vec<SongInfo>>(&buffer) {
                        return Ok(songs);
                    }
                }
            }
            if let Ok(rs) = self.musicapi.user_cloud_disk() {
                if !rs.is_empty() {
                    fs::write(path, bincode::serialize(&rs).map_err(|_| Errors::NoneError)?).await?;
                    return Ok(rs);
                }
            }
        }
        Err(Errors::NoneError)
    }

    // 私人FM
    #[allow(unused)]
    pub(crate) async fn personal_fm(&mut self) -> NCMResult<Vec<SongInfo>> {
        self.musicapi.personal_fm()
    }

    // 收藏/喜欢
    // songid: 歌曲id
    // like: true 收藏，false 取消
    #[allow(unused)]
    pub(crate) async fn like(&mut self, like: bool, songid: u64) -> bool {
        if self.musicapi.like(like, songid) {
            if let Ok(login_info) = self.login_info().await {
                if let Ok(usl) = &self.user_song_list(login_info.uid, 0, 50).await {
                    let row_id = 0; // 假定喜欢的音乐歌单总排在第一位
                    self.song_list_detail(usl[row_id].id, true).await;
                    return true;
                }
            }
        }
        false
    }

    // FM 不喜欢
    // songid: 歌曲id
    #[allow(unused)]
    pub(crate) async fn fm_trash(&mut self, songid: u64) -> bool {
        self.musicapi.fm_trash(songid)
    }

    // 搜索
    // keywords: 关键词
    // types: 单曲(1)，歌手(100)，专辑(10)，歌单(1000)，用户(1002) *(type)*
    // offset: 起始点
    // limit: 数量
    #[allow(unused)]
    pub(crate) async fn search(&mut self, keywords: String, types: u32, offset: u16, limit: u16) -> NCMResult<String> {
        self.musicapi.search(keywords, types, offset, limit)
    }

    // 新碟上架
    // offset: 起始点
    // limit: 数量
    #[allow(unused)]
    pub(crate) async fn new_albums(&mut self, offset: u8, limit: u8) -> NCMResult<Vec<SongList>> {
        let path = format!("{}new_albums.db", NCM_DATA.to_string_lossy());
        if let Ok(buffer) = fs::read(&path).await {
            if let Ok(song_list) = bincode::deserialize::<Vec<SongList>>(&buffer) {
                return Ok(song_list);
            }
        }
        if let Ok(na) = self.musicapi.new_albums(offset, limit) {
            if !na.is_empty() {
                fs::write(
                    path,
                    bincode::serialize(&na)
                        .map_err(|_| Errors::NoneError)
                        .map_err(|_| Errors::NoneError)?,
                )
                .await?;
                return Ok(na);
            }
        }
        Err(Errors::NoneError)
    }

    // 专辑
    // album_id: 专辑 id
    #[allow(unused)]
    pub(crate) async fn album(&mut self, album_id: u64) -> NCMResult<Vec<SongInfo>> {
        let path = format!("{}album_{}.db", NCM_DATA.to_string_lossy(), album_id);
        if let Ok(buffer) = fs::read(&path).await {
            if let Ok(album) = bincode::deserialize::<Vec<SongInfo>>(&buffer) {
                return Ok(album);
            }
        }
        if let Ok(a) = self.musicapi.album(album_id) {
            if !a.is_empty() {
                fs::write(path, bincode::serialize(&a).map_err(|_| Errors::NoneError)?).await?;
                return Ok(a);
            }
        }
        Err(Errors::NoneError)
    }

    // 热门推荐歌单
    // offset: 起始点
    // limit: 数量
    // order: 排序方式:
    //	      "hot": 热门，
    //        "new": 最新
    #[allow(unused)]
    pub(crate) async fn top_song_list(&mut self, order: &str, offset: u8, limit: u8) -> NCMResult<Vec<SongList>> {
        let path = format!("{}top_song_list.db", NCM_DATA.to_string_lossy());
        if let Ok(buffer) = fs::read(&path).await {
            if let Ok(to_song_list) = bincode::deserialize::<Vec<SongList>>(&buffer) {
                return Ok(to_song_list);
            }
        }
        if let Ok(tsl) = self.musicapi.top_song_list(order, offset, limit) {
            if !tsl.is_empty() {
                fs::write(path, bincode::serialize(&tsl).map_err(|_| Errors::NoneError)?).await?;
                return Ok(tsl);
            }
        }
        Err(Errors::NoneError)
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
    pub(crate) async fn top_songs(&mut self, list_id: u64) -> NCMResult<Vec<SongInfo>> {
        self.song_list_detail(list_id, true).await
    }

    // 查询歌词
    // music_id: 歌曲id
    #[allow(unused)]
    pub(crate) async fn song_lyric(&mut self, music_id: u64) -> NCMResult<Vec<String>> {
        let path = format!("{}song_lyric_{}.db", NCM_DATA.to_string_lossy(), music_id);
        if let Ok(buffer) = fs::read(&path).await {
            if let Ok(song_lyric) = bincode::deserialize::<Vec<String>>(&buffer) {
                return Ok(song_lyric);
            }
        }
        if let Ok(sl) = self.musicapi.song_lyric(music_id) {
            if !sl.is_empty() {
                fs::write(path, bincode::serialize(&sl).map_err(|_| Errors::NoneError)?).await?;
                return Ok(sl);
            }
        }
        Err(Errors::NoneError)
    }

    // 收藏/取消收藏歌单
    // like: true 收藏，false 取消
    // id: 歌单 id
    #[allow(unused)]
    pub(crate) async fn song_list_like(&mut self, like: bool, id: u64) -> bool {
        self.musicapi.song_list_like(like, id)
    }
}

// 删除缓存文件
pub(crate) async fn clear_cache(dir: &Path) -> NCMResult<()> {
    if dir.is_dir() {
        let mut entries = fs::read_dir(dir).await?;
        while let Some(res) = entries.next().await {
            let path = res?.path();
            fs::remove_file(&path).await?;
        }
    }
    Ok(())
}

#[allow(unused)]
// 删除缓存文件
async fn clear_cache_starts_with(dir: &Path, start: &str) -> NCMResult<()> {
    if dir.is_dir() {
        let mut entries = fs::read_dir(dir).await?;
        while let Some(res) = entries.next().await {
            let file = res?;
            let path = file.path();
            if file.file_name().to_string_lossy().starts_with(start) {
                fs::remove_file(&path).await?;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    //use super::*;

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
