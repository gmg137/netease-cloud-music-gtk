//
// header.rs
// Copyright (C) 2019 gmg137 <gmg137@live.com>
// Distributed under terms of the GPLv3 license.
//

use super::preferences::Preferences;
use crate::{
    app::Action,
    musicapi::model::LoginInfo,
    utils::*,
    {data::MusicData, model::NCM_CACHE, APP_VERSION},
};
use async_std::task;
use glib::{clone, Sender};
use gtk::{
    prelude::*, AboutDialog, Builder, Button, Dialog, Entry, Image, Label, MenuButton, ModelButton, Popover, SearchBar,
    SearchEntry, StackSwitcher, ToggleButton,
};
use std::rc::Rc;

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
}

#[derive(Debug, Clone)]
pub(crate) struct LoginDialog {
    dialog: Dialog,
    name: Entry,
    pass: Entry,
    ok: Button,
    cancel: Button,
}

impl Header {
    pub(crate) fn new(builder: &Builder, sender: &Sender<Action>, configs: &Configs) -> Rc<Self> {
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
        let cancel: Button = builder.get_object("cancel_login").expect("Couldn't get login button");
        let login_dialog = LoginDialog {
            dialog,
            name,
            pass,
            ok,
            cancel,
        };
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
        };
        let h = Rc::new(header);
        Self::init(&h, &sender);
        h
    }

    fn init(s: &Rc<Self>, sender: &Sender<Action>) {
        s.user_button.set_sensitive(false);
        sender.send(Action::RefreshHeaderUser).unwrap();

        // 登陆按钮
        s.login
            .connect_clicked(clone!(@weak s.login_dialog.dialog as dialog => move |_| {
                dialog.run();
                dialog.hide();
            }));

        // 退出按钮
        let sen = sender.clone();
        s.logout.connect_clicked(move |_| {
            sen.send(Action::Logout).unwrap();
        });

        // 登陆对话框
        let dialog = &s.login_dialog.dialog;
        let name = &s.login_dialog.name;
        let pass = &s.login_dialog.pass;
        let sen = sender.clone();
        s.login_dialog
            .ok
            .connect_clicked(clone!(@weak dialog, @weak name,@weak pass => move |_| {
            let name = name.get_text().unwrap().as_str().to_owned();
            let pass = pass.get_text().unwrap().as_str().to_owned();
            if !name.is_empty() && !pass.is_empty(){
                sen.send(Action::Login(name,pass)).unwrap();
                dialog.hide();
            }}));

        // 取消登陆按钮
        s.login_dialog
            .cancel
            .connect_clicked(clone!(@weak s.login_dialog.dialog as dialog => move |_| {
                dialog.hide();
            }));

        // 签到按钮
        let sen = sender.clone();
        s.task.connect_clicked(move |_| {
            sen.send(Action::DailyTask).unwrap();
        });

        // 搜索按钮
        let search_bar = &s.search_bar;
        let search_entry = &s.search_entry;
        s.search
            .connect_clicked(clone!(@weak search_bar, @weak search_entry=> move |_| {
                search_entry.set_property_is_focus(true);
                search_bar.set_search_mode(!search_bar.get_search_mode());
            }));

        // 搜索框
        let search_entry = &s.search_entry;
        let sender_clone = sender.clone();
        s.search_entry.connect_activate(clone!(@weak search_entry => move |_| {
            // 回车键直接搜索
            if let Some(text) = search_entry.get_text(){
                if !text.is_empty(){
                    sender_clone.send(Action::Search(text.to_owned())).unwrap_or(());
                }
            }
        }));

        // 返回按钮
        let send = sender.clone();
        let title = &s.title;
        let switch = &s.switch;
        let back = &s.back;
        s.back
            .connect_clicked(clone!(@weak title, @weak switch, @weak back=> move |_| {
                title.hide();
                switch.show();
                back.hide();
                send.send(Action::SwitchStackMain).unwrap();
            }));

        // 设置关于窗口版本号
        s.about.set_version(Some(APP_VERSION));

        // 关于按钮
        s.about_button
            .connect_clicked(clone!(@weak s.about as about => move |_| {
                about.run();
                about.hide();
            }));

        // 首选项
        s.preferences_button
            .connect_clicked(clone!(@weak s.preferences.dialog as dialog => move |_| {
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
        task::spawn(async move {
            if let Ok(mut data) = MusicData::new().await {
                sender.send(Action::RefreshHome).unwrap();
                if let Ok(login_info) = data.login(name, pass).await {
                    if login_info.code == 200 {
                        sender.send(Action::RefreshHeaderUser).unwrap();
                        return;
                    } else {
                        sender.send(Action::ShowNotice(login_info.msg)).unwrap();
                        return;
                    }
                };
                sender.send(Action::ShowNotice("登陆异常!".to_owned())).unwrap();
            } else {
                sender.send(Action::ShowNotice("接口请求异常!".to_owned())).unwrap();
            }
        });
    }

    // 退出
    pub(crate) fn logout(&self) {
        let sender = self.sender.clone();
        task::spawn(async move {
            if let Ok(mut data) = MusicData::new().await {
                data.logout().await.ok();
                sender.send(Action::RefreshHeaderUser).unwrap();
            } else {
                sender.send(Action::ShowNotice("接口请求异常!".to_owned())).unwrap();
            }
        });
    }

    // 更新用户头像和相关按钮
    pub(crate) fn update_user_button(&self) {
        let sender = self.sender.clone();
        task::spawn(async move {
            if let Ok(mut data) = MusicData::new().await {
                if data.login {
                    if let Ok(login_info) = data.login_info().await {
                        let image_path = format!("{}{}.jpg", NCM_CACHE.to_string_lossy(), &login_info.uid);
                        download_img(&login_info.avatar_url, &image_path, 37, 37, 5000)
                            .await
                            .ok();
                        sender
                            .send(Action::RefreshHeaderUserLogin(login_info.to_owned()))
                            .unwrap();
                        sender.send(Action::RefreshMine).unwrap_or(());
                        return;
                    }
                }
                sender.send(Action::RefreshHeaderUserLogout).unwrap();
                sender.send(Action::RefreshMine).unwrap_or(());
                return;
            } else {
                sender.send(Action::ShowNotice("接口请求异常!".to_owned())).unwrap();
            }
        });
    }

    // 更新标题栏为已登录
    pub(crate) fn update_user_login(&self, login_info: LoginInfo) {
        self.user_button.set_sensitive(true);
        self.logoutbox.show_all();
        let image_path = format!("{}{}.jpg", NCM_CACHE.to_string_lossy(), &login_info.uid);
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
        task::spawn(async move {
            if let Ok(mut data) = MusicData::new().await {
                if let Ok(task) = data.daily_task().await {
                    if task.code == 200 {
                        sender.send(Action::ShowNotice("签到成功!".to_owned())).unwrap();
                    } else {
                        sender.send(Action::ShowNotice(task.msg)).unwrap();
                    }
                } else {
                    sender.send(Action::ShowNotice("接口请求异常!".to_owned())).unwrap();
                }
            } else {
                sender.send(Action::ShowNotice("接口请求异常!".to_owned())).unwrap();
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
