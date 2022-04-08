//
// mod.rs
// Copyright (C) 2019 gmg137 <gmg137@live.com>
// Distributed under terms of the GPLv3 license.
//

mod found;
mod home;
mod mine;
mod subpages;
use crate::{
    app::Action,
    data::MusicData,
    model::{NCM_CACHE, NCM_DATA, TOP_ID, TOP_NAME},
    musicapi::model::*,
    task::Task,
    utils::*,
};
use async_std::{
    fs,
    sync::{Arc, Mutex},
    task,
};
use found::*;
use futures::{channel::mpsc, sink::SinkExt};
use glib::Sender;
use gtk::{prelude::*, Builder, Spinner, Stack};
use home::*;
use mine::*;
use std::{cell::RefCell, rc::Rc};
use subpages::*;

#[derive(Clone)]
pub(crate) struct View {
    stack: Stack,
    main_stack: Stack,
    mine_stack: Stack,
    mine_login_center_stack: Stack,
    home: Rc<RefCell<Home>>,
    found: Rc<RefCell<Found>>,
    found_content_stack: Stack,
    mine: Rc<RefCell<Mine>>,
    subpages: Rc<RefCell<Subpages>>,
    sender: Sender<Action>,
    sender_task: mpsc::Sender<Task>,
    music_data: Arc<Mutex<MusicData>>,
}

impl View {
    pub(crate) fn new(
        builder: &Builder,
        sender: &Sender<Action>,
        sender_task: &mpsc::Sender<Task>,
        music_data: Arc<Mutex<MusicData>>,
    ) -> Rc<Self> {
        let stack: Stack = builder.get_object("stack").expect("无法获取 stack.");
        let main_stack: Stack = builder
            .get_object("main_pages_stack")
            .expect("无法获取 main_pages_stack.");
        let mine_stack: Stack = builder.get_object("mine_stack").expect("无法获取 mine_stack.");

        let glade_src = include_str!("../../ui/subpages.ui");
        let subpages_builder = Builder::from_string(glade_src);
        let subpages_stack: Stack = subpages_builder
            .get_object("subpages_stack")
            .expect("无法获取 subpages_stack.");

        let glade_src = include_str!("../../ui/home.ui");
        let home_builder = Builder::from_string(glade_src);
        let home_stack: Stack = home_builder.get_object("home_stack").expect("无法获取 home_stack.");

        let glade_src = include_str!("../../ui/found.ui");
        let found_builder = Builder::from_string(glade_src);
        let found_stack: Stack = found_builder.get_object("found_stack").expect("无法获取 found_stack.");
        let found_content_stack: Stack = found_builder
            .get_object("found_content_stack")
            .expect("无法获取 found_content_stack.");

        let spinner_stack = Stack::new();
        let spinner = Spinner::new();
        spinner.set_size_request(50, 50);
        spinner.start();
        spinner_stack.add(&spinner);
        found_content_stack.add_named(&spinner_stack, "spinner_stack");
        found_content_stack.set_visible_child_name("found_right_stack");

        let glade_src = include_str!("../../ui/mine_fm.ui");
        let mine_login_fm_builder = Builder::from_string(glade_src);
        let mine_fm_stack: Stack = mine_login_fm_builder
            .get_object("mine_fm_stack")
            .expect("无法获取 mine_fm_stack.");

        let glade_src = include_str!("../../ui/mine_list.ui");
        let mine_login_list_builder = Builder::from_string(glade_src);
        let mine_list_stack: Stack = mine_login_list_builder
            .get_object("mine_list_stack")
            .expect("无法获取 mine_list_stack.");

        let glade_src = include_str!("../../ui/mine_login.ui");
        let mine_login_builder = Builder::from_string(glade_src);
        let mine_login_stack: Stack = mine_login_builder
            .get_object("mine_login_stack")
            .expect("无法获取 mine_login_stack.");
        let mine_login_center_stack: Stack = mine_login_builder
            .get_object("mine_login_center_stack")
            .expect("无法获取 mine_login_center_stack.");

        let spinner_stack = Stack::new();
        let spinner = Spinner::new();
        spinner.set_size_request(50, 50);
        spinner.start();
        spinner_stack.add(&spinner);

        mine_login_center_stack.add_named(&mine_fm_stack, "mine_fm_stack");
        mine_login_center_stack.add_named(&mine_list_stack, "mine_list_stack");
        mine_login_center_stack.add_named(&spinner_stack, "spinner_stack");
        mine_login_center_stack.set_visible_child_name("mine_fm_stack");

        let glade_src = include_str!("../../ui/mine_not_login.ui");
        let mine_not_login_builder = Builder::from_string(glade_src);
        let mine_not_login_stack: Stack = mine_not_login_builder
            .get_object("mine_not_login_stack")
            .expect("无法获取 mine_not_login_stack.");

        mine_stack.add_named(&mine_login_stack, "mine_login_stack");
        mine_stack.add_named(&mine_not_login_stack, "mine_not_login_stack");
        mine_stack.set_visible_child_name("mine_not_login_stack");

        main_stack.add_titled(&home_stack, "home", "首页");
        main_stack.add_titled(&found_stack, "found", "发现");
        main_stack.add_titled(&mine_stack, "mine", "我的");

        let spinner_stack = Stack::new();
        let spinner = Spinner::new();
        spinner.set_size_request(50, 50);
        spinner.start();
        spinner_stack.add(&spinner);

        stack.add_named(&main_stack, "main_stack");
        stack.add_named(&subpages_stack, "subpages_stack");
        stack.add_named(&spinner_stack, "spinner_stack");
        stack.set_visible_child_name("main_stack");

        let home = Rc::new(RefCell::new(Home::new(&home_builder, sender.clone())));
        let found = Rc::new(RefCell::new(Found::new(&found_builder, sender.clone())));
        let mine = Rc::new(RefCell::new(Mine::new(
            &mine_login_builder,
            &mine_login_fm_builder,
            &mine_login_list_builder,
            sender.clone(),
            music_data.clone(),
        )));
        let subpages = Rc::new(RefCell::new(Subpages::new(&subpages_builder, sender.clone())));

        Rc::new(View {
            stack,
            main_stack,
            mine_stack,
            mine_login_center_stack,
            home,
            found,
            found_content_stack,
            mine,
            subpages,
            sender: sender.clone(),
            sender_task: sender_task.clone(),
            music_data,
        })
    }

    pub(crate) fn switch_stack_main(&self) {
        self.stack.set_visible_child(&self.main_stack);
    }

    pub(crate) fn switch_stack_subpages(&self) {
        self.stack.set_visible_child_name("subpages_stack");
    }

    pub(crate) fn switch_stack_sub(&self, id: u64, name: String, image_path: String, parse: Parse) {
        self.stack.set_visible_child_name("spinner_stack");
        let sender = self.sender.clone();
        let name_clone = name.to_owned();
        let data = self.music_data.clone();
        task::spawn(async move {
            let mut data = data.lock().await;
            match parse {
                Parse::USL => {
                    if let Ok(song_list) = data.song_list_detail(id, false).await {
                        // 发送更新子页概览
                        if sender
                            .send(Action::RefreshSubUpView(id, name_clone, image_path))
                            .is_ok()
                        {
                            sender.send(Action::RefreshSubLowView(song_list)).unwrap_or(());
                            sender.send(Action::SwitchStackSubSpinner).unwrap_or(());
                        }
                    } else {
                        sender.send(Action::ShowNotice("数据解析异常!".to_owned())).unwrap();
                    }
                }
                Parse::ALBUM => {
                    if let Ok(song_list) = data.album(id).await {
                        // 发送更新子页概览
                        if sender
                            .send(Action::RefreshSubUpView(id, name_clone, image_path))
                            .is_ok()
                        {
                            sender.send(Action::RefreshSubLowView(song_list)).unwrap_or(());
                            sender.send(Action::SwitchStackSubSpinner).unwrap_or(());
                        }
                    } else {
                        sender.send(Action::ShowNotice("数据解析异常!".to_owned())).unwrap();
                    }
                }
                _ => {}
            }
        });
        self.sender.send(Action::SwitchHeaderBar(name)).unwrap_or(());
    }

    pub(crate) fn switch_stack_search(&self, text: String) {
        self.stack.set_visible_child_name("spinner_stack");
        let sender = self.sender.clone();
        let text_clone = text.clone();
        task::spawn(async move {
            let mut data = MusicData::new().await;
            if let Ok(json) = data.search(text_clone.to_owned(), 1, 0, 50).await {
                if let Ok(song_list) = serde_json::from_str::<Vec<SongInfo>>(&json) {
                    let search_text = format!("search:{}", text_clone);
                    // 发送更新子页概览, 用以清除原始歌曲列表
                    if sender
                        .send(Action::RefreshSubUpView(0, search_text, String::new()))
                        .is_ok()
                    {
                        // 刷新搜索结果
                        sender.send(Action::RefreshSubLowView(song_list)).unwrap_or(());
                        sender.send(Action::SwitchStackSubSpinner).unwrap_or(());
                    }
                }
            } else {
                sender.send(Action::ShowNotice("数据解析异常!".to_owned())).unwrap();
            }
        });
        self.sender.send(Action::SwitchHeaderBar(text)).unwrap_or(());
    }

    // 附加搜索
    pub(crate) fn append_search(&self, text: String, num: usize) {
        let sender = self.sender.clone();
        task::spawn(async move {
            let mut data = MusicData::new().await;
            if let Ok(json) = data.search(text.to_owned(), 1, num as u16, 50).await {
                if let Ok(song_list) = serde_json::from_str::<Vec<SongInfo>>(&json) {
                    // 刷新搜索结果
                    sender.send(Action::AppendSubLowView(song_list)).unwrap_or(());
                }
            } else {
                sender.send(Action::ShowNotice("没有更多搜索结果!".to_owned())).unwrap();
            }
        });
    }

    pub(crate) fn append_sub_low_view(&self, song_list: Vec<SongInfo>) {
        self.subpages.borrow_mut().append_low_view(song_list);
    }

    pub(crate) fn update_home_view(&self, tsl: Arc<Vec<SongList>>, rr: Arc<Vec<SongList>>) {
        self.home.borrow_mut().update(tsl, rr);
    }

    pub(crate) fn update_sub_up_view(&self, id: u64, name: String, image_path: String) {
        self.subpages.borrow_mut().update_up_view(id, name, image_path);
        let sender = self.sender.clone();
        task::spawn(async move {
            let mut data = MusicData::new().await;
            if data.login_info().await.is_ok() {
                sender.send(Action::ShowSubLike(true)).unwrap_or(());
            } else {
                sender.send(Action::ShowSubLike(false)).unwrap_or(());
            }
        });
    }

    pub(crate) fn show_sub_like_button(&self, show: bool) {
        self.subpages.borrow_mut().show_like(show);
    }

    pub(crate) fn sub_like_song_list(&self) {
        let sender = self.sender.clone();
        let id = self.subpages.borrow_mut().get_song_list_id();
        let data = self.music_data.clone();
        task::spawn(async move {
            let mut data = data.lock().await;
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
        });
    }

    pub(crate) fn update_sub_low_view(&self, song_list: Vec<SongInfo>) {
        self.subpages.borrow_mut().update_low_view(song_list);
    }

    pub(crate) fn get_sub_page_data(&self) -> Option<(String, usize)> {
        self.subpages.borrow_mut().get_search_data()
    }

    pub(crate) fn update_home(&self) {
        let sender = self.sender.clone();
        let mut sender_task = self.sender_task.clone();
        task::spawn(async move {
            let mut data = MusicData::new().await;
            if let Ok(tsl) = data.top_song_list("hot", 0, 8).await {
                let tsl = Arc::new(tsl);
                sender_task.send(Task::DownloadHomeUpImage(Arc::clone(&tsl))).await.ok();
                if let Ok(na) = data.new_albums(0, 4).await {
                    let na = Arc::new(na);
                    sender_task.send(Task::DownloadHomeLowImage(Arc::clone(&na))).await.ok();
                    sender.send(Action::RefreshHomeView(tsl, na)).unwrap_or(());
                    return;
                }
                sender
                    .send(Action::RefreshHomeView(tsl, Arc::new(vec![])))
                    .unwrap_or(());
            } else {
                sender.send(Action::ShowNotice("数据解析异常!".to_owned())).unwrap();
            }
        });
    }

    pub(crate) fn set_home_up_image(&self, left: i32, top: i32, sl: SongList) {
        self.home.borrow_mut().set_up_image(left, top, sl);
    }

    pub(crate) fn set_home_low_image(&self, left: i32, top: i32, sl: SongList) {
        self.home.borrow_mut().set_low_image(left, top, sl);
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

    pub(crate) fn found_content_switch_stack_right(&self) {
        self.found_content_stack.set_visible_child_name("found_right_stack");
    }

    pub(crate) fn update_found_view_data(&self, row_id: u8) {
        let sender = self.sender.clone();
        let lid = TOP_ID.get(&row_id).unwrap();
        let title = TOP_NAME.get(&row_id).unwrap();
        self.found_content_stack.set_visible_child_name("spinner_stack");
        task::spawn(async move {
            let mut data = MusicData::new().await;
            if let Ok(song_list) = data.song_list_detail(*lid, false).await {
                sender
                    .send(Action::RefreshFoundView(song_list, (*title).to_string()))
                    .unwrap_or(());
                sender.send(Action::SwitchStackFoundSpinner).unwrap_or(());
            } else {
                sender.send(Action::ShowNotice("数据解析异常!".to_owned())).unwrap();
            }
        });
    }

    pub(crate) fn play_found(&self) {
        self.found.borrow_mut().play_all();
    }

    #[allow(unused_variables)]
    pub(crate) fn mine_init(&self) {
        self.sender.send(Action::MineShowNotLogin).unwrap_or(());
        let sender = self.sender.clone();
        let mut sender_task = self.sender_task.clone();
        let data = self.music_data.clone();
        task::spawn(async move {
            let mut data = data.lock().await;
            // 判断是否已经登录
            if let Ok(login_info) = data.login_info().await {
                sender.send(Action::MineShowLogin).unwrap_or(());
                sender.send(Action::MineShowFm).unwrap_or(());
                if let Ok(vsl) = data.user_song_list(login_info.uid, 0, 50).await {
                    sender.send(Action::RefreshMineSidebar(vsl)).unwrap_or(());
                }
                if let Ok(rr) = data.recommend_resource().await {
                    let rr = Arc::new(rr);
                    sender_task
                        .send(Task::DownloadMineRecommendImage(Arc::clone(&rr)))
                        .await
                        .ok();
                    sender.send(Action::RefreshMineRecommendView(rr)).unwrap_or(());
                    return;
                }
            }
        });
    }

    pub(crate) fn refresh_mine_recommend_image(&self, left: i32, top: i32, sl: SongList) {
        self.mine.borrow_mut().fmview.set_recommend_image(left, top, sl);
    }

    pub(crate) fn mine_switch_not_login(&self) {
        self.mine_stack.set_visible_child_name("mine_not_login_stack");
    }

    pub(crate) fn mine_switch_login(&self) {
        self.mine_stack.set_visible_child_name("mine_login_stack");
    }

    pub(crate) fn mine_login_switch_fm(&self) {
        self.mine_login_center_stack.set_visible_child_name("mine_fm_stack");
    }

    pub(crate) fn mine_login_switch_list(&self) {
        self.mine_login_center_stack.set_visible_child_name("mine_list_stack");
    }

    pub(crate) fn refresh_fm_player_list(&self) {
        let sender = self.sender.clone();
        let data = self.music_data.clone();
        task::spawn(async move {
            let mut data = data.lock().await;
            if let Ok(vsi) = data.personal_fm().await {
                // 提取歌曲 id 列表
                if !vsi.is_empty() {
                    let mut api = MusicData::new().await;
                    // 创建播放列表
                    create_player_list(&mut api, &vsi, sender.clone(), false, true)
                        .await
                        .ok();
                    // 下载专辑图片
                    let image_path = format!("{}{}.jpg", NCM_CACHE.to_string_lossy(), &vsi[0].id);
                    download_img(&vsi[0].pic_url, &image_path, 130, 130, 100_000).await.ok();
                    if sender.send(Action::RefreshMineFm(vsi[0].to_owned())).is_ok() {
                        sender.send(Action::PlayerFm).unwrap_or(());
                    }
                }
            } else {
                sender.send(Action::ShowNotice("获取 FM 歌单失败!".to_owned())).unwrap();
            }
        });
    }

    pub(crate) fn update_mine_sidebar(&self, vsl: Vec<SongList>) {
        self.mine.borrow_mut().update_sidebar(vsl);
    }

    pub(crate) fn update_mine_fm(&self, song_info: SongInfo) {
        self.mine.borrow_mut().fmview.update_fm_view(&song_info);
        self.mine.borrow_mut().fmview.set_now_play(song_info);
    }

    pub(crate) fn update_mine_recommend(&self, rr: Arc<Vec<SongList>>) {
        self.mine.borrow_mut().fmview.update_recommend_view(rr);
    }

    pub(crate) fn update_mine_up_view(&self, title: String) {
        let sidebar_id = self.mine.borrow_mut().get_selected_row_id();
        self.mine.borrow_mut().listview.update_up_view(title, sidebar_id);
    }

    pub(crate) fn update_mine_low_view(&self, song_list: Vec<SongInfo>) {
        self.mine.borrow_mut().listview.update_low_view(song_list);
    }

    pub(crate) fn update_mine_view(&self, song_list: Vec<SongInfo>, title: String) {
        self.update_mine_up_view(title);
        self.update_mine_low_view(song_list);
    }

    #[allow(unused_variables)]
    pub(crate) fn update_mine_view_data(&self, row_id: i32, refresh: bool) {
        let mut row_id = row_id as usize;
        if row_id == 0 {
            self.mine_login_switch_fm();
            if self.mine.borrow().fmview.get_now_play().is_none() {
                let sender = self.sender.clone();
                let data = self.music_data.clone();
                task::spawn(async move {
                    let mut data = data.lock().await;
                    if let Ok(vsi) = data.personal_fm().await {
                        // 提取歌曲 id 列表
                        let song_id_list = vsi.iter().map(|si| si.id).collect::<Vec<u64>>();
                        if !vsi.is_empty() {
                            let mut api = MusicData::new().await;
                            // 创建播放列表
                            create_player_list(&mut api, &vsi, sender.clone(), false, true)
                                .await
                                .ok();
                            // 下载专辑图片
                            let image_path = format!("{}{}.jpg", NCM_CACHE.to_string_lossy(), &vsi[0].id);
                            download_img(&vsi[0].pic_url, &image_path, 130, 130, 100_000).await.ok();
                            sender.send(Action::RefreshMineFm(vsi[0].to_owned())).unwrap_or(());
                        }
                    } else {
                        sender.send(Action::ShowNotice("获取 FM 歌单失败!".to_owned())).unwrap();
                    }
                });
            }
            return;
        } else {
            self.mine_login_switch_list();
            self.mine_login_center_stack.set_visible_child_name("spinner_stack");
        }
        let sender = self.sender.clone();
        let data = self.music_data.clone();
        task::spawn(async move {
            let mut data = data.lock().await;
            if row_id == 1 {
                if let Ok(song_list) = data.cloud_disk(refresh).await {
                    sender
                        .send(Action::RefreshMineView(song_list, "音乐云盘".to_owned()))
                        .unwrap_or(());
                    sender.send(Action::SwitchStackMineSpinner).unwrap_or(());
                } else {
                    sender.send(Action::ShowNotice("数据解析异常!".to_owned())).unwrap();
                }
            } else if row_id == 2 {
                if let Ok(song_list) = data.recommend_songs().await {
                    sender
                        .send(Action::RefreshMineView(song_list, "每日歌曲推荐".to_owned()))
                        .unwrap_or(());
                    sender.send(Action::SwitchStackMineSpinner).unwrap_or(());
                } else {
                    sender.send(Action::ShowNotice("数据解析异常!".to_owned())).unwrap();
                }
            } else {
                row_id -= 3;
                if let Ok(login_info) = data.login_info().await {
                    if let Ok(user_song_list) = &data.user_song_list(login_info.uid, 0, 50).await {
                        if let Ok(song_list) = data.song_list_detail(user_song_list[row_id].id, refresh).await {
                            sender
                                .send(Action::RefreshMineView(
                                    song_list,
                                    user_song_list[row_id].name.to_owned(),
                                ))
                                .unwrap_or(());
                            sender.send(Action::SwitchStackMineSpinner).unwrap_or(());
                        } else {
                            sender.send(Action::ShowNotice("数据解析异常!".to_owned())).unwrap();
                        }
                    }
                }
            }
        });
    }

    pub(crate) fn update_mine_current_view_data(&self) {
        let row_id = self.mine.borrow().get_selected_row_id();
        self.update_mine_view_data(row_id, true);
    }

    pub(crate) fn update_like_song_list(&self) {
        if self.mine.borrow().get_selected_row_id() == 2 {
            self.update_mine_view_data(2, false);
        }
    }

    pub(crate) fn dis_like_song_list(&self) {
        let mut row_id = self.mine.borrow().get_selected_row_id();
        if row_id > 3 {
            let sender = self.sender.clone();
            let data = self.music_data.clone();
            task::spawn(async move {
                let mut data = data.lock().await;
                row_id -= 3;
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
            });
        }
    }

    pub(crate) fn play_mine(&self) {
        self.mine.borrow_mut().listview.play_all();
    }

    pub(crate) fn play_fm(&self) {
        self.mine.borrow_mut().fmview.play_fm();
    }

    pub(crate) fn like_fm(&self) {
        if let Some(si) = self.mine.borrow_mut().fmview.get_now_play() {
            let sender = self.sender.clone();
            let data = self.music_data.clone();
            task::spawn(async move {
                let mut data = data.lock().await;
                if data.like(true, si.id).await {
                    sender.send(Action::ShowNotice("收藏成功!".to_owned())).unwrap();
                } else {
                    sender.send(Action::ShowNotice("收藏失败!".to_owned())).unwrap();
                }
            });
        }
    }

    pub(crate) fn dislike_fm(&self) {
        if let Some(si) = self.mine.borrow_mut().fmview.get_now_play() {
            let data = self.music_data.clone();
            task::spawn(async move {
                let mut data = data.lock().await;
                data.fm_trash(si.id).await;
            });
        }
    }

    pub(crate) fn switch_fm_play(&self) {
        self.mine.borrow_mut().fmview.switch_play();
    }

    pub(crate) fn switch_fm_pause(&self) {
        self.mine.borrow_mut().fmview.switch_pause();
    }

    pub(crate) fn set_fm_image(&self, path: String) {
        self.mine.borrow_mut().fmview.set_fm_image(path);
    }

    pub(crate) fn cancel_collection(&self) {
        if let Some(id) = self.mine.borrow_mut().listview.get_song_id() {
            let sender = self.sender.clone();
            let data = self.music_data.clone();
            task::spawn(async move {
                let mut data = data.lock().await;
                data.like(false, id).await;
                sender.send(Action::RefreshMineCurrentView()).unwrap_or(());
            });
        }
    }
}
