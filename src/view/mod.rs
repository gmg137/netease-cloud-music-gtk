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
use crate::musicapi::model::*;
use crate::TOP_ID;
use crossbeam_channel::Sender;
use found::*;
use gtk::{prelude::*, Builder, Stack};
use home::*;
use mine::*;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::thread::spawn;
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
    data: Arc<Mutex<MusicData>>,
}

impl View {
    pub(crate) fn new(
        builder: &Builder,
        sender: &Sender<Action>,
        data: Arc<Mutex<MusicData>>,
    ) -> Rc<Self> {
        let stack: Stack = builder
            .get_object("stack")
            .expect("无法获取 stack 窗口.");
        let main_stack: Stack = builder
            .get_object("stack_main_pages")
            .expect("无法获取 stack_main_pages 窗口.");
        let subpages_stack: Stack = builder
            .get_object("stack_subpages")
            .expect("无法获取 stack_subpages 窗口.");

        let home_stack: Stack = builder
            .get_object("stack_home")
            .expect("无法获取 stack_home 窗口.");
        let found_stack: Stack = builder
            .get_object("stack_found")
            .expect("无法获取 stack_found 窗口.");
        let mine_stack: Stack = builder
            .get_object("stack_mine")
            .expect("无法获取 stack_mine 窗口.");

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
            data,
        })
    }

    pub(crate) fn switch_stack_main(&self) {
        self.stack.set_visible_child(&self.main_stack);
    }

    pub(crate) fn switch_stack_sub(&self, id: u32, name: String, image_path: String) {
        // 发送更新子页概览
        self.sender
            .send(Action::RefreshSubUpView(name.to_owned(), image_path))
            .unwrap_or(());
        let sender = self.sender.clone();
        let data = self.data.clone();
        spawn(move || {
            let mut data = data.lock().unwrap();
            if let Some(song_list) = data.song_list_detail(id) {
                sender
                    .send(Action::RefreshSubLowView(song_list))
                    .unwrap_or(());
            } else {
                sender
                    .send(Action::ShowNotice("网络异常!".to_owned()))
                    .unwrap();
            }
        });
        self.sender
            .send(Action::SwitchHeaderBar(name))
            .unwrap_or(());
        self.stack.set_visible_child(&self.subpages_stack);
    }

    pub(crate) fn switch_stack_search(&self, text: String) {
        // 发送更新子页概览
        self.sender
            .send(Action::RefreshSubUpView(String::new(), String::new()))
            .unwrap_or(());
        let sender = self.sender.clone();
        let data = self.data.clone();
        let text_clone = text.clone();
        spawn(move || {
            let mut data = data.lock().unwrap();
            if let Some(json) = data.search(text_clone, 1, 0, 50) {
                if let Ok(song_list) = serde_json::from_str::<Vec<SongInfo>>(&json) {
                    sender
                        .send(Action::RefreshSubLowView(song_list))
                        .unwrap_or(());
                }
            } else {
                sender
                    .send(Action::ShowNotice("网络异常!".to_owned()))
                    .unwrap();
            }
        });
        self.sender
            .send(Action::SwitchHeaderBar(text))
            .unwrap_or(());
        self.stack.set_visible_child(&self.subpages_stack);
    }

    pub(crate) fn update_home_view(&self, tsl: Vec<SongList>, rr: Vec<SongList>) {
        self.home.borrow_mut().update(tsl, rr);
    }

    pub(crate) fn update_sub_up_view(&self, name: String, image_path: String) {
        self.subpages.borrow_mut().update_up_view(name, image_path);
    }

    pub(crate) fn update_sub_low_view(&self, song_list: Vec<SongInfo>) {
        self.subpages.borrow_mut().update_low_view(song_list);
    }

    pub(crate) fn update_home(&self) {
        let sender = self.sender.clone();
        let data = self.data.clone();
        spawn(move || {
            let mut data = data.lock().unwrap();
            if let Some(tsl) = data.top_song_list("hot", 0, 9) {
                if data.login {
                    if let Some(rr) = data.recommend_resource() {
                        if let Some(rr) = data.recommend_resource() {
                            sender
                                .send(Action::RefreshHomeView(tsl[0..8].to_owned(), rr))
                                .unwrap_or(());
                            return;
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
                sender
                    .send(Action::ShowNotice("网络异常!".to_owned()))
                    .unwrap();
            }
        });
    }

    pub(crate) fn play_subpages(&self) {
        self.subpages.borrow_mut().play_all();
    }

    pub(crate) fn update_found_up_view(&self, row_id: u8) {
        self.found.borrow_mut().update_up_view(row_id);
    }

    pub(crate) fn update_found_low_view(&self, song_list: Vec<SongInfo>) {
        self.found.borrow_mut().update_low_view(song_list);
    }

    pub(crate) fn update_found_view(&self, song_list: Vec<SongInfo>) {
        self.update_found_low_view(song_list);
    }

    pub(crate) fn update_found_view_data(&self, row_id: u8) {
        self.update_found_up_view(row_id);
        let sender = self.sender.clone();
        let data = self.data.clone();
        let lid = TOP_ID.get(&row_id).unwrap();
        spawn(move || {
            let mut data = data.lock().unwrap();
            if let Some(song_list) = data.song_list_detail(*lid) {
                sender
                    .send(Action::RefreshFoundView(song_list))
                    .unwrap_or(());
            } else {
                sender
                    .send(Action::ShowNotice("网络异常!".to_owned()))
                    .unwrap();
            }
        });
    }

    pub(crate) fn play_found(&self) {
        self.found.borrow_mut().play_all();
    }

    pub(crate) fn mine_init(&self) {
        let data = self.data.clone();
        let data = data.lock().unwrap();
        if data.login {
            self.mine.borrow_mut().show_fm();
            let sender = self.sender.clone();
            let data = self.data.clone();
            spawn(move || {
                let mut data = data.lock().unwrap();
                if let Some(login_info) = data.login_info() {
                    if let Some(vsl) = data.user_song_list(login_info.uid, 0, 50) {
                        sender.send(Action::RefreshMineSidebar(vsl)).unwrap_or(());
                    }
                }
            });
        } else {
            self.mine.borrow_mut().hide_all();
        }
    }

    pub(crate) fn refresh_fm_player_list(&self) {
        let sender = self.sender.clone();
        let data = self.data.clone();
        spawn(move || {
            let mut data = data.lock().unwrap();
            if let Some(vsi) = data.personal_fm() {
                // 提取歌曲 id 列表
                let song_id_list = vsi.iter().map(|si| si.id).collect::<Vec<u32>>();
                if let Some(si) = data.songs_detail(&song_id_list) {
                    if !vsi.is_empty() {
                        crate::utils::create_player_list(&si, sender.clone(), false);
                        if sender.send(Action::RefreshMineFm(si[0].to_owned())).is_ok() {
                            sender.send(Action::PlayerFm).unwrap_or(());
                        }
                    }
                }
            } else {
                sender
                    .send(Action::ShowNotice("获取 FM 歌单失败!".to_owned()))
                    .unwrap();
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

    pub(crate) fn update_mine_view_data(&self, row_id: i32) {
        let mut row_id = row_id as usize;
        if row_id == 0 {
            self.mine.borrow_mut().show_fm();
            if self.mine.borrow().get_now_play().is_none() {
                let sender = self.sender.clone();
                let data = self.data.clone();
                spawn(move || {
                    let mut data = data.lock().unwrap();
                    if let Some(vsi) = data.personal_fm() {
                        // 提取歌曲 id 列表
                        let song_id_list = vsi.iter().map(|si| si.id).collect::<Vec<u32>>();
                        if let Some(si) = data.songs_detail(&song_id_list) {
                            if !vsi.is_empty() {
                                crate::utils::create_player_list(&si, sender.clone(), false);
                                sender
                                    .send(Action::RefreshMineFm(si[0].to_owned()))
                                    .unwrap_or(());
                            }
                        }
                    } else {
                        sender
                            .send(Action::ShowNotice("获取 FM 歌单失败!".to_owned()))
                            .unwrap();
                    }
                });
            }
            return;
        } else {
            self.mine.borrow_mut().hide_fm();
        }
        let sender = self.sender.clone();
        let data = self.data.clone();
        spawn(move || {
            let mut data = data.lock().unwrap();
            if row_id == 1 {
                sender
                    .send(Action::RefreshMineView(
                        vec![],
                        "每日歌曲推荐".to_owned(),
                    ))
                    .unwrap_or(());
                if let Some(song_list) = data.recommend_songs() {
                    sender
                        .send(Action::RefreshMineView(
                            song_list,
                            "每日歌曲推荐".to_owned(),
                        ))
                        .unwrap_or(());
                } else {
                    sender
                        .send(Action::ShowNotice("网络异常!".to_owned()))
                        .unwrap();
                }
            } else {
                row_id -= 2;
                let uid = data.login_info().unwrap().uid;
                let sl = &data.user_song_list(uid, 0, 50).unwrap()[row_id];
                if let Some(song_list) = data.song_list_detail(sl.id) {
                    sender
                        .send(Action::RefreshMineView(song_list, sl.name.to_owned()))
                        .unwrap_or(());
                } else {
                    sender
                        .send(Action::ShowNotice("网络异常!".to_owned()))
                        .unwrap();
                }
            }
        });
    }

    pub(crate) fn play_mine(&self) {
        self.mine.borrow_mut().play_all();
    }

    pub(crate) fn play_fm(&self) {
        self.mine.borrow_mut().play_fm();
    }

    pub(crate) fn like_fm(&self) {
        if let Some(si) = self.mine.borrow_mut().get_now_play() {
            let data = self.data.clone();
            spawn(move || {
                let mut data = data.lock().unwrap();
                data.like(true, si.id);
            });
        }
    }

    pub(crate) fn dislike_fm(&self) {
        if let Some(si) = self.mine.borrow_mut().get_now_play() {
            let data = self.data.clone();
            spawn(move || {
                let mut data = data.lock().unwrap();
                data.fm_trash(si.id);
            });
        }
    }

    pub(crate) fn cancel_collection(&self) {
        if let Some(id) = self.mine.borrow_mut().get_song_id() {
            let data = self.data.clone();
            let sender = self.sender.clone();
            spawn(move || {
                let mut data = data.lock().unwrap();
                data.like(false, id);
                sender.send(Action::RefreshMineViewInit(2)).unwrap_or(());
            });
        }
    }
}
