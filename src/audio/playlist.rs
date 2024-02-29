//
// playlist.rs
// Copyright (C) 2022 gmg137 <gmg137 AT live.com>
// Distributed under terms of the GPL-3.0-or-later license.
//

use gtk::glib;
use mpris_server::LoopStatus;
use ncm_api::SongInfo;

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
        }
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

    pub fn add_list(&mut self, list: Vec<SongInfo>) {
        self.list = list.clone();
        let mut list = list;
        if let LoopsState::Shuffle = self.loops {
            fastrand::shuffle(&mut list);
            self.shuffle = list;
        }
        self.position = 0;
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

    // 查询下一曲
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

    // 查询上一曲
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
