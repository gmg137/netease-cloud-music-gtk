//
// header.rs
// Copyright (C) 2019 gmg137 <gmg137@live.com>
// Distributed under terms of the GPLv3 license.
//

use super::preferences::Preferences;
use crate::app::Action;
use crate::data::MusicData;
use crate::musicapi::model::LoginInfo;
use crate::utils::*;
use crate::{clone, upgrade_weak};
use crate::{APP_VERSION, CACHED_PATH};
use crossbeam_channel::Sender;
use gtk::prelude::*;
use gtk::{
    AboutDialog, Builder, Button, Dialog, Entry, Image, Label, MenuButton, ModelButton, Popover, SearchBar,
    SearchEntry, StackSwitcher, ToggleButton,
};
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::thread::spawn;

#[derive(Clone)]
pub(crate) struct Header {
    back: Button,
    switch: StackSwitcher,
    title: Label,
    search: ToggleButton,
    search_bar: SearchBar,
    search_entry: SearchEntry,
    username: Label,
    avatar: Image,
    menu: MenuButton,
    logoutbox: gtk::Box,
    login: ModelButton,
    logout: Button,
    task: Button,
    user_button: MenuButton,
    login_dialog: LoginDialog,
    popover_user: Popover,
    preferences_button: ModelButton,
    preferences: Preferences,
    about_button: ModelButton,
    close_button: ModelButton,
    about: AboutDialog,
    sender: Sender<Action>,
    data: Arc<Mutex<u8>>,
}

#[derive(Debug, Clone)]
pub(crate) struct LoginDialog {
    dialog: Dialog,
    name: Entry,
    pass: Entry,
    ok: Button,
}

impl Header {
    pub(crate) fn new(builder: &Builder, sender: &Sender<Action>, data: Arc<Mutex<u8>>, configs: &Configs) -> Rc<Self> {
        let back: Button = builder.get_object("back_button").expect("Couldn't get back button");
        let switch: StackSwitcher = builder.get_object("stack_switch").expect("Couldn't get stack switch");
        let title: Label = builder.get_object("subpages_title").expect("Couldn't get title");
        let search: ToggleButton = builder.get_object("search_button").expect("Couldn't get search button");
        let search_bar: SearchBar = builder.get_object("search_bar").expect("Couldn't get search bar");
        let search_entry: SearchEntry = builder.get_object("search_entry").expect("Couldn't get search entry");
        let avatar: Image = builder.get_object("avatar").expect("Couldn't get avatar image");
        let username: Label = builder
            .get_object("username_label")
            .expect("Couldn't get username_label");
        let menu: MenuButton = builder.get_object("menu_button").expect("Couldn't get menu button");
        let logoutbox: gtk::Box = builder.get_object("logout_box").expect("Couldn't get login button");
        let login: ModelButton = builder.get_object("login_button").expect("Couldn't get login button");
        let logout: Button = builder.get_object("logout_button").expect("Couldn't get logout button");
        let user_button: MenuButton = builder.get_object("user_button").expect("Couldn't get user button");
        let preferences_button: ModelButton = builder
            .get_object("preferences_button")
            .expect("Couldn't get preferences button");
        let about_button: ModelButton = builder.get_object("about_button").expect("Couldn't get about button");
        let close_button: ModelButton = builder.get_object("close_button").expect("Couldn't get close button");
        let about: AboutDialog = builder.get_object("about_dialog").expect("Couldn't get about dialog");
        let task: Button = builder.get_object("task_button").expect("Couldn't get task button");
        let dialog: Dialog = builder.get_object("login_dialog").expect("Couldn't get login dialog");
        let popover_user: Popover = builder.get_object("popover_user").expect("Couldn't get popover");
        let name: Entry = builder.get_object("name_entry").expect("Couldn't get name entry");
        let pass: Entry = builder.get_object("pass_entry").expect("Couldn't get pass entry");
        let ok: Button = builder.get_object("login").expect("Couldn't get login button");
        let login_dialog = LoginDialog { dialog, name, pass, ok };
        let preferences = Preferences::new(builder, sender.clone(), configs);
        let header = Header {
            back,
            switch,
            title,
            search,
            search_bar,
            search_entry,
            avatar,
            username,
            menu,
            popover_user,
            logoutbox,
            login,
            logout,
            user_button,
            preferences_button,
            preferences,
            about_button,
            close_button,
            about,
            task,
            login_dialog,
            sender: sender.clone(),
            data: data.clone(),
        };
        let h = Rc::new(header);
        Self::init(&h, &sender);
        h
    }

    fn init(s: &Rc<Self>, sender: &Sender<Action>) {
        s.user_button.set_sensitive(false);
        sender.send(Action::RefreshHeaderUser).unwrap();

        // 登陆按钮
        let dialog_weak = s.login_dialog.dialog.downgrade();
        s.login.connect_clicked(clone!(dialog_weak=>move|_| {
            let dialog = upgrade_weak!(dialog_weak);
            dialog.run();
            dialog.hide();
        }));

        // 退出按钮
        let sen = sender.clone();
        s.logout.connect_clicked(move |_| {
            sen.send(Action::Logout).unwrap();
        });

        // 登陆对话框
        let dialog_weak = s.login_dialog.dialog.downgrade();
        let name_weak = s.login_dialog.name.downgrade();
        let pass_weak = s.login_dialog.pass.downgrade();
        let sen = sender.clone();
        s.login_dialog
            .ok
            .connect_clicked(clone!(dialog_weak,name_weak,pass_weak=>move|_| {
            let dialog = upgrade_weak!(dialog_weak);
            let name = upgrade_weak!(name_weak).get_text().unwrap().as_str().to_owned();
            let pass = upgrade_weak!(pass_weak).get_text().unwrap().as_str().to_owned();
            if !name.is_empty() && !pass.is_empty(){
                sen.send(Action::Login(name,pass)).unwrap();
                dialog.hide();
            }}));

        // 签到按钮
        let sen = sender.clone();
        s.task.connect_clicked(move |_| {
            sen.send(Action::DailyTask).unwrap();
        });

        // 搜索按钮
        let search_bar_weak = s.search_bar.downgrade();
        let search_entry_weak = s.search_entry.downgrade();
        s.search
            .connect_clicked(clone!(search_bar_weak ,search_entry_weak=> move |_| {
                let search_bar = upgrade_weak!(search_bar_weak);
                let search_entry = upgrade_weak!(search_entry_weak);
                search_entry.set_property_is_focus(true);
                search_bar.set_search_mode(!search_bar.get_search_mode());
            }));

        // 搜索框
        let search_entry_weak = s.search_entry.downgrade();
        let sender_clone = sender.clone();
        s.search_entry
            .connect_key_press_event(clone!(search_entry_weak =>move|_, key| {
                // 回车键直接搜索
                let keyval = key.get_keyval();
                if keyval == 65293 || keyval == 65421 {
                    if let Some(text) = search_entry_weak.upgrade().unwrap().get_text(){
                        if !text.is_empty(){
                            sender_clone.send(Action::Search(text.to_owned())).unwrap_or(());
                        }
                    }
                }
                Inhibit(false)
            }));

        // 返回按钮
        let sen = sender.clone();
        let title_weak = s.title.downgrade();
        let switch_weak = s.switch.downgrade();
        let back_weak = s.back.downgrade();
        s.back
            .connect_clicked(clone!(title_weak,switch_weak,back_weak => move |_| {
                let title = upgrade_weak!(title_weak);
                let switch = upgrade_weak!(switch_weak);
                let back = upgrade_weak!(back_weak);
                title.hide();
                switch.show();
                back.hide();
                sen.send(Action::SwitchStackMain).unwrap();
            }));

        // 设置关于窗口版本号
        s.about.set_version(Some(APP_VERSION));

        // 关于按钮
        let about_weak = s.about.downgrade();
        s.about_button.connect_clicked(clone!(about_weak => move |_| {
            let about = upgrade_weak!(about_weak);
            about.run();
            about.hide();
        }));

        // 首选项
        let dialog_weak = s.preferences.dialog.downgrade();
        s.preferences_button.connect_clicked(clone!(dialog_weak => move |_| {
            let dialog = upgrade_weak!(dialog_weak);
            dialog.run();
            dialog.hide();
        }));

        // 关闭按钮
        let sen = sender.clone();
        s.close_button.connect_clicked(move |_| {
            sen.send(Action::QuitMain).unwrap();
        });
    }

    // 登录
    pub(crate) fn login(&self, name: String, pass: String) {
        let sender = self.sender.clone();
        let data = self.data.clone();
        spawn(move || {
            #[allow(unused_variables)]
            let lock = data.lock().unwrap();
            let mut data = MusicData::new();
            if let Some(login_info) = data.login(name, pass) {
                if login_info.code == 200 {
                    sender.send(Action::RefreshHeaderUser).unwrap();
                    sender.send(Action::RefreshHome).unwrap();
                    sender.send(Action::RefreshMine).unwrap();
                    return;
                } else {
                    sender.send(Action::ShowNotice(login_info.msg)).unwrap();
                    return;
                }
            };
            sender.send(Action::ShowNotice("登陆异常!".to_owned())).unwrap();
        });
    }

    // 退出
    pub(crate) fn logout(&self) {
        let sender = self.sender.clone();
        let data = self.data.clone();
        spawn(move || {
            #[allow(unused_variables)]
            let lock = data.lock().unwrap();
            let mut data = MusicData::new();
            data.logout();
            sender.send(Action::RefreshHeaderUser).unwrap();
            sender.send(Action::RefreshHome).unwrap();
            sender.send(Action::RefreshMine).unwrap();
        });
    }

    // 更新用户头像和相关按钮
    pub(crate) fn update_user_button(&self) {
        let data = self.data.clone();
        let sender = self.sender.clone();
        spawn(move || {
            #[allow(unused_variables)]
            let lock = data.lock().unwrap();
            let mut data = MusicData::new();
            if data.login {
                if let Some(login_info) = data.login_info() {
                    sender
                        .send(Action::RefreshHeaderUserLogin(login_info.to_owned()))
                        .unwrap();
                }
            } else {
                sender.send(Action::RefreshHeaderUserLogout).unwrap();
            }
        });
    }

    // 更新标题栏为已登录
    pub(crate) fn update_user_login(&self, login_info: LoginInfo) {
        self.user_button.set_sensitive(true);
        self.popover_user.show_all();
        let image_url = format!("{}?param=37y37", &login_info.avatar_url);
        let image_path = format!("{}/{}.jpg", CACHED_PATH.to_owned(), &login_info.uid);
        download_img(&image_url, &image_path, 37, 37);
        if let Ok(image) = create_round_avatar(&image_path) {
            self.avatar.set_from_pixbuf(Some(&image));
        }
        self.username.set_text(&login_info.nickname);
        self.login.hide();
    }

    // 更新标题栏为未登录
    pub(crate) fn update_user_logout(&self) {
        self.user_button.set_sensitive(true);
        self.popover_user.show_all();
        self.logoutbox.hide();
    }

    // 签到
    pub(crate) fn daily_task(&self) {
        let sender = self.sender.clone();
        let data = self.data.clone();
        spawn(move || {
            #[allow(unused_variables)]
            let lock = data.lock().unwrap();
            let mut data = MusicData::new();
            if let Some(task) = data.daily_task() {
                if task.code == 200 {
                    sender.send(Action::ShowNotice("签到成功!".to_owned())).unwrap();
                } else {
                    sender.send(Action::ShowNotice(task.msg)).unwrap();
                }
            } else {
                sender.send(Action::ShowNotice("网络异常!".to_owned())).unwrap();
            }
        });
    }

    // 更新标题栏
    pub(crate) fn switch_header(&self, title: String) {
        self.switch.hide();
        self.title.set_markup(&title);
        self.title.show();
        self.back.show();
    }
}
