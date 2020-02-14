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
use async_std::{fs, future};
use cairo::{Context, ImageSurface};
use crossbeam_channel::Sender;
use gdk::pixbuf_get_from_surface;
use gdk::prelude::GdkContextExt;
use gdk_pixbuf::Pixbuf;
use rand::seq::SliceRandom;
use rand::thread_rng;
use serde::{Deserialize, Serialize};
use std::{io, io::Error, time::Duration};

// 下载音乐
// url: 网址
// path: 本地保存路径(包含文件名)
// timeout: 请求超时,单位毫秒(默认:1000)
pub(crate) async fn download_music<I, U>(url: I, path: I, timeout: U) -> Result<(), surf::Exception>
where
    I: Into<String>,
    U: Into<Option<u64>>,
{
    let url = url.into();
    let path = path.into();
    let timeout = timeout.into().unwrap_or(1000);
    if !std::path::Path::new(&path).exists() {
        if url.starts_with("http://") || url.starts_with("https://") {
            let music_url = url.replace("https:", "http:");
            let buffer = future::timeout(Duration::from_millis(timeout), surf::get(music_url).recv_bytes()).await??;
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
// timeout: 请求超时,单位毫秒(默认:1000)
pub(crate) async fn download_img<I, U>(
    url: I,
    path: I,
    width: u32,
    high: u32,
    timeout: U,
) -> Result<(), surf::Exception>
where
    I: Into<String>,
    U: Into<Option<u64>>,
{
    let url = url.into();
    let path = path.into();
    let timeout = timeout.into().unwrap_or(1000);
    if !std::path::Path::new(&path).exists() {
        if url.starts_with("http://") || url.starts_with("https://") {
            let image_url = format!("{}?param={}y{}", url, width, high).replace("https:", "http:");
            if let Ok(buffer) =
                future::timeout(Duration::from_millis(timeout), surf::get(image_url).recv_bytes()).await?
            {
                fs::write(path, buffer).await?;
            }
        }
    }
    Ok(())
}

// 播放列表数据
#[derive(Debug, Deserialize, Serialize)]
struct PlayerListData {
    // 播放列表: (歌曲信息,是否播放)
    player_list: Vec<(SongInfo, bool)>,
    // 混淆后的播放列表索引
    shuffle_list: Vec<i32>,
    // 当前播放歌曲的索引
    index: i32,
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
        let mut player_list: Vec<(SongInfo, bool)> = Vec::new();
        // 匹配歌曲 URL, 生成播放列表
        list.iter().for_each(|si| {
            if let Some(song_url) = v.iter().find(|su| su.id.eq(&si.id)) {
                player_list.push((
                    SongInfo {
                        song_url: song_url.url.to_owned(),
                        ..si.to_owned()
                    },
                    false,
                ));
            }
        });
        // 如果需要播放
        if !player_list.is_empty() && play {
            // 播放列表长度
            let len = player_list.len();
            // 创建随机播放 id 列表
            let mut shuffle_list: Vec<i32> = (0..).take(len).collect();
            shuffle_list.shuffle(&mut thread_rng());
            // 将播放列表写入数据库
            if let Ok(buffer) = bincode::serialize(&PlayerListData {
                player_list: player_list.to_owned(),
                shuffle_list: shuffle_list.clone(),
                index: -1,
            }) {
                let path = format!("{}player_list.db", NCM_DATA.to_string_lossy());
                if fs::write(path, buffer).await.is_ok() {
                    if play {
                        // 播放歌单
                        sender.send(Action::PlayerForward).ok();
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
// loops: 是否从头循环
#[allow(unused)]
pub(crate) async fn get_player_list_song(pd: PD, shuffle: bool, loops: bool) -> NCMResult<SongInfo> {
    // 查询播放列表
    let path = format!("{}player_list.db", NCM_DATA.to_string_lossy());
    let buffer = fs::read(&path).await?;
    // 反序列化播放列表
    let PlayerListData {
        mut player_list,
        index,
        shuffle_list,
    } = bincode::deserialize(&buffer).map_err(|_| Errors::NoneError)?;
    // 记录播放进度
    let mut index_new = index;
    // 要播放的歌曲索引
    let mut player_index = index;
    // 是否继续播放
    let mut proceed = true;
    // 如果播放列表不为空
    if !player_list.is_empty() {
        match pd {
            // 下一曲
            PD::FORWARD => {
                index_new += 1;
                player_index += 1;
                // 标记上一歌曲为已播放
                if let Some((_, p)) = player_list.get_mut(index as usize) {
                    *p = true;
                }
                // 从头开始播放
                if loops {
                    if index + 1 >= player_list.len() as i32 {
                        index_new = 0;
                        player_index = 0;
                        player_list.iter_mut().for_each(|(_, p)| *p = false);
                    }
                } else {
                    if index + 1 >= player_list.len() as i32 {
                        index_new -= 1;
                        proceed = false;
                    } else {
                        if shuffle {
                            loop {
                                player_index = if let Some(pi) = shuffle_list.get(index_new as usize) {
                                    *pi
                                } else {
                                    index_new = index;
                                    proceed = false;
                                    break;
                                };
                                if !player_list[player_index as usize].1 {
                                    break;
                                }
                                index_new += 1;
                            }
                        }
                    }
                }
                fs::write(
                    &path,
                    bincode::serialize(&PlayerListData {
                        player_list: player_list.to_owned(),
                        index: index_new,
                        shuffle_list,
                    })
                    .map_err(|_| Errors::NoneError)?,
                )
                .await?;
                if proceed {
                    if let Some((si, _)) = player_list.get(player_index as usize) {
                        return Ok(si.to_owned());
                    }
                }
            }
            // 上一曲
            PD::BACKWARD => {
                index_new -= 1;
                player_index -= 1;
                // 循环播放
                if index_new < 0 {
                    if loops {
                        index_new = player_list.len() as i32 - 1;
                        player_index = index_new;
                    } else {
                        index_new += 1;
                        proceed = false;
                    }
                } else {
                    // 查找上一曲索引
                    if shuffle {
                        // 混淆模式的歌曲索引
                        player_index = *shuffle_list.get(index_new as usize).unwrap_or(&0);
                    }
                }
                // 标记当前歌曲为未播放
                if let Some((_, p)) = player_list.get_mut(index as usize) {
                    *p = false;
                }
                fs::write(
                    path,
                    bincode::serialize(&PlayerListData {
                        player_list: player_list.to_owned(),
                        index: index_new,
                        shuffle_list,
                    })
                    .map_err(|_| Errors::NoneError)?,
                )
                .await?;
                if let Some((si, _)) = player_list.get(player_index as usize) {
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
    } = bincode::deserialize(&buffer).map_err(|_| Errors::NoneError)?;
    // 提取歌曲 id 列表
    let song_id_list = player_list.iter().map(|(si, _)| si.id).collect::<Vec<u32>>();
    let mut api = MusicData::new().await?;
    // 批量搜索歌曲 URL
    if let Ok(v) = api.songs_url(&song_id_list, 320).await {
        // 初始化播放列表
        let mut new_player_list: Vec<(SongInfo, bool)> = Vec::new();
        // 匹配歌曲 URL, 生成播放列表
        player_list.iter().for_each(|(si, p)| {
            if let Some(song_url) = v.iter().find(|su| su.id.eq(&si.id)) {
                new_player_list.push((
                    SongInfo {
                        song_url: song_url.url.to_owned(),
                        ..si.to_owned()
                    },
                    *p,
                ));
            }
        });
        // 如果播放列表为空则退出
        if !new_player_list.is_empty() {
            // 删除错误缓存
            let mp3_path = format!(
                "{}{}.mp3",
                NCM_CACHE.to_string_lossy(),
                new_player_list[index as usize].0.id
            );
            fs::remove_file(&mp3_path).await.ok();
            // 继续播放歌曲
            sender
                .send(Action::ReadyPlayer(new_player_list[index as usize].0.to_owned()))
                .ok();
            // 将播放列表写入数据库
            fs::write(
                path,
                bincode::serialize(&PlayerListData {
                    player_list: new_player_list,
                    index,
                    shuffle_list,
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

// 下载 osdlyrics 歌词
pub(crate) async fn download_lyrics(data: &mut MusicData, file: &str, song_info: &SongInfo) -> NCMResult<()> {
    let path = format!("{}/{}.lrc", *LYRICS_PATH, file);
    if !std::path::Path::new(&path).exists() {
        let vec = data.song_lyric(song_info.id).await?;
        let lrc = vec.join("\n");
        fs::write(path, lrc).await?;
    }
    Ok(())
}

// 获取歌词
pub(crate) async fn get_lyrics(data: &mut MusicData, song_id: u32) -> NCMResult<String> {
    let path = format!("{}{}.lrc", NCM_CACHE.to_string_lossy(), song_id);
    if !std::path::Path::new(&path).exists() {
        let vec = data.song_lyric(song_id).await?;
        let re = regex::Regex::new(r"\[\d+:\d+.\d+\]").unwrap();
        let lrc = vec
            .into_iter()
            .map(|v| re.replace_all(&v, "").to_string())
            .collect::<Vec<String>>()
            .join("\n");
        fs::write(&path, &lrc).await?;
        return Ok(lrc);
    }
    let lrc = fs::read_to_string(&path).await?;
    Ok(lrc)
}
