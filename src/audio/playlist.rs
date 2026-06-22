//
// playlist.rs
// Copyright (C) 2022 gmg137 <gmg137 AT live.com>
// Distributed under terms of the GPL-3.0-or-later license.
//

use gtk::glib;
use log::*;
use mpris_server::LoopStatus;
use ncm_api::SongInfo;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use crate::path::CONFIG;

// 播放列表持久化数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct PlayListData {
    // 原始歌曲列表
    pub list: Vec<SongInfo>,
    // 随机播放顺序
    pub shuffle: Vec<SongInfo>,
    // 循环模式字符串（none/one/loop/shuffle）
    pub loops: String,
    // 播放状态（true=正在播放）
    pub play_state: bool,
    // 当前歌曲在列表中的索引
    pub position: usize,
    // 播放进度（微秒）
    pub play_position: u64,
    // 当前播放歌曲 ID，用于加载时精确定位
    pub current_song_id: u64,
}

impl Default for PlayListData {
    fn default() -> Self {
        Self {
            list: Vec::new(),
            shuffle: Vec::new(),
            loops: "none".to_string(),
            play_state: false,
            position: 0,
            play_position: 0,
            current_song_id: 0,
        }
    }
}

// 播放列表保存文件路径
fn playlist_file_path() -> PathBuf {
    CONFIG.join("playlist.json")
}

// 原子写入文件，防止写入过程中崩溃导致数据损坏
fn atomic_write(path: &Path, data: &[u8]) -> std::io::Result<()> {
    let tmp_path = path.with_extension("json.tmp");
    fs::write(&tmp_path, data)?;
    fs::rename(&tmp_path, path)?;
    Ok(())
}

#[derive(Debug)]
pub struct PlayList {
    // 播放列表
    list: Vec<SongInfo>,
    // 随机播放列表
    shuffle: Vec<SongInfo>,
    // 循环状态
    loops: LoopsState,
    // 播放状态
    play_state: bool,
    // 当前播放位置
    position: usize,
    // 当前播放进度（微秒）
    play_position: u64,
}

impl Default for PlayList {
    fn default() -> Self {
        Self::new()
    }
}

impl PlayList {
    pub fn new() -> Self {
        PlayList {
            list: Vec::new(),
            shuffle: Vec::new(),
            loops: LoopsState::None,
            play_state: false,
            position: 0,
            play_position: 0,
        }
    }

    // 从文件加载播放列表
    pub fn load_from_file() -> Self {
        let path = playlist_file_path();
        if !path.exists() {
            return Self::new();
        }
        match fs::read_to_string(&path) {
            Ok(json) => match serde_json::from_str::<PlayListData>(&json) {
                Ok(mut data) => {
                    // 清除保存的URL（会在播放时重新获取）
                    for song in &mut data.list {
                        song.song_url.clear();
                    }
                    for song in &mut data.shuffle {
                        song.song_url.clear();
                    }
                    let loops = LoopsState::from_str(&data.loops);
                    let mut shuffle = data.shuffle;
                    // 确保 shuffle 列表非空
                    if shuffle.is_empty() && !data.list.is_empty() {
                        shuffle = data.list.clone();
                        fastrand::shuffle(&mut shuffle);
                    }
                    // 约束 position 在有效范围内
                    let max_pos = if loops == LoopsState::Shuffle {
                        shuffle.len().saturating_sub(1)
                    } else {
                        data.list.len().saturating_sub(1)
                    };
                    let position = data.position.min(max_pos);
                    let mut playlist = PlayList {
                        list: data.list,
                        shuffle,
                        loops,
                        play_state: data.play_state,
                        position,
                        play_position: data.play_position,
                    };
                    if data.current_song_id > 0 {
                        playlist.sync_position_with_song_id(data.current_song_id);
                    }
                    playlist
                }
                Err(e) => {
                    warn!("解析播放列表数据失败: {e:?}");
                    Self::new()
                }
            },
            Err(e) => {
                warn!("读取播放列表文件失败: {e:?}");
                Self::new()
            }
        }
    }

    // 保存播放列表到文件
    pub fn save_to_file(&self) {
        let data = PlayListData {
            list: self.list.clone(),
            shuffle: self.shuffle.clone(),
            loops: self.loops.to_string(),
            play_state: self.play_state,
            position: self.position,
            play_position: self.play_position,
            current_song_id: self.current_song().map(|s| s.id).unwrap_or(0),
        };
        match serde_json::to_string_pretty(&data) {
            Ok(json) => {
                if let Err(e) = atomic_write(&playlist_file_path(), json.as_bytes()) {
                    warn!("保存播放列表失败: {e:?}");
                }
            }
            Err(e) => {
                warn!("序列化播放列表数据失败: {e:?}");
            }
        }
    }

    // 获取播放进度（微秒）
    pub fn get_play_position(&self) -> u64 {
        self.play_position
    }

    // 设置播放进度（微秒）
    pub fn set_play_position(&mut self, position: u64) {
        self.play_position = position;
    }

    // 获取原始歌曲列表（克隆）
    pub fn get_list(&self) -> Vec<SongInfo> {
        self.list.clone()
    }

    pub fn current_song(&self) -> Option<&SongInfo> {
        if let LoopsState::Shuffle = self.loops {
            if let Some(song) = self.shuffle.get(self.position) {
                return Some(song);
            } else {
                return None;
            }
        }
        if let Some(song) = self.list.get(self.position) {
            Some(song)
        } else {
            None
        }
    }

    pub fn add_song(&mut self, song: SongInfo) {
        if self.list.is_empty() {
            if let LoopsState::Shuffle = self.loops {
                self.shuffle.push(song.clone());
            }
            self.list.push(song);
            return;
        }
        if !self.list.contains(&song) {
            if let LoopsState::Shuffle = self.loops {
                self.shuffle.insert(self.position + 1, song.clone())
            }
            self.list.insert(self.position + 1, song);
            self.position += 1;
        } else if let LoopsState::Shuffle = self.loops {
            for (i, v) in self.shuffle.iter().enumerate() {
                if v.id == song.id {
                    self.position = i;
                    break;
                }
            }
        } else {
            for (i, v) in self.list.iter().enumerate() {
                if v.id == song.id {
                    self.position = i;
                    break;
                }
            }
        }
    }

    #[must_use]
    pub fn len(&self) -> usize {
        match self.loops {
            LoopsState::Shuffle => self.shuffle.len(),
            _ => self.list.len(),
        }
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn remove_song(&mut self, song: SongInfo) {
        if self.list.is_empty() || !self.list.contains(&song) {
            return;
        }

        let mut shuffle_index = 0;
        let mut list_index = 0;

        if let Some(idx) = self.shuffle.iter().position(|s| s.id == song.id) {
            self.shuffle.remove(idx);
            shuffle_index = idx;
        }

        if let Some(idx) = self.list.iter().position(|s| s.id == song.id) {
            self.list.remove(idx);
            list_index = idx;
        }

        if let LoopsState::Shuffle = self.loops {
            if self.position > shuffle_index {
                self.position -= 1;
            }
        } else if self.position > list_index {
            self.position -= 1;
        }
    }

    pub fn add_list(&mut self, list: Vec<SongInfo>) {
        self.list = list.clone();
        let mut list = list;
        if let LoopsState::Shuffle = self.loops {
            fastrand::shuffle(&mut list);
            self.shuffle = list;
        }
        self.position = 0;
    }

    pub fn set_song_url(&mut self, si: SongInfo) {
        if let Some(si_old) = self.list.iter_mut().find(|s| s.id == si.id) {
            si_old.song_url = si.song_url.clone();
            si_old.quality = si.quality.clone();
        }
        if let LoopsState::Shuffle = self.loops {
            if let Some(si_old) = self.shuffle.iter_mut().find(|s| s.id == si.id) {
                si_old.song_url = si.song_url;
                si_old.quality = si.quality;
            }
        }
    }

    // 设置播放状态
    pub fn set_play_state(&mut self, state: bool) {
        self.play_state = state;
    }

    // 获取播放状态（true=正在播放）
    pub fn get_play_state(&self) -> bool {
        self.play_state
    }

    pub fn set_loops(&mut self, loops: LoopsState) {
        if let LoopsState::Shuffle = loops {
            if self.play_state {
                let first = self.list.remove(self.position);
                let mut list = self.list.clone();
                fastrand::shuffle(&mut list);
                list.insert(0, first);
                self.shuffle = list;
            } else {
                let mut list = self.list.clone();
                fastrand::shuffle(&mut list);
                self.shuffle = list;
            }
            self.position = 0;
        }
        self.loops = loops;
    }

    pub fn get_position(&self) -> usize {
        if let LoopsState::Shuffle = self.loops {
            if let Some(song) = self.current_song() {
                for (p, si) in self.list.iter().enumerate() {
                    if si.id == song.id {
                        return p;
                    }
                }
            }
        }
        self.position
    }

    pub fn set_position(&mut self, position: usize) {
        self.position = position;
    }

    // 根据 song_id 定位当前播放位置（防止歌单变更后位置偏移）
    pub fn sync_position_with_song_id(&mut self, song_id: u64) {
        let list = match self.loops {
            LoopsState::Shuffle => &self.shuffle,
            _ => &self.list,
        };
        if let Some(pos) = list.iter().position(|s| s.id == song_id) {
            self.position = pos;
        } else if let Some(pos) = self.list.iter().position(|s| s.id == song_id) {
            self.position = pos;
        }
    }

    // 查询下一曲
    pub fn get_next_song(&mut self) -> Option<&SongInfo> {
        match self.loops {
            LoopsState::Shuffle => {
                if let Some(song) = self.shuffle.get(self.position + 1) {
                    Some(song)
                } else {
                    self.shuffle.first()
                }
            }
            LoopsState::Playlist => {
                if let Some(song) = self.list.get(self.position + 1) {
                    Some(song)
                } else {
                    self.list.first()
                }
            }
            LoopsState::Track => self.list.get(self.position),
            LoopsState::None => {
                if let Some(song) = self.list.get(self.position + 1) {
                    Some(song)
                } else {
                    None
                }
            }
        }
    }

    // 获取下一曲
    pub fn next_song(&mut self) -> Option<&SongInfo> {
        match self.loops {
            LoopsState::Shuffle => {
                if let Some(song) = self.shuffle.get(self.position + 1) {
                    self.position += 1;
                    Some(song)
                } else {
                    self.position = 0;
                    self.shuffle.first()
                }
            }
            LoopsState::Playlist => {
                if let Some(song) = self.list.get(self.position + 1) {
                    self.position += 1;
                    Some(song)
                } else {
                    self.position = 0;
                    self.list.first()
                }
            }
            LoopsState::Track => self.list.get(self.position),
            LoopsState::None => {
                if let Some(song) = self.list.get(self.position + 1) {
                    self.position += 1;
                    Some(song)
                } else {
                    None
                }
            }
        }
    }

    // 获取上一曲
    pub fn prev_song(&mut self) -> Option<&SongInfo> {
        let position = if self.position == 0 {
            0
        } else {
            self.position - 1
        };
        match self.loops {
            LoopsState::Shuffle => {
                if let Some(song) = self.shuffle.get(position) {
                    self.position = position;
                    Some(song)
                } else {
                    None
                }
            }
            LoopsState::Playlist => {
                let position = if self.position == 0 {
                    self.list.len() - 1
                } else {
                    self.position - 1
                };
                if let Some(song) = self.list.get(position) {
                    self.position = position;
                    Some(song)
                } else {
                    self.position = 0;
                    self.list.first()
                }
            }
            LoopsState::Track => self.list.get(self.position),
            LoopsState::None => {
                if let Some(song) = self.list.get(position) {
                    self.position = position;
                    Some(song)
                } else {
                    None
                }
            }
        }
    }
}

#[allow(clippy::upper_case_acronyms)]
#[derive(Debug, Default, PartialEq, Eq, Clone, Copy, glib::Enum)]
#[enum_type(name = "LoopsState")]
pub enum LoopsState {
    // 随机
    Shuffle,
    // 列表循环
    Playlist,
    // 单曲循环
    Track,
    // 不循环
    #[default]
    None,
}

impl LoopsState {
    pub fn from_str(s: &str) -> Self {
        <Self as FromStr>::from_str(s).unwrap_or_default()
    }
}

impl FromStr for LoopsState {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "none" => Ok(LoopsState::None),
            "one" => Ok(LoopsState::Track),
            "loop" => Ok(LoopsState::Playlist),
            "shuffle" => Ok(LoopsState::Shuffle),
            _ => Err(()),
        }
    }
}

impl From<LoopStatus> for LoopsState {
    fn from(status: LoopStatus) -> Self {
        match status {
            LoopStatus::None => LoopsState::None,
            LoopStatus::Track => LoopsState::Track,
            LoopStatus::Playlist => LoopsState::Playlist,
        }
    }
}

impl std::fmt::Display for LoopsState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LoopsState::None => write!(f, "none"),
            LoopsState::Track => write!(f, "one"),
            LoopsState::Playlist => write!(f, "loop"),
            LoopsState::Shuffle => write!(f, "shuffle"),
        }
    }
}
