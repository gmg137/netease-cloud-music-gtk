//! 本地歌曲的收听统计：播放次数、累计时长、收藏、最近播放、喜爱推荐。
//!
//! 数据落在 `CONFIG/local_stats.json`，结构：
//! ```json
//! { "songs": { "<id>": {plays, seconds, last_played, fav, added, path} }, "blocked": [<id>] }
//! ```
//! 全部读写都在进程内缓存，写盘做节流（累计时长每 30 秒最多写一次），
//! 收藏/拉黑这类低频操作立刻落盘。

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};

/// 单曲统计
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SongStat {
    #[serde(default)]
    pub plays: u32,
    /// 累计收听秒数
    #[serde(default)]
    pub seconds: u64,
    /// 最近一次播放时间（unix 秒）
    #[serde(default)]
    pub last_played: u64,
    #[serde(default)]
    pub fav: bool,
    /// 收藏时间（unix 秒），收藏列表按它倒序
    #[serde(default)]
    pub added: u64,
    /// 文件路径，用于歌曲已不在播放列表时重建 SongInfo
    #[serde(default)]
    pub path: String,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct Stats {
    #[serde(default)]
    songs: HashMap<String, SongStat>,
    /// 用户从「喜爱推荐」里移除的歌曲（不再推荐）
    #[serde(default)]
    blocked: Vec<u64>,
}

static STATS: Lazy<Mutex<Stats>> = Lazy::new(|| Mutex::new(load()));
static LAST_SAVE: AtomicU64 = AtomicU64::new(0);

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn file() -> PathBuf {
    crate::path::CONFIG.join("local_stats.json")
}

fn load() -> Stats {
    std::fs::read_to_string(file())
        .ok()
        .and_then(|json| serde_json::from_str::<Stats>(&json).ok())
        .unwrap_or_default()
}

fn save(stats: &Stats) {
    match serde_json::to_string(stats) {
        Ok(json) => {
            let path = file();
            let tmp = path.with_extension("json.tmp");
            if std::fs::write(&tmp, json).is_err() || std::fs::rename(&tmp, &path).is_err() {
                log::debug!("保存本地音乐统计失败");
            }
            LAST_SAVE.store(now(), Ordering::Relaxed);
        }
        Err(err) => log::debug!("序列化本地音乐统计失败: {:?}", err),
    }
}

fn entry<'a>(stats: &'a mut Stats, id: u64) -> &'a mut SongStat {
    stats.songs.entry(id.to_string()).or_default()
}

/// 立即写盘（退出前/批量修改后调用）
pub fn flush() {
    if let Ok(stats) = STATS.lock() {
        save(&stats);
    }
}

/// 一首歌开始播放：次数 +1，刷新最近播放时间
pub fn record_play(id: u64, path: &str) {
    let Ok(mut stats) = STATS.lock() else {
        return;
    };
    let stat = entry(&mut stats, id);
    stat.plays = stat.plays.saturating_add(1);
    stat.last_played = now();
    if !path.is_empty() {
        stat.path = path.to_string();
    }
    save(&stats);
}

/// 累计收听时长（秒），30 秒内的连续累加只写一次盘
pub fn record_listen(id: u64, secs: u64) {
    if secs == 0 {
        return;
    }
    let Ok(mut stats) = STATS.lock() else {
        return;
    };
    let stat = entry(&mut stats, id);
    stat.seconds = stat.seconds.saturating_add(secs);
    if stat.last_played == 0 {
        stat.last_played = now();
    }
    let last = LAST_SAVE.load(Ordering::Relaxed);
    if now().saturating_sub(last) >= 30 {
        save(&stats);
    }
}

pub fn is_fav(id: u64) -> bool {
    STATS
        .lock()
        .map(|stats| {
            stats
                .songs
                .get(&id.to_string())
                .map(|stat| stat.fav)
                .unwrap_or(false)
        })
        .unwrap_or(false)
}

/// 切换收藏，返回切换后的状态
pub fn toggle_fav(id: u64, path: &str) -> bool {
    let Ok(mut stats) = STATS.lock() else {
        return false;
    };
    let stat = entry(&mut stats, id);
    stat.fav = !stat.fav;
    let fav = stat.fav;
    if fav {
        stat.added = now();
        if !path.is_empty() {
            stat.path = path.to_string();
        }
    }
    save(&stats);
    fav
}

/// 收藏列表（按收藏时间倒序）
pub fn favorites() -> Vec<u64> {
    let Ok(stats) = STATS.lock() else {
        return Vec::new();
    };
    let mut list: Vec<(u64, u64)> = stats
        .songs
        .iter()
        .filter(|(_, stat)| stat.fav)
        .filter_map(|(id, stat)| id.parse::<u64>().ok().map(|id| (id, stat.added)))
        .collect();
    list.sort_by(|a, b| b.1.cmp(&a.1));
    list.into_iter().map(|(id, _)| id).collect()
}

/// 最近播放（次数 > 0，按时间倒序）
pub fn recent(limit: usize) -> Vec<u64> {
    let Ok(stats) = STATS.lock() else {
        return Vec::new();
    };
    let mut list: Vec<(u64, u64)> = stats
        .songs
        .iter()
        .filter(|(_, stat)| stat.plays > 0 && stat.last_played > 0)
        .filter_map(|(id, stat)| {
            id.parse::<u64>()
                .ok()
                .map(|id| (id, stat.last_played))
        })
        .collect();
    list.sort_by(|a, b| b.1.cmp(&a.1));
    list.into_iter().take(limit).map(|(id, _)| id).collect()
}

/// 喜爱推荐：收藏 + 播放次数 + 累计时长 + 近期活跃度打分，剔除被移除的条目
pub fn recommend(limit: usize) -> Vec<u64> {
    let Ok(stats) = STATS.lock() else {
        return Vec::new();
    };
    let now = now();
    let mut scored: Vec<(u64, f64)> = stats
        .songs
        .iter()
        .filter_map(|(id, stat)| {
            let id = id.parse::<u64>().ok()?;
            if stats.blocked.contains(&id) {
                return None;
            }
            // 既没收藏又没播过的不算「喜欢」
            if !stat.fav && stat.plays == 0 {
                return None;
            }
            let mut score = 0.0;
            if stat.fav {
                score += 3.0;
            }
            if stat.plays > 0 {
                score += (stat.plays as f64).min(30.0) * 0.15;
                // 平均每首听不到 20 秒，说明是跳过而不是喜欢
                let avg = stat.seconds as f64 / stat.plays as f64;
                if stat.plays >= 3 && avg < 20.0 {
                    score *= 0.3;
                }
            }
            // 累计时长，最多算 4 小时
            let minutes = stat.seconds as f64 / 60.0;
            score += minutes.min(240.0) / 60.0;
            // 近期活跃
            let days = now.saturating_sub(stat.last_played) / 86_400;
            score += match days {
                0..=2 => 1.0,
                3..=13 => 0.5,
                14..=59 => 0.2,
                _ => 0.0,
            };
            if score <= 0.0 {
                return None;
            }
            Some((id, score))
        })
        .collect();
    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    scored.into_iter().take(limit).map(|(id, _)| id).collect()
}

/// 从推荐里移除（不再推荐）
pub fn block_recommend(id: u64) {
    let Ok(mut stats) = STATS.lock() else {
        return;
    };
    if !stats.blocked.contains(&id) {
        stats.blocked.push(id);
        save(&stats);
    }
}

/// 歌曲路径（统计里记的），用于重建 SongInfo
pub fn path_of(id: u64) -> Option<String> {
    STATS
        .lock()
        .ok()
        .and_then(|stats| stats.songs.get(&id.to_string()).map(|s| s.path.clone()))
        .filter(|path| !path.is_empty())
}

