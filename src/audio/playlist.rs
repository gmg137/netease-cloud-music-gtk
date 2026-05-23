//
// playlist.rs
// Copyright (C) 2022 gmg137 <gmg137 AT live.com>
// Distributed under terms of the GPL-3.0-or-later license.
//

use gtk::glib;
use mpris_server::LoopStatus;
use ncm_api::SongInfo;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

// 用于持久化的播放列表数据结构
#[derive(Debug, Serialize, Deserialize)]
struct PlayListData {
    list: Vec<SongInfo>,
    loops: String,
    position: usize,
    #[serde(default)]
    play_position: u64, // 播放位置，单位：微秒
    #[serde(default)]
    play_state: bool, // 是否正在播放
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
    // 播放进度位置（微秒）
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

    // 获取播放列表持久化文件路径
    fn get_playlist_file_path() -> PathBuf {
        let mut path = crate::path::CACHE.clone();
        path.push("playlist.json");
        path
    }

    // 保存播放列表到文件
    pub fn save_to_file(&self) -> Result<(), Box<dyn std::error::Error>> {
        log::debug!("save_to_file() 被调用，播放列表大小: {}", self.list.len());

        let data = PlayListData {
            list: self.list.clone(),
            loops: self.loops.to_string(),
            position: self.position,
            play_position: self.play_position,
            play_state: self.play_state,
        };

        let json = serde_json::to_string_pretty(&data)?;
        let json_size = json.len();
        log::debug!("序列化后的 JSON 大小: {} 字节", json_size);
        log::debug!("保存播放进度: {} 微秒 (约 {} 秒)", self.play_position, self.play_position / 1_000_000);

        let path = Self::get_playlist_file_path();
        log::debug!("播放列表文件路径: {:?}", path);

        // 确保目录存在
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                log::error!("无法创建播放列表目录 {:?}: {}", parent, e);
                e
            })?;
        }

        fs::write(&path, json).map_err(|e| {
            log::error!("无法保存播放列表到 {:?}: {}", path, e);
            e
        })?;
        log::info!("播放列表已成功保存到 {:?}，共 {} 首歌曲，文件大小 {} 字节", path, self.list.len(), json_size);
        Ok(())
    }

    // 从文件加载播放列表
    pub fn load_from_file() -> Result<Self, Box<dyn std::error::Error>> {
        let path = Self::get_playlist_file_path();
        if !path.exists() {
            return Ok(Self::new());
        }

        let json = fs::read_to_string(path)?;
        let data: PlayListData = serde_json::from_str(&json)?;

        let mut playlist = Self::new();
        playlist.list = data.list.clone();
        playlist.position = data.position;
        playlist.loops = LoopsState::from_str(&data.loops);
        playlist.play_position = data.play_position;
        playlist.play_state = data.play_state;

        // 如果是随机模式，重新生成随机列表
        if let LoopsState::Shuffle = playlist.loops {
            let mut list = data.list;
            fastrand::shuffle(&mut list);
            playlist.shuffle = list;
        }

        log::debug!("播放列表已加载，共 {} 首歌曲", playlist.list.len());
        log::debug!("恢复播放进度: {} 微秒 (约 {} 秒)", playlist.play_position, playlist.play_position / 1_000_000);
        Ok(playlist)
    }

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

    pub fn len(&self) -> usize {
        match self.loops {
            LoopsState::Shuffle => self.shuffle.len(),
            _ => self.list.len(),
        }
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
        }
        if let LoopsState::Shuffle = self.loops {
            if let Some(si_old) = self.shuffle.iter_mut().find(|s| s.id == si.id) {
                si_old.song_url = si.song_url;
            }
        }
    }

    pub fn set_play_state(&mut self, state: bool) {
        self.play_state = state;
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

    pub fn get_play_position(&self) -> u64 {
        self.play_position
    }

    pub fn set_play_position(&mut self, position: u64) {
        self.play_position = position;
    }

    pub fn get_play_state(&self) -> bool {
        self.play_state
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
        match s {
            "none" => LoopsState::None,
            "one" => LoopsState::Track,
            "loop" => LoopsState::Playlist,
            "shuffle" => LoopsState::Shuffle,
            _ => LoopsState::None,
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

impl ToString for LoopsState {
    fn to_string(&self) -> String {
        match self {
            LoopsState::None => "none".to_string(),
            LoopsState::Track => "one".to_string(),
            LoopsState::Playlist => "loop".to_string(),
            LoopsState::Shuffle => "shuffle".to_string(),
        }
    }
}
