#[derive(Debug, Copy, Clone)]
pub enum PlaybackStatus{
    Playing,
    Paused,
    Stopped,
}

impl PlaybackStatus {
    pub fn value(&self) -> String {
        match *self {
            PlaybackStatus::Playing => "Playing".to_string(),
            PlaybackStatus::Paused => "Paused".to_string(),
            PlaybackStatus::Stopped => "Stopped".to_string(),
        }
    }
}


#[derive(Debug, Copy, Clone)]
pub enum LoopStatus{
    None,
    Track,
    Playlist,
}

impl LoopStatus {
    pub fn value(&self) -> String {
        match *self {
            LoopStatus::None => "None".to_string(),
            LoopStatus::Track => "Track".to_string(),
            LoopStatus::Playlist => "Playlist".to_string(),
        }
    }
}
