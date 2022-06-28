//
// model.rs
// Copyright (C) 2022 gmg137 <gmg137 AT live.com>
// Distributed under terms of the GPL-3.0-or-later license.
//
use gtk::glib;

#[derive(Debug, Clone)]
pub enum UserMenuChild {
    Qr,
    Phone,
    User,
}

#[derive(Debug, Clone)]
pub enum DiscoverSubPage {
    SongList,
    Album,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, glib::Enum)]
#[repr(i32)]
#[enum_type(name = "SearchType")]
pub enum SearchType {
    // 搜索歌曲
    Song,
    // 搜索歌手
    Singer,
    // 搜索专辑
    Album,
    // 搜索歌词
    Lyrics,
    // 搜索歌单
    SongList,
    // 搜索歌手歌曲
    SingerSongs,
    // 搜索热门歌单
    TopPicks,
    // 搜索全部专辑
    AllAlbums,
    // 搜索每日推荐歌曲
    DailyRec,
    // 我喜欢的音乐
    Heartbeat,
    // 云盘音乐
    CloudDisk,
    // 每人FM
    Fm,
    // 收藏的专辑
    LikeAlbums,
    // 收藏的歌单
    LikeSongList,
}

impl Default for SearchType {
    fn default() -> Self {
        SearchType::Song
    }
}
