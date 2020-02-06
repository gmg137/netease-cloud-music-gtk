//
// mod.rs
// Copyright (C) 2019 gmg137 <gmg137@live.com>
// Distributed under terms of the GPLv3 license.
//

mod found;
mod home;
mod mine;
mod subpages;
use crate::app::Action;
use crate::data::MusicData;
use crate::model::{NCM_CACHE, NCM_DATA, TOP_ID, TOP_NAME};
use crate::musicapi::model::*;
use crate::utils::*;
use async_std::{fs, task};
use crossbeam_channel::Sender;
use found::*;
use gtk::{prelude::*, Builder, Stack};
use home::*;
use mine::*;
use std::cell::RefCell;
use std::rc::Rc;
use subpages::*;

#[derive(Clone)]
pub(crate) struct View {
    stack: gtk::Stack,
    main_stack: gtk::Stack,
    subpages_stack: gtk::Stack,
    home: Rc<RefCell<Home>>,
    found: Rc<RefCell<Found>>,
    mine: Rc<RefCell<Mine>>,
    subpages: Rc<RefCell<Subpages>>,
    sender: Sender<Action>,
}

impl View {
    pub(crate) fn new(builder: &Builder, sender: &Sender<Action>) -> Rc<Self> {
        let stack: Stack = builder.get_object("stack").expect("无法获取 stack 窗口.");
        let main_stack: Stack = builder
            .get_object("stack_main_pages")
            .expect("无法获取 stack_main_pages 窗口.");
        let subpages_stack: Stack = builder
            .get_object("stack_subpages")
            .expect("无法获取 stack_subpages 窗口.");

        let home_stack: Stack = builder.get_object("stack_home").expect("无法获取 stack_home 窗口.");
        let found_stack: Stack = builder.get_object("stack_found").expect("无法获取 stack_found 窗口.");
        let mine_stack: Stack = builder.get_object("stack_mine").expect("无法获取 stack_mine 窗口.");

        main_stack.add_titled(&home_stack, "home", "首页");
        main_stack.add_titled(&found_stack, "found", "发现");
        main_stack.add_titled(&mine_stack, "mine", "我的");

        stack.add(&main_stack);
        stack.add(&subpages_stack);
        stack.set_visible_child(&main_stack);

        let home = Rc::new(RefCell::new(Home::new(builder, sender.clone())));
        let found = Rc::new(RefCell::new(Found::new(builder, sender.clone())));
        let mine = Rc::new(RefCell::new(Mine::new(builder, sender.clone())));
        let subpages = Rc::new(RefCell::new(Subpages::new(builder, sender.clone())));

        Rc::new(View {
            stack,
            main_stack,
            subpages_stack,
            home,
            found,
            mine,
            subpages,
            sender: sender.clone(),
        })
    }

    pub(crate) fn switch_stack_main(&self) {
        self.stack.set_visible_child(&self.main_stack);
    }

    pub(crate) fn switch_stack_sub(&self, id: u32, name: String, image_path: String) {
        let sender = self.sender.clone();
        let name_clone = name.to_owned();
        task::spawn(async move {
            if let Ok(mut data) = MusicData::new().await {
                if let Ok(song_list) = data.song_list_detail(id, false).await {
                    // 发送更新子页概览
                    if sender
                        .send(Action::RefreshSubUpView(id, name_clone, image_path))
                        .is_ok()
                    {
                        sender.send(Action::RefreshSubLowView(song_list)).unwrap_or(());
                    }
                } else {
                    sender.send(Action::ShowNotice("数据解析异常!".to_owned())).unwrap();
                }
            } else {
                sender.send(Action::ShowNotice("接口请求异常!".to_owned())).unwrap();
            }
        });
        self.sender.send(Action::SwitchHeaderBar(name)).unwrap_or(());
        self.stack.set_visible_child(&self.subpages_stack);
    }

    pub(crate) fn switch_stack_search(&self, text: String) {
        let sender = self.sender.clone();
        let text_clone = text.clone();
        task::spawn(async move {
            if let Ok(mut data) = MusicData::new().await {
                if let Ok(json) = data.search(text_clone, 1, 0, 50).await {
                    if let Ok(song_list) = serde_json::from_str::<Vec<SongInfo>>(&json) {
                        // 发送更新子页概览, 用以清除原始歌曲列表
                        if sender
                            .send(Action::RefreshSubUpView(0, String::new(), String::new()))
                            .is_ok()
                        {
                            // 刷新搜索结果
                            sender.send(Action::RefreshSubLowView(song_list)).unwrap_or(());
                        }
                    }
                } else {
                    sender.send(Action::ShowNotice("数据解析异常!".to_owned())).unwrap();
                }
            } else {
                sender.send(Action::ShowNotice("接口请求异常!".to_owned())).unwrap();
            }
        });
        self.sender.send(Action::SwitchHeaderBar(text)).unwrap_or(());
        self.stack.set_visible_child(&self.subpages_stack);
    }

    pub(crate) fn update_home_view(&self, tsl: Vec<SongList>, rr: Vec<SongList>) {
        self.home.borrow_mut().update(tsl, rr);
    }

    pub(crate) fn update_sub_up_view(&self, id: u32, name: String, image_path: String) {
        self.subpages.borrow_mut().update_up_view(id, name, image_path);
        let sender = self.sender.clone();
        task::spawn(async move {
            if let Ok(mut data) = MusicData::new().await {
                if data.login_info().await.is_ok() {
                    sender.send(Action::ShowSubLike(true)).unwrap_or(());
                } else {
                    sender.send(Action::ShowSubLike(false)).unwrap_or(());
                }
            } else {
                sender.send(Action::ShowNotice("接口请求异常!".to_owned())).unwrap();
            }
        });
    }

    pub(crate) fn show_sub_like_button(&self, show: bool) {
        self.subpages.borrow_mut().show_like(show);
    }

    pub(crate) fn sub_like_song_list(&self) {
        let sender = self.sender.clone();
        let id = self.subpages.borrow_mut().get_song_list_id();
        task::spawn(async move {
            if let Ok(mut data) = MusicData::new().await {
                if data.song_list_like(true, id).await {
                    fs::remove_file(format!("{}user_song_list.db", NCM_DATA.to_string_lossy()))
                        .await
                        .ok();
                    sender
                        .send(Action::ShowNotice("收藏歌单成功!".to_owned()))
                        .unwrap_or(());
                    if let Ok(login_info) = data.login_info().await {
                        if let Ok(vsl) = data.user_song_list(login_info.uid, 0, 50).await {
                            sender.send(Action::RefreshMineSidebar(vsl)).unwrap_or(());
                        }
                    }
                } else {
                    sender
                        .send(Action::ShowNotice("收藏歌单失败!".to_owned()))
                        .unwrap_or(());
                }
            } else {
                sender
                    .send(Action::ShowNotice("接口请求异常!".to_owned()))
                    .unwrap_or(());
            }
        });
    }

    pub(crate) fn update_sub_low_view(&self, song_list: Vec<SongInfo>) {
        self.subpages.borrow_mut().update_low_view(song_list);
    }

    pub(crate) fn update_home(&self) {
        let sender = self.sender.clone();
        task::spawn(async move {
            if let Ok(mut data) = MusicData::new().await {
                if let Ok(tsl) = data.top_song_list("hot", 0, 9).await {
                    // 异步并行下载图片
                    let mut tasks = Vec::with_capacity(tsl.len());
                    for sl in tsl.clone().into_iter() {
                        tasks.push(task::spawn(async move {
                            let image_path = format!("{}{}.jpg", NCM_CACHE.to_string_lossy(), &sl.id);
                            crate::utils::download_img(&sl.cover_img_url, &image_path, 210, 210)
                                .await
                                .ok();
                        }))
                    }
                    for t in tasks {
                        t.await;
                    }
                    // 判断是否已经登陆
                    if data.login_info().await.is_ok() {
                        if let Ok(rr) = data.recommend_resource().await {
                            // 异步并行下载图片
                            let mut tasks = Vec::with_capacity(rr.len());
                            for sl in rr.clone().into_iter() {
                                tasks.push(task::spawn(async move {
                                    let image_path = format!("{}{}.jpg", NCM_CACHE.to_string_lossy(), &sl.id);
                                    crate::utils::download_img(&sl.cover_img_url, &image_path, 210, 210)
                                        .await
                                        .ok();
                                }))
                            }
                            for t in tasks {
                                t.await;
                            }
                            sender
                                .send(Action::RefreshHomeView(tsl[0..8].to_owned(), rr))
                                .unwrap_or(());
                            return;
                        }
                    }
                    sender
                        .send(Action::RefreshHomeView(tsl[0..8].to_owned(), vec![]))
                        .unwrap_or(());
                } else {
                    sender.send(Action::ShowNotice("数据解析异常!".to_owned())).unwrap();
                }
            } else {
                sender.send(Action::ShowNotice("接口请求异常!".to_owned())).unwrap();
            }
        });
    }

    pub(crate) fn play_subpages(&self) {
        self.subpages.borrow_mut().play_all();
    }

    pub(crate) fn update_found_up_view(&self, title: String) {
        self.found.borrow_mut().update_up_view(title);
    }

    pub(crate) fn update_found_low_view(&self, song_list: Vec<SongInfo>) {
        self.found.borrow_mut().update_low_view(song_list);
    }

    pub(crate) fn update_found_view(&self, song_list: Vec<SongInfo>, title: String) {
        self.update_found_up_view(title);
        self.update_found_low_view(song_list);
    }

    pub(crate) fn update_found_view_data(&self, row_id: u8) {
        let sender = self.sender.clone();
        let lid = TOP_ID.get(&row_id).unwrap();
        let title = TOP_NAME.get(&row_id).unwrap();
        task::spawn(async move {
            if let Ok(mut data) = MusicData::new().await {
                if let Ok(song_list) = data.song_list_detail(*lid, false).await {
                    sender
                        .send(Action::RefreshFoundView(song_list, title.to_string()))
                        .unwrap_or(());
                } else {
                    sender.send(Action::ShowNotice("数据解析异常!".to_owned())).unwrap();
                }
            } else {
                sender.send(Action::ShowNotice("接口请求异常!".to_owned())).unwrap();
            }
        });
    }

    pub(crate) fn play_found(&self) {
        self.found.borrow_mut().play_all();
    }

    #[allow(unused_variables)]
    pub(crate) fn mine_init(&self) {
        self.sender.send(Action::MineHideAll).unwrap_or(());
        let sender = self.sender.clone();
        task::spawn(async move {
            if let Ok(mut data) = MusicData::new().await {
                if let Ok(login_info) = data.login_info().await {
                    sender.send(Action::MineShowFm).unwrap_or(());
                    if let Ok(vsl) = data.user_song_list(login_info.uid, 0, 50).await {
                        sender.send(Action::RefreshMineSidebar(vsl)).unwrap_or(());
                    }
                }
            } else {
                sender.send(Action::ShowNotice("接口请求异常!".to_owned())).unwrap();
            }
        });
    }

    pub(crate) fn mine_hide_all(&self) {
        self.mine.borrow_mut().hide_all();
    }

    pub(crate) fn mine_show_fm(&self) {
        self.mine.borrow_mut().show_fm();
    }

    pub(crate) fn refresh_fm_player_list(&self) {
        let sender = self.sender.clone();
        task::spawn(async move {
            if let Ok(mut data) = MusicData::new().await {
                if let Ok(vsi) = data.personal_fm().await {
                    // 提取歌曲 id 列表
                    let song_id_list = vsi.iter().map(|si| si.id).collect::<Vec<u32>>();
                    if let Ok(si) = data.songs_detail(&song_id_list).await {
                        if !vsi.is_empty() {
                            // 创建播放列表
                            create_player_list(&si, sender.clone(), false).await.ok();
                            // 下载专辑图片
                            let image_path = format!("{}/{}.jpg", NCM_CACHE.to_string_lossy(), &si[0].id);
                            download_img(&si[0].pic_url, &image_path, 210, 210).await.ok();
                            if sender.send(Action::RefreshMineFm(si[0].to_owned())).is_ok() {
                                sender.send(Action::PlayerFm).unwrap_or(());
                            }
                        }
                    }
                } else {
                    sender.send(Action::ShowNotice("获取 FM 歌单失败!".to_owned())).unwrap();
                }
            } else {
                sender.send(Action::ShowNotice("接口请求异常!".to_owned())).unwrap();
            }
        });
    }

    pub(crate) fn update_mine_sidebar(&self, vsl: Vec<SongList>) {
        self.mine.borrow_mut().update_sidebar(vsl);
    }

    pub(crate) fn update_mine_fm(&self, song_info: SongInfo) {
        self.mine.borrow_mut().update_fm_view(&song_info);
        self.mine.borrow_mut().set_now_play(song_info);
    }

    pub(crate) fn update_mine_up_view(&self, title: String) {
        self.mine.borrow_mut().update_up_view(title);
    }

    pub(crate) fn update_mine_low_view(&self, song_list: Vec<SongInfo>) {
        self.mine.borrow_mut().update_low_view(song_list);
    }

    pub(crate) fn update_mine_view(&self, song_list: Vec<SongInfo>, title: String) {
        self.update_mine_up_view(title);
        self.update_mine_low_view(song_list);
    }

    #[allow(unused_variables)]
    pub(crate) fn update_mine_view_data(&self, row_id: i32, refresh: bool) {
        let mut row_id = row_id as usize;
        if row_id == 0 {
            self.mine.borrow_mut().show_fm();
            if self.mine.borrow().get_now_play().is_none() {
                let sender = self.sender.clone();
                task::spawn(async move {
                    if let Ok(mut data) = MusicData::new().await {
                        if let Ok(vsi) = data.personal_fm().await {
                            // 提取歌曲 id 列表
                            let song_id_list = vsi.iter().map(|si| si.id).collect::<Vec<u32>>();
                            if let Ok(si) = data.songs_detail(&song_id_list).await {
                                if !vsi.is_empty() {
                                    // 创建播放列表
                                    create_player_list(&si, sender.clone(), false).await.ok();
                                    // 下载专辑图片
                                    let image_path = format!("{}/{}.jpg", NCM_CACHE.to_string_lossy(), &si[0].id);
                                    download_img(&si[0].pic_url, &image_path, 210, 210).await.ok();
                                    sender.send(Action::RefreshMineFm(si[0].to_owned())).unwrap_or(());
                                }
                            }
                        } else {
                            sender.send(Action::ShowNotice("获取 FM 歌单失败!".to_owned())).unwrap();
                        }
                    } else {
                        sender.send(Action::ShowNotice("接口请求异常!".to_owned())).unwrap();
                    }
                });
            }
            return;
        } else {
            self.mine.borrow_mut().hide_fm();
        }
        let sender = self.sender.clone();
        task::spawn(async move {
            if let Ok(mut data) = MusicData::new().await {
                if row_id == 1 {
                    if let Ok(song_list) = data.recommend_songs(refresh).await {
                        sender
                            .send(Action::RefreshMineView(song_list, "每日歌曲推荐".to_owned()))
                            .unwrap_or(());
                    } else {
                        sender.send(Action::ShowNotice("数据解析异常!".to_owned())).unwrap();
                    }
                } else {
                    row_id -= 2;
                    if let Ok(login_info) = data.login_info().await {
                        if let Ok(user_song_list) = &data.user_song_list(login_info.uid, 0, 50).await {
                            if let Ok(song_list) = data.song_list_detail(user_song_list[row_id].id, refresh).await {
                                sender
                                    .send(Action::RefreshMineView(
                                        song_list,
                                        user_song_list[row_id].name.to_owned(),
                                    ))
                                    .unwrap_or(());
                            } else {
                                sender.send(Action::ShowNotice("数据解析异常!".to_owned())).unwrap();
                            }
                        }
                    }
                }
            } else {
                sender.send(Action::ShowNotice("接口请求异常!".to_owned())).unwrap();
            }
        });
    }

    pub fn update_mine_current_view_data(&self) {
        let row_id = self.mine.borrow().get_selected_row_id();
        self.update_mine_view_data(row_id, true);
    }

    pub fn update_like_song_list(&self) {
        if self.mine.borrow().get_selected_row_id() == 2 {
            self.update_mine_view_data(2, false);
        }
    }

    pub(crate) fn dis_like_song_list(&self) {
        let mut row_id = self.mine.borrow().get_selected_row_id();
        if row_id > 2 {
            let sender = self.sender.clone();
            task::spawn(async move {
                if let Ok(mut data) = MusicData::new().await {
                    row_id -= 2;
                    if let Ok(login_info) = data.login_info().await {
                        if let Ok(sl) = &data.user_song_list(login_info.uid, 0, 50).await {
                            if data.song_list_like(false, sl[row_id as usize].id).await {
                                fs::remove_file(format!("{}user_song_list.db", NCM_DATA.to_string_lossy()))
                                    .await
                                    .ok();
                                sender.send(Action::ShowNotice("已删除歌单!".to_owned())).unwrap_or(());
                                if let Ok(vsl) = data.user_song_list(login_info.uid, 0, 50).await {
                                    sender.send(Action::RefreshMineSidebar(vsl)).unwrap_or(());
                                }
                            } else {
                                sender
                                    .send(Action::ShowNotice("删除歌单失败!".to_owned()))
                                    .unwrap_or(());
                            }
                        }
                    }
                } else {
                    sender.send(Action::ShowNotice("接口请求异常!".to_owned())).unwrap();
                }
            });
        }
    }

    pub(crate) fn play_mine(&self) {
        self.mine.borrow_mut().play_all();
    }

    pub(crate) fn play_fm(&self) {
        self.mine.borrow_mut().play_fm();
    }

    pub(crate) fn like_fm(&self) {
        if let Some(si) = self.mine.borrow_mut().get_now_play() {
            let sender = self.sender.clone();
            task::spawn(async move {
                if let Ok(mut data) = MusicData::new().await {
                    if data.like(true, si.id).await {
                        sender.send(Action::ShowNotice("收藏成功!".to_owned())).unwrap();
                    } else {
                        sender.send(Action::ShowNotice("收藏失败!".to_owned())).unwrap();
                    }
                } else {
                    sender.send(Action::ShowNotice("接口请求异常!".to_owned())).unwrap();
                }
            });
        }
    }

    pub(crate) fn dislike_fm(&self) {
        if let Some(si) = self.mine.borrow_mut().get_now_play() {
            let sender = self.sender.clone();
            task::spawn(async move {
                if let Ok(mut data) = MusicData::new().await {
                    data.fm_trash(si.id).await;
                } else {
                    sender.send(Action::ShowNotice("接口请求异常!".to_owned())).unwrap();
                }
            });
        }
    }

    pub(crate) fn cancel_collection(&self) {
        if let Some(id) = self.mine.borrow_mut().get_song_id() {
            let sender = self.sender.clone();
            task::spawn(async move {
                if let Ok(mut data) = MusicData::new().await {
                    data.like(false, id).await;
                    sender.send(Action::RefreshMineCurrentView()).unwrap_or(());
                }
            });
        }
    }
}
