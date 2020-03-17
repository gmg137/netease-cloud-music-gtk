//
// task.rs
// Copyright (C) 2020 gmg137 <gmg137@live.com>
// Distributed under terms of the MIT license.
//
use crate::{app::Action, model::*, musicapi::model::*, utils::*};
use async_std::{sync::Arc, task};
use futures::{channel::mpsc::Receiver, future::join_all, stream::StreamExt};
use glib::Sender;

type AsyncResult<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

#[allow(clippy::enum_variant_names)]
pub(crate) enum Task {
    DownloadPlayerImg {
        url: String,
        path: String,
        width: u32,
        high: u32,
        timeout: u64,
    },
    DownloadMusic {
        url: String,
        path: String,
        timeout: u64,
    },
    DownloadMineRecommendImage(Arc<Vec<SongList>>),
    DownloadHomeUpImage(Arc<Vec<SongList>>),
    DownloadHomeLowImage(Arc<Vec<SongList>>),
}

pub(crate) async fn actuator_loop(receiver: Receiver<Task>, sender: Sender<Action>) -> AsyncResult<()> {
    let mut receiver = receiver.fuse();
    while let Some(task) = receiver.next().await {
        match task {
            Task::DownloadMusic { url, path, timeout } => {
                download_music(&url, &path, timeout).await.ok();
            }
            Task::DownloadPlayerImg {
                url,
                path,
                width,
                high,
                timeout,
            } => {
                download_img(&url, &path, width, high, timeout).await.ok();
                sender.send(Action::RefreshPlayerImage(path)).unwrap();
            }
            Task::DownloadMineRecommendImage(rr) => {
                let sender = sender.clone();
                task::spawn(async move {
                    // 异步并行下载图片
                    let mut tasks = Vec::with_capacity(rr.len());
                    let mut l = 0;
                    for sl in rr.iter() {
                        let mut left = l;
                        let top = if l >= 4 {
                            left = l % 4;
                            l / 4
                        } else {
                            0
                        };
                        let image_path = format!("{}{}.jpg", NCM_CACHE.to_string_lossy(), &sl.id);
                        let sender_clone = sender.clone();
                        tasks.push(async move {
                            download_img(&sl.cover_img_url, &image_path, 140, 140, 100_000)
                                .await
                                .ok();
                            sender_clone
                                .send(Action::RefreshMineRecommendImage(left, top, sl.to_owned()))
                                .unwrap_or(());
                        });
                        l += 1;
                    }
                    join_all(tasks).await;
                });
            }
            Task::DownloadHomeUpImage(tsl) => {
                let sender = sender.clone();
                task::spawn(async move {
                    // 异步并行下载图片
                    let mut tasks = Vec::new();
                    let mut l = 0;
                    for sl in tsl.iter() {
                        if tasks.len() >= 8 {
                            break;
                        }
                        let mut left = l;
                        let top = if l >= 4 {
                            left = l % 4;
                            l / 4
                        } else {
                            0
                        };
                        let image_path = format!("{}{}.jpg", NCM_CACHE.to_string_lossy(), &sl.id);
                        let sender_clone = sender.clone();
                        let ssl = sl.to_owned();
                        tasks.push(async move {
                            download_img(&sl.cover_img_url, &image_path, 140, 140, 100_000)
                                .await
                                .ok();
                            sender_clone.send(Action::RefreshHomeUpImage(left, top, ssl)).unwrap();
                        });
                        l += 1;
                    }
                    join_all(tasks).await;
                });
            }
            Task::DownloadHomeLowImage(na) => {
                let sender = sender.clone();
                task::spawn(async move {
                    let mut tasks = Vec::new();
                    let mut l = 0;
                    // 异步并行下载图片
                    for sl in na.iter() {
                        let mut left = l;
                        let top = if l >= 4 {
                            left = l % 4;
                            l / 4
                        } else {
                            0
                        };
                        let image_path = format!("{}{}.jpg", NCM_CACHE.to_string_lossy(), &sl.id);
                        let sender_clone = sender.clone();
                        let ssl = sl.to_owned();
                        tasks.push(async move {
                            download_img(&sl.cover_img_url, &image_path, 130, 130, 100_000)
                                .await
                                .ok();
                            sender_clone.send(Action::RefreshHomeLowImage(left, top, ssl)).unwrap();
                        });
                        l += 1;
                    }
                    join_all(tasks).await;
                });
            }
        }
    }
    Ok(())
}
