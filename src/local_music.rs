//! 本地音乐支持：把本地音频文件映射成 `SongInfo` 播放。
//!
//! 约定：`song_url` 以 `file://` 开头即为本地歌曲；`id` 用路径哈希并置最高位，
//! 保证与网易云歌曲 id 不会冲突。本地歌曲不请求接口，歌词优先读同名 `.lrc`。

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use lofty::file::{AudioFile, TaggedFile, TaggedFileExt};
use lofty::prelude::Accessor;
use ncm_api::{SongCopyright, SongInfo, SongQualityState};

/// 本地歌曲 id 的最高位标记
pub const LOCAL_ID_FLAG: u64 = 1 << 63;

const AUDIO_EXTS: [&str; 9] = [
    "mp3", "flac", "m4a", "aac", "ogg", "opus", "wav", "wv", "ape",
];

pub fn is_audio_file(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| AUDIO_EXTS.contains(&ext.to_lowercase().as_str()))
        .unwrap_or(false)
}

/// 是否本地歌曲
pub fn is_local(song_url: &str) -> bool {
    song_url.starts_with("file://")
}

pub fn is_local_id(id: u64) -> bool {
    id & LOCAL_ID_FLAG != 0
}

pub fn local_file_of(song_url: &str) -> Option<PathBuf> {
    song_url.strip_prefix("file://").map(PathBuf::from)
}

/// 同名 `.lrc` 歌词文件
pub fn lyric_file_of(path: &Path) -> Option<PathBuf> {
    let lrc = path.with_extension("lrc");
    lrc.is_file().then_some(lrc)
}

/// 解析 `.lrc` 文本为 (微秒, 文本) 列表，支持一行多个时间戳
pub fn parse_lrc(content: &str) -> Vec<(u64, String)> {
    let mut lines = Vec::new();
    for raw in content.lines() {
        let mut stamps = Vec::new();
        let mut rest = raw;
        while let Some(start) = rest.find('[') {
            let Some(end) = rest[start..].find(']') else {
                break;
            };
            let stamp = &rest[start + 1..start + end];
            if let Some(usec) = parse_stamp(stamp) {
                stamps.push(usec);
                rest = &rest[start + end + 1..];
            } else {
                break;
            }
        }
        let text = rest.trim();
        if text.is_empty() {
            continue;
        }
        for usec in stamps {
            lines.push((usec, text.to_string()));
        }
    }
    lines.sort_by_key(|(usec, _)| *usec);
    lines
}

fn parse_stamp(stamp: &str) -> Option<u64> {
    let (min, rest) = stamp.split_once(':')?;
    let min: u64 = min.trim().parse().ok()?;
    let (sec, frac) = match rest.split_once('.') {
        Some((sec, frac)) => (sec, frac),
        None => (rest, ""),
    };
    let sec: u64 = sec.trim().parse().ok()?;
    let millis: u64 = if frac.is_empty() {
        0
    } else {
        let digits = &frac.chars().filter(|c| c.is_ascii_digit()).collect::<String>();
        let value: u64 = digits.parse().ok()?;
        // 1~3 位小数按毫秒补零
        match digits.len() {
            1 => value * 100,
            2 => value * 10,
            _ => value,
        }
    };
    Some((min * 60 + sec) * 1_000_000 + millis * 1000)
}

/// 本地歌曲的音质描述（例如 `FLAC 24bit/96kHz`、`MP3 320kbps`）
fn quality_map() -> &'static Mutex<HashMap<u64, String>> {
    static MAP: OnceLock<Mutex<HashMap<u64, String>>> = OnceLock::new();
    MAP.get_or_init(|| Mutex::new(HashMap::new()))
}

pub fn local_quality_text(id: u64) -> Option<String> {
    quality_map().lock().ok()?.get(&id).cloned()
}

/// 取本地歌曲的格式描述；内存里没有时（例如重启后从播放列表恢复）重新解析文件
pub fn local_quality_text_for(song_info: &SongInfo) -> Option<String> {
    if !is_local(&song_info.song_url) {
        return None;
    }
    if let Some(text) = local_quality_text(song_info.id) {
        return Some(text);
    }
    let path = local_file_of(&song_info.song_url)?;
    if !path.is_file() {
        return None;
    }
    let (text, _duration) = describe_quality(&path);
    if let Ok(mut map) = quality_map().lock() {
        map.insert(song_info.id, text.clone());
    }
    Some(text)
}

fn path_id(path: &Path) -> u64 {
    // FNV-1a：同一路径始终得到同一 id，播放列表可持久化
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in path.to_string_lossy().as_bytes() {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash | LOCAL_ID_FLAG
}

fn describe_quality(path: &Path) -> (String, u64) {
    match lofty::read_from_path(path) {
        Ok(tagged) => describe_quality_of(path, &tagged),
        Err(err) => {
            log::debug!("读取本地音频信息失败: {:?} {:?}", path, err);
            (ext_upper(path), 0)
        }
    }
}

fn ext_upper(path: &Path) -> String {
    path.extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("audio")
        .to_uppercase()
}

/// 已经读过标签文件时复用，避免同一文件读两遍
fn describe_quality_of(path: &Path, tagged: &TaggedFile) -> (String, u64) {
    let ext = ext_upper(path);
    let props = tagged.properties();
    let duration = props.duration().as_millis() as u64;

    let mut text = match path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_lowercase())
    {
        Some(ext) if ext == "mp3" => "MP3".to_string(),
        _ => ext,
    };
    if let Some(depth) = props.bit_depth() {
        let depth = depth as u32;
        if depth > 16 {
            text.push_str(&format!(" {depth}bit"));
            if let Some(rate) = props.sample_rate() {
                text.push_str(&format!("/{}kHz", rate / 1000));
            }
            return (text, duration);
        }
    }
    if let Some(rate) = props.sample_rate() {
        if props.bit_depth().is_none() {
            text.push_str(&format!(" {}kHz", rate / 1000));
        }
    }
    if let Some(bitrate) = props.audio_bitrate() {
        text.push_str(&format!(" {bitrate}kbps"));
    }
    (text, duration)
}

/// 本地歌曲封面缓存文件（从音频内嵌图提取）
pub fn local_cover_path(id: u64) -> PathBuf {
    let mut path = crate::path::CACHE.clone();
    path.push(format!("local_cover_{id}.jpg"));
    path
}

/// 从内嵌标签里提取封面，写入缓存供播放栏使用
fn extract_cover(id: u64, tagged: &TaggedFile) {
    let dst = local_cover_path(id);
    if dst.exists() {
        return;
    }
    let Some(tag) = tagged.primary_tag().or_else(|| tagged.first_tag()) else {
        return;
    };
    if let Some(picture) = tag.pictures().first() {
        if let Err(err) = std::fs::write(&dst, picture.data()) {
            log::debug!("写入本地封面失败: {:?} {:?}", dst, err);
        }
    }
}

/// 由本地文件构造 `SongInfo`；解析失败也返回（用文件名兜底）
pub fn song_info_from_path(path: &Path) -> Option<SongInfo> {
    let path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    if !path.is_file() {
        return None;
    }
    let id = path_id(&path);
    // 每个文件只解析一次标签，音质/时长/封面/元数据共用
    let tagged = lofty::read_from_path(&path).ok();
    let (quality_text, duration) = match tagged.as_ref() {
        Some(tagged) => describe_quality_of(&path, tagged),
        None => {
            log::debug!("读取本地音频信息失败: {:?}", path);
            (ext_upper(&path), 0)
        }
    };
    if let Ok(mut map) = quality_map().lock() {
        map.insert(id, quality_text);
    }

    let stem = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("未知歌曲")
        .to_string();
    let (mut name, mut singer, mut album) = (stem.clone(), "未知歌手".to_string(), "本地音乐".to_string());
    if let Some(tagged) = tagged.as_ref() {
        extract_cover(id, tagged);
        let tag = tagged.primary_tag().or_else(|| tagged.first_tag());
        if let Some(tag) = tag {
            if let Some(title) = tag.title().map(|t| t.trim().to_string()) {
                if !title.is_empty() {
                    name = title;
                }
            }
            if let Some(artist) = tag.artist().map(|a| a.trim().to_string()) {
                if !artist.is_empty() {
                    singer = artist;
                }
            }
            if let Some(tag_album) = tag.album().map(|a| a.trim().to_string()) {
                if !tag_album.is_empty() {
                    album = tag_album;
                }
            }
        }
    }
    // 没有标签时按「歌手 - 歌名」拆分文件名
    if singer == "未知歌手" && name == stem {
        if let Some((artist, title)) = stem.split_once(" - ") {
            let (artist, title) = (artist.trim(), title.trim());
            if !artist.is_empty() && !title.is_empty() {
                singer = artist.to_string();
                name = title.to_string();
            }
        }
    }

    Some(SongInfo {
        id,
        name,
        singer,
        album,
        album_id: 0,
        pic_url: String::new(),
        duration,
        song_url: format!("file://{}", path.to_string_lossy()),
        quality: SongQualityState::default(),
        copyright: SongCopyright::Free,
    })
}

/// 已记住的本地音乐目录（配置文件 `local_dirs.json`）
pub fn local_dirs() -> Vec<PathBuf> {
    let Ok(json) = std::fs::read_to_string(dirs_file()) else {
        return Vec::new();
    };
    serde_json::from_str::<Vec<String>>(&json)
        .map(|list| {
            list.into_iter()
                .map(PathBuf::from)
                .filter(|dir| dir.is_dir())
                .collect()
        })
        .unwrap_or_default()
}

fn dirs_file() -> PathBuf {
    crate::path::CONFIG.join("local_dirs.json")
}

fn save_dirs(dirs: &[PathBuf]) {
    let list: Vec<String> = dirs
        .iter()
        .map(|dir| dir.to_string_lossy().to_string())
        .collect();
    match serde_json::to_string_pretty(&list) {
        Ok(json) => {
            if let Err(err) = std::fs::write(dirs_file(), json) {
                log::debug!("保存本地音乐目录失败: {:?}", err);
            }
        }
        Err(err) => log::debug!("序列化本地音乐目录失败: {:?}", err),
    }
}

/// 记住一个音乐目录（已存在则忽略），返回是否新增
pub fn remember_dir(dir: &Path) -> bool {
    let dir = dir.canonicalize().unwrap_or_else(|_| dir.to_path_buf());
    let mut dirs = local_dirs();
    if dirs.iter().any(|item| item == &dir) {
        return false;
    }
    dirs.push(dir);
    save_dirs(&dirs);
    true
}

/// 忘记所有已记住的目录
pub fn forget_dirs() {
    let _ = std::fs::remove_file(dirs_file());
}

/// 扫描所有已记住的目录
pub fn scan_all() -> Vec<PathBuf> {
    let mut files = Vec::new();
    for dir in local_dirs() {
        files.extend(scan_dir(&dir));
    }
    files.sort();
    files.dedup();
    files
}

/// 递归扫描目录下的音频文件（按路径排序，便于稳定播放顺序）
pub fn scan_dir(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(current) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&current) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let name = path.file_name().and_then(|name| name.to_str()).unwrap_or("");
            if name.starts_with('.') {
                continue;
            }
            if path.is_dir() {
                stack.push(path);
            } else if is_audio_file(&path) {
                files.push(path);
            }
        }
    }
    files.sort();
    files
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_lrc_lines() {
        let lrc = "[00:01.50]第一行\n[00:03.00][00:05.00]重复行\n[ar:某人]\n";
        let lines = parse_lrc(lrc);
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0].0, 1_500_000);
        assert_eq!(lines[1].1, "重复行");
        assert_eq!(lines[2].0, 5_000_000);
    }
}

/// 本地曲库索引（`local_library.json`）：记录扫描到的全部本地歌曲，
/// 与播放队列解耦——从「收藏/推荐/最近播放」连续播放时队列会被替换，
/// 但曲库仍然是完整的。
static LIBRARY_CACHE: OnceLock<Mutex<Option<Vec<SongInfo>>>> = OnceLock::new();

fn library_cache() -> &'static Mutex<Option<Vec<SongInfo>>> {
    LIBRARY_CACHE.get_or_init(|| Mutex::new(None))
}

pub fn library() -> Vec<SongInfo> {
    let cache = library_cache();
    let mut guard = cache.lock().unwrap();
    if guard.is_none() {
        let list = std::fs::read_to_string(library_file())
            .ok()
            .and_then(|json| serde_json::from_str::<Vec<SongInfo>>(&json).ok())
            .unwrap_or_default();
        *guard = Some(list);
    }
    guard.clone().unwrap_or_default()
}

/// 把扫描到的歌曲并入曲库索引（按 id 去重），返回曲库总数
pub fn merge_library(songs: &[SongInfo]) -> usize {
    let cache = library_cache();
    let mut guard = cache.lock().unwrap();
    if guard.is_none() {
        *guard = Some(
            std::fs::read_to_string(library_file())
                .ok()
                .and_then(|json| serde_json::from_str::<Vec<SongInfo>>(&json).ok())
                .unwrap_or_default(),
        );
    }
    let list = guard.as_mut().unwrap();
    let mut added = 0;
    for song in songs {
        if !list.iter().any(|item| item.id == song.id) {
            list.push(song.clone());
            added += 1;
        }
    }
    let total = list.len();
    if added > 0 {
        save_library(list);
    }
    total
}

/// 丢掉曲库里文件已不存在的条目（重扫后调用）
pub fn prune_library() -> usize {
    let cache = library_cache();
    let mut guard = cache.lock().unwrap();
    if guard.is_none() {
        return 0;
    }
    let list = guard.as_mut().unwrap();
    let before = list.len();
    list.retain(|song| {
        local_file_of(&song.song_url)
            .map(|path| path.is_file())
            .unwrap_or(false)
    });
    let removed = before - list.len();
    if removed > 0 {
        save_library(list);
    }
    removed
}

/// 清空曲库索引
pub fn clear_library() {
    let cache = library_cache();
    if let Ok(mut guard) = cache.lock() {
        if let Some(list) = guard.as_mut() {
            list.clear();
        }
    }
    let _ = std::fs::remove_file(library_file());
}

fn library_file() -> PathBuf {
    crate::path::CONFIG.join("local_library.json")
}

fn save_library(list: &[SongInfo]) {
    match serde_json::to_string(list) {
        Ok(json) => {
            let path = library_file();
            let tmp = path.with_extension("json.tmp");
            if std::fs::write(&tmp, json).is_err() || std::fs::rename(&tmp, &path).is_err() {
                log::debug!("保存本地曲库索引失败");
            }
        }
        Err(err) => log::debug!("序列化本地曲库索引失败: {:?}", err),
    }
}
