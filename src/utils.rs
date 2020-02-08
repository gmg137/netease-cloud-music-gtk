//
// utils.rs
// Copyright (C) 2019 gmg137 <gmg137@live.com>
// Distributed under terms of the GPLv3 license.
//
use crate::app::Action;
use crate::data::{clear_cache, MusicData};
use crate::model::{Errors, NCMResult, DATE_DAY, DATE_MONTH, ISO_WEEK, LYRICS_PATH, NCM_CACHE, NCM_CONFIG, NCM_DATA};
use crate::musicapi::model::SongInfo;
use crate::widgets::player::LoopsState;
use async_std::fs;
use cairo::{Context, ImageSurface};
use crossbeam_channel::Sender;
use gdk::pixbuf_get_from_surface;
use gdk::prelude::GdkContextExt;
use gdk_pixbuf::Pixbuf;
use rand::seq::SliceRandom;
use rand::thread_rng;
use serde::{Deserialize, Serialize};
use std::{io, io::Error};

// 下载音乐
// url: 网址
// path: 本地保存路径(包含文件名)
pub(crate) async fn download_music(url: &str, path: &str) -> Result<(), surf::Exception> {
    if !std::path::Path::new(&path).exists() {
        if url.starts_with("http://") || url.starts_with("https://") {
            let music_url = url.replace("https:", "http:");
            let buffer = surf::get(music_url).recv_bytes().await?;
            if !buffer.is_empty() {
                fs::write(path, buffer).await?;
            }
        }
    }
    Ok(())
}

// 从网络下载图片
// url: 网址
// path: 本地保存路径(包含文件名)
// width: 宽度
// high: 高度
pub(crate) async fn download_img(url: &str, path: &str, width: u32, high: u32) -> Result<(), surf::Exception> {
    if !std::path::Path::new(&path).exists() {
        if url.starts_with("http://") || url.starts_with("https://") {
            let image_url = format!("{}?param={}y{}", url, width, high).replace("https:", "http:");
            let buffer = surf::get(image_url).recv_bytes().await?;
            fs::write(path, buffer).await?;
        }
    }
    Ok(())
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
pub(crate) async fn create_player_list(list: &Vec<SongInfo>, sender: Sender<Action>, play: bool) -> NCMResult<()> {
    // 提取歌曲 id 列表
    let song_id_list = list.iter().map(|si| si.id).collect::<Vec<u32>>();
    let mut api = MusicData::new().await?;
    // 批量搜索歌曲 URL
    if let Ok(v) = api.songs_url(&song_id_list, 320).await {
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
        // 如果需要播放
        if !player_list.is_empty() && play {
            // 播放列表长度
            let len = player_list.len();
            // 创建随机播放 id 列表
            let mut shuffle_list: Vec<u32> = (0..).take(len).collect();
            shuffle_list.shuffle(&mut thread_rng());
            // 将播放列表写入数据库
            if let Ok(buffer) = bincode::serialize(&PlayerListData {
                player_list: player_list.to_owned(),
                index: 0,
                shuffle_list: shuffle_list.clone(),
                play_flag: vec![false; len],
            }) {
                let path = format!("{}player_list.db", NCM_DATA.to_string_lossy());
                if fs::write(path, buffer).await.is_ok() {
                    if play {
                        // 播放第一首歌曲
                        sender.send(Action::ReadyPlayer(player_list[0].to_owned())).ok();
                        return Ok(());
                    }
                }
            }
            sender.send(Action::ShowNotice("播放失败!".to_owned())).ok();
        }
    }
    Ok(())
}

// 查询播放列表
// pd: 上一曲/下一曲
// shuffle: 是否为随机查找
// update: 是否从头循环
#[allow(unused)]
pub(crate) async fn get_player_list_song(pd: PD, shuffle: bool, update: bool) -> NCMResult<SongInfo> {
    // 查询播放列表
    let path = format!("{}player_list.db", NCM_DATA.to_string_lossy());
    let buffer = fs::read(&path).await?;
    // 反序列化播放列表
    let PlayerListData {
        player_list,
        index,
        shuffle_list,
        play_flag,
    } = bincode::deserialize(&buffer).map_err(|_| Errors::NoneError)?;
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
                            return Err(Errors::NoneError);
                        }
                    }
                }
                fs::write(
                    &path,
                    bincode::serialize(&PlayerListData {
                        player_list: player_list.to_owned(),
                        index: index_old,
                        shuffle_list,
                        play_flag,
                    })
                    .map_err(|_| Errors::NoneError)?,
                )
                .await?;
                if let Some(si) = player_list.get(index_new as usize) {
                    return Ok(si.to_owned());
                }
            }
            // 上一曲
            PD::BACKWARD => {
                // 查找上一曲索引
                index_new = if shuffle {
                    // 混淆模式的歌曲索引
                    *shuffle_list.get(index_old as usize - 1).unwrap_or(&0)
                } else {
                    if index_old == 0 {
                        0
                    } else {
                        index_old - 1
                    }
                };
                // 标记当前歌曲为未播放
                play_flag[index_old as usize] = false;
                fs::write(
                    path,
                    bincode::serialize(&PlayerListData {
                        player_list: player_list.to_owned(),
                        index: if index_old == 0 { 0 } else { index_old - 1 },
                        shuffle_list,
                        play_flag,
                    })
                    .map_err(|_| Errors::NoneError)?,
                )
                .await?;
                if let Some(si) = player_list.get(index_new as usize) {
                    return Ok(si.to_owned());
                }
            }
        }
    }
    Err(Errors::NoneError)
}

// 刷新播放列表
#[allow(unused)]
pub(crate) async fn update_player_list(sender: Sender<Action>) -> NCMResult<()> {
    let path = format!("{}player_list.db", NCM_DATA.to_string_lossy());
    let buffer = fs::read(&path).await?;
    // 反序列化播放列表
    let PlayerListData {
        player_list,
        index,
        shuffle_list,
        play_flag,
    } = bincode::deserialize(&buffer).map_err(|_| Errors::NoneError)?;
    // 提取歌曲 id 列表
    let song_id_list = player_list.iter().map(|si| si.id).collect::<Vec<u32>>();
    let mut api = MusicData::new().await?;
    // 批量搜索歌曲 URL
    if let Ok(v) = api.songs_url(&song_id_list, 320).await {
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
        if !new_player_list.is_empty() {
            // 删除错误缓存
            let mp3_path = format!(
                "{}{}.mp3",
                NCM_CACHE.to_string_lossy(),
                new_player_list[index as usize].id
            );
            fs::remove_file(&mp3_path).await.ok();
            // 继续播放歌曲
            sender
                .send(Action::ReadyPlayer(new_player_list[index as usize].to_owned()))
                .ok();
            // 将播放列表写入数据库
            fs::write(
                path,
                bincode::serialize(&PlayerListData {
                    player_list: new_player_list,
                    index,
                    shuffle_list,
                    play_flag,
                })
                .map_err(|_| Errors::NoneError)?,
            )
            .await?;
        }
    }
    Ok(())
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
pub(crate) fn create_round_avatar(src: &str) -> io::Result<Pixbuf> {
    // 初始化图像
    let image = Pixbuf::new_from_file(src).map_err(|_| Error::last_os_error())?;

    // 获取宽高
    let w = image.get_width();
    let h = image.get_height();

    // 创建底图
    let surface = ImageSurface::create(cairo::Format::ARgb32, w, h).map_err(|_| Error::last_os_error())?;
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
    context.set_source_pixbuf(&image, 0.0, 0.0);
    context.paint();

    let pixbuf = pixbuf_get_from_surface(&surface, 0, 0, w, h).ok_or(Error::last_os_error())?;

    Ok(pixbuf)
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

// 缓存治理规则
#[derive(Debug, Clone, Deserialize, Serialize)]
pub(crate) enum ClearCached {
    // 从不
    NONE,
    // 每月
    MONTH(u32),
    // 每周
    WEEK(u32),
    // 每天
    DAY(u32),
}

// 全局配置
#[derive(Debug, Clone, Deserialize, Serialize)]
pub(crate) struct Configs {
    // 是否关闭到系统托盘
    pub(crate) tray: bool,
    // 是否下载歌词
    pub(crate) lyrics: bool,
    // 循环模式
    pub(crate) loops: LoopsState,
    // 自动清理缓存
    pub(crate) clear: ClearCached,
}

// 加载配置
#[allow(unused)]
pub(crate) async fn get_config() -> NCMResult<Configs> {
    let path = format!("{}config.db", NCM_CONFIG.to_string_lossy());
    if let Ok(buffer) = fs::read(path).await {
        if let Ok(mut conf) = bincode::deserialize::<Configs>(&buffer).map_err(|_| Errors::NoneError) {
            match conf.clear {
                ClearCached::NONE => {}
                ClearCached::MONTH(month) => {
                    if month != *DATE_MONTH {
                        // 清理缓存文件
                        clear_cache(&NCM_CACHE).await;
                        conf.clear = ClearCached::MONTH(*DATE_MONTH);
                        save_config(&conf).await;
                    }
                }
                ClearCached::WEEK(week) => {
                    if week != *ISO_WEEK {
                        // 清理缓存文件
                        clear_cache(&NCM_CACHE).await;
                        conf.clear = ClearCached::WEEK(*ISO_WEEK);
                        save_config(&conf).await;
                    }
                }
                ClearCached::DAY(day) => {
                    if day != *DATE_DAY {
                        // 清理缓存文件
                        clear_cache(&NCM_CACHE).await;
                        conf.clear = ClearCached::DAY(*DATE_DAY);
                        save_config(&conf).await;
                    }
                }
            }
            return Ok(conf);
        }
    }
    let conf = Configs {
        tray: false,
        lyrics: false,
        loops: LoopsState::CONSECUTIVE,
        clear: ClearCached::NONE,
    };
    Ok(conf)
}

// 保存配置
#[allow(unused)]
pub(crate) async fn save_config(conf: &Configs) -> NCMResult<()> {
    fs::write(
        format!("{}config.db", NCM_CONFIG.to_string_lossy()),
        bincode::serialize(&conf).map_err(|_| Errors::NoneError)?,
    )
    .await?;
    Ok(())
}

// 下载歌词
pub(crate) async fn download_lyrics(file: &str, song_info: &SongInfo) -> NCMResult<()> {
    let path = format!("{}/{}.lrc", *LYRICS_PATH, file);
    if !std::path::Path::new(&path).exists() {
        let mut data = MusicData::new().await?;
        let vec = data.song_lyric(song_info.id).await?;
        let mut lrc = String::new();
        vec.iter().for_each(|v| {
            lrc.push_str(v);
            lrc.push_str("\n");
        });
        fs::write(path, lrc).await?;
    }
    Ok(())
}
