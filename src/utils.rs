//
// utils.rs
// Copyright (C) 2019 gmg137 <gmg137@live.com>
// Distributed under terms of the GPLv3 license.
//
use crate::app::Action;
use crate::data::MusicData;
use crate::musicapi::{model::SongInfo, MusicApi};
use crate::{CONFIG_PATH, LYRICS_PATH};
use cairo::{Context, ImageSurface};
use crossbeam_channel::Sender;
use rand::seq::SliceRandom;
use rand::thread_rng;
use serde::{Deserialize, Serialize};
use sled::*;
use std::sync::{Arc, Mutex};
use std::{fs::File, io, io::Error};

// 从网络下载图片
// url: 网址
// path: 本地保存路径(包含文件名)
// width: 宽度
// high: 高度
pub(crate) fn download_img(url: &str, path: &str, width: u32, high: u32) {
    if !std::path::Path::new(&path).exists() {
        let image_url = format!("{}?param={}y{}", url, width, high);
        if let Ok(mut body) = reqwest::get(&image_url) {
            if let Ok(mut out) = std::fs::File::create(&path) {
                std::io::copy(&mut body, &mut out).unwrap_or(0);
            }
        }
    }
}

// 播放列表数据
#[derive(Debug, Deserialize, Serialize)]
struct PlayerListData {
    // 播放列表
    player_list: Vec<SongInfo>,
    // 当前播放歌曲的索引
    index: u32,
    // 混淆后的播放列表索引
    shuffle_list: Vec<u32>,
    // 播放标志
    play_flag: Vec<bool>,
}

// 创建播放列表
// play: 是否立即播放
#[allow(unused)]
pub(crate) fn create_player_list(list: &Vec<SongInfo>, sender: Sender<Action>, play: bool) {
    let list = list.clone();
    let sender = sender.clone();
    std::thread::spawn(move || {
        // 提取歌曲 id 列表
        let song_id_list = list.iter().map(|si| si.id).collect::<Vec<u32>>();
        let mut api = MusicApi::new();
        // 批量搜索歌曲 URL
        if let Some(v) = api.songs_url(&song_id_list, 320) {
            // 初始化播放列表
            let mut player_list: Vec<SongInfo> = Vec::new();
            // 匹配歌曲 URL, 生成播放列表
            list.iter().for_each(|si| {
                if let Some(song_url) = v.iter().find(|su| su.id.eq(&si.id)) {
                    player_list.push(SongInfo {
                        song_url: song_url.url.to_owned(),
                        ..si.to_owned()
                    });
                }
            });
            // 如果播放列表为空则提示
            if player_list.is_empty() && play {
                sender
                    .send(Action::ShowNotice("播放失败!".to_owned()))
                    .unwrap();
                return;
            }
            // 播放列表长度
            let len = player_list.len();
            // 创建随机播放 id 列表
            let mut rng = thread_rng();
            let mut shuffle_list: Vec<u32> = (0..).take(len).collect();
            shuffle_list.shuffle(&mut rng);
            if play {
                // 播放第一首歌曲
                sender
                    .send(Action::Player(
                        player_list[0].to_owned(),
                        player_list[0].song_url.to_owned(),
                    ))
                    .unwrap();
            }
            // 将播放列表写入数据库
            let config = ConfigBuilder::default()
                .path(format!("{}/player_list.db", CONFIG_PATH.to_owned()))
                .build();
            if let Ok(db) = Db::start(config) {
                db.set(
                    b"player_list_data",
                    serde_json::to_vec(&PlayerListData {
                        player_list,
                        index: 0,
                        shuffle_list,
                        play_flag: vec![false; len],
                    })
                    .unwrap_or(vec![]),
                );
                db.flush();
            }
        }
    });
}

// 查询播放列表
// pd: 上一曲/下一曲
// shuffle: 是否为随机查找
// update: 是否从头循环
#[allow(unused)]
pub(crate) fn get_player_list_song(pd: PD, shuffle: bool, update: bool) -> Option<SongInfo> {
    let config = ConfigBuilder::default()
        .path(format!("{}/player_list.db", CONFIG_PATH.to_owned()))
        .build();
    if let Ok(db) = Db::start(config) {
        // 从数据库查询播放列表
        if let Some(player_list_data_v) = db.get(b"player_list_data").unwrap_or(None) {
            // 反序列化播放列表
            let PlayerListData {
                player_list,
                index,
                shuffle_list,
                play_flag,
            } = serde_json::from_slice::<PlayerListData>(&player_list_data_v).unwrap_or(
                PlayerListData {
                    player_list: vec![],
                    index: 0,
                    shuffle_list: vec![],
                    play_flag: vec![],
                },
            );
            let mut index_old = index;
            let mut index_new = index;
            let mut play_flag = play_flag;
            let len = play_flag.len();
            // 如果播放列表不为空
            if !player_list.is_empty() {
                match pd {
                    // 下一曲
                    PD::FORWARD => {
                        // 标记上一歌曲为已播放
                        play_flag[index as usize] = true;
                        // 从头开始播放
                        if update {
                            index_new = 0;
                            play_flag = vec![false; len];
                        } else {
                            // 查找下一曲索引
                            loop {
                                // 下一曲索引
                                let index_now = if shuffle {
                                    // 混淆模式的歌曲索引
                                    *shuffle_list.get(index_old as usize + 1).unwrap_or(&0)
                                } else {
                                    index_old + 1
                                };
                                // 判断该索引对应歌曲是否播放过
                                if let Some(flag) = play_flag.get(index_now as usize) {
                                    if *flag {
                                        index_old += 1;
                                    } else {
                                        index_old += 1;
                                        index_new = index_now;
                                        break;
                                    }
                                } else {
                                    return None;
                                }
                            }
                        }
                        db.set(
                            b"player_list_data",
                            serde_json::to_vec(&PlayerListData {
                                player_list: player_list.to_owned(),
                                index: index_old,
                                shuffle_list,
                                play_flag,
                            })
                            .unwrap_or(vec![]),
                        );
                        db.flush();
                        if let Some(si) = player_list.get(index_new as usize) {
                            return Some(si.to_owned());
                        }
                    }
                    // 上一曲
                    PD::BACKWARD => {
                        // 查找上一曲索引
                        index_new = if shuffle {
                            // 混淆模式的歌曲索引
                            *shuffle_list.get(index_old as usize - 1).unwrap_or(&0)
                        } else {
                            index_old - 1
                        };
                        // 标记当前歌曲为未播放
                        play_flag[index_old as usize] = false;
                        db.set(
                            b"player_list_data",
                            serde_json::to_vec(&PlayerListData {
                                player_list: player_list.to_owned(),
                                index: if index_old == 0 { 0 } else { index_old - 1 },
                                shuffle_list,
                                play_flag,
                            })
                            .unwrap_or(vec![]),
                        );
                        db.flush();
                        if let Some(si) = player_list.get(index_new as usize) {
                            return Some(si.to_owned());
                        }
                    }
                }
            }
        }
    }
    None
}

// 刷新播放列表
#[allow(unused)]
pub(crate) fn update_player_list(sender: Sender<Action>) {
    let sender = sender.clone();
    std::thread::spawn(move || {
        let config = ConfigBuilder::default()
            .path(format!("{}/player_list.db", CONFIG_PATH.to_owned()))
            .build();
        if let Ok(db) = Db::start(config) {
            // 从数据库查询播放列表
            if let Some(player_list_data_v) = db.get(b"player_list_data").unwrap_or(None) {
                // 反序列化播放列表
                let PlayerListData {
                    player_list,
                    index,
                    shuffle_list,
                    play_flag,
                } = serde_json::from_slice::<PlayerListData>(&player_list_data_v).unwrap_or(
                    PlayerListData {
                        player_list: vec![],
                        index: 0,
                        shuffle_list: vec![],
                        play_flag: vec![],
                    },
                );
                // 提取歌曲 id 列表
                let song_id_list = player_list.iter().map(|si| si.id).collect::<Vec<u32>>();
                let mut api = MusicApi::new();
                // 批量搜索歌曲 URL
                if let Some(v) = api.songs_url(&song_id_list, 320) {
                    // 初始化播放列表
                    let mut new_player_list: Vec<SongInfo> = Vec::new();
                    // 匹配歌曲 URL, 生成播放列表
                    player_list.iter().for_each(|si| {
                        if let Some(song_url) = v.iter().find(|su| su.id.eq(&si.id)) {
                            new_player_list.push(SongInfo {
                                song_url: song_url.url.to_owned(),
                                ..si.to_owned()
                            });
                        }
                    });
                    // 如果播放列表为空则退出
                    if new_player_list.is_empty() {
                        return;
                    }
                    // 播放第一首歌曲
                    sender
                        .send(Action::Player(
                            new_player_list[index as usize].to_owned(),
                            new_player_list[index as usize].song_url.to_owned(),
                        ))
                        .unwrap();
                    // 将播放列表写入数据库
                    db.set(
                        b"player_list_data",
                        serde_json::to_vec(&PlayerListData {
                            player_list: new_player_list,
                            index,
                            shuffle_list,
                            play_flag,
                        })
                        .unwrap_or(vec![]),
                    );
                    db.flush();
                }
            }
        }
    });
}

// 查询方向
pub(crate) enum PD {
    // 下一曲
    FORWARD,
    // 上一曲
    BACKWARD,
}

// 创建圆形头像
#[allow(unused)]
pub(crate) fn create_round_avatar(src: String) -> io::Result<()> {
    // 转换 jpg 为 png
    let image = image::open(&format!("{}.jpg", src)).map_err(|_| Error::last_os_error())?;
    let src = format!("{}.png", src);
    image.save(&src).map_err(|_| Error::last_os_error())?;

    // 初始化图像
    let mut f = File::open(&src)?;
    let image = ImageSurface::create_from_png(&mut f).map_err(|_| Error::last_os_error())?;

    // 获取宽高
    let w = image.get_width();
    let h = image.get_height();

    // 创建底图
    let surface =
        ImageSurface::create(cairo::Format::ARgb32, w, h).map_err(|_| Error::last_os_error())?;
    let context = Context::new(&surface);
    // 画出圆弧
    context.arc(
        w as f64 / 2.,
        h as f64 / 2.,
        w as f64 / 2.,
        0.0,
        2.0 * std::f64::consts::PI,
    );
    context.clip();
    context.new_path();

    context.scale(1.0, 1.0);
    context.set_source_surface(&image, 0.0, 0.0);
    context.paint();

    let mut file = File::create(&src)?;
    surface
        .write_to_png(&mut file)
        .map_err(|_| Error::last_os_error())?;

    Ok(())
}

#[allow(unused)]
#[derive(Debug, Clone)]
// 播放模式
pub(crate) enum PlayerTypes {
    // 歌曲
    Song,
    // Fm
    Fm,
}

// 全局配置
#[derive(Debug, Clone, Deserialize, Serialize)]
pub(crate) struct Configs {
    // 是否关闭到系统托盘
    pub(crate) tray: bool,
    // 是否下载歌词
    pub(crate) lyrics: bool,
}

// 加载配置
#[allow(unused)]
pub(crate) fn load_config() -> Configs {
    let config = ConfigBuilder::default()
        .path(format!("{}/config.db", CONFIG_PATH.to_owned()))
        .build();
    if let Ok(db) = Db::start(config) {
        if let Some(conf) = db.get(b"config").unwrap_or(None) {
            return serde_json::from_slice::<Configs>(&conf).unwrap_or(Configs {
                tray: false,
                lyrics: false,
            });
        }
    }
    let conf = Configs {
        tray: false,
        lyrics: false,
    };
    save_config(&conf);
    conf
}

// 保存配置
#[allow(unused)]
pub(crate) fn save_config(conf: &Configs) {
    let config = ConfigBuilder::default()
        .path(format!("{}/config.db", CONFIG_PATH.to_owned()))
        .build();
    if let Ok(db) = Db::start(config) {
        db.set(b"config", serde_json::to_vec(&conf).unwrap_or(vec![]));
        db.flush();
    }
}

// 下载歌词
pub(crate) fn download_lyrics(file: &str, song_info: &SongInfo, data: Arc<Mutex<u8>>) {
    let path = format!("{}/{}.lrc", *LYRICS_PATH, file);
    if !std::path::Path::new(&path).exists() {
        #[allow(unused_variables)]
        let lock = data.lock().unwrap();
        let mut data = MusicData::new();
        if let Some(vec) = data.song_lyric(song_info.id) {
            let mut lrc = String::new();
            vec.iter().for_each(|v| {
                lrc.push_str(v);
                lrc.push_str("\n");
            });
            std::fs::write(path, lrc).unwrap_or(());
        }
    }
}
