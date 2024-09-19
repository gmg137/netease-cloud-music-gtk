//
// user_menus.rs
// Copyright (C) 2022 gmg137 <gmg137 AT live.com>
// Distributed under terms of the GPL-3.0-or-later license.
//
use crate::{
    application::Action,
    model::{ImageDownloadImpl, UserMenuChild},
};
use adw::*;
use async_channel::Sender;
use gettextrs::gettext;
use glib::clone;
use gtk::{prelude::*, *};
use once_cell::sync::OnceCell;
use std::cell::Cell;
use std::path::PathBuf;

#[derive(Debug, Default)]
pub struct UserMenus {
    pub qrbox: Box,
    pub qrimage: Image,
    pub refresh_button: Button,
    pub change_button: Button,
    pub sender: OnceCell<Sender<Action>>,

    pub phonebox: Box,
    pub ctcode_entry: Entry,
    pub phone_entry: Entry,
    pub captcha_entry: Entry,
    pub captcha_button: Button,
    pub login_button: Button,
    pub back_button: Button,

    pub userbox: Box,
    pub avatar: Avatar,
    pub user_name: Label,
    pub logout_button: Button,

    current_menu: Cell<UserMenuChild>,
}

impl UserMenus {
    pub fn new(send: Sender<Action>) -> Self {
        let builder = gtk::Builder::from_resource(
            "/com/gitee/gmg137/NeteaseCloudMusicGtk4/gtk/user-menus.ui",
        );
        let qrbox: Box = builder.object("qrbox").unwrap();
        let qrimage: Image = builder.object("qrimage").unwrap();
        let refresh_button: Button = builder.object("refresh_button").unwrap();
        let change_button: Button = builder.object("change_button").unwrap();

        let phonebox: Box = builder.object("phonebox").unwrap();
        let ctcode_entry: Entry = builder.object("ctcode_entry").unwrap();
        let phone_entry: Entry = builder.object("phone_entry").unwrap();
        let captcha_entry: Entry = builder.object("captcha_entry").unwrap();
        let captcha_button: Button = builder.object("captcha_button").unwrap();
        let login_button: Button = builder.object("login_button").unwrap();
        let back_button: Button = builder.object("back_button").unwrap();

        let userbox: Box = builder.object("userbox").unwrap();
        let avatar: Avatar = builder.object("avatar").unwrap();
        let user_name: Label = builder.object("user_name").unwrap();
        let logout_button: Button = builder.object("logout_button").unwrap();

        let current_menu: Cell<UserMenuChild> = Cell::new(UserMenuChild::default());

        let sender = OnceCell::new();
        sender.set(send).unwrap();
        let s = Self {
            qrbox,
            qrimage,
            refresh_button,
            change_button,
            sender,

            phonebox,
            ctcode_entry,
            phone_entry,
            captcha_entry,
            captcha_button,
            login_button,
            back_button,

            userbox,
            avatar,
            user_name,
            logout_button,

            current_menu,
        };
        s.setup_signal();
        s
    }

    fn setup_signal(&self) {
        let sender = self.sender.get().unwrap().clone();
        self.refresh_button.connect_clicked(move |_| {
            sender.send_blocking(Action::TryUpdateQrCode).unwrap();
        });

        let sender = self.sender.get().unwrap().clone();
        self.change_button.connect_clicked(move |_| {
            sender.send_blocking(Action::SwitchUserMenuToPhone).unwrap();
        });

        let sender = self.sender.get().unwrap().clone();
        self.back_button.connect_clicked(move |_| {
            sender.send_blocking(Action::SwitchUserMenuToQr).unwrap();
        });

        let sender = self.sender.get().unwrap().clone();
        self.captcha_button.connect_clicked(clone!(
            #[weak(rename_to = ctcode)]
            self.ctcode_entry,
            #[weak(rename_to = phone)]
            self.phone_entry,
            move |_| {
                let ctcode = ctcode.text().to_string();
                let phone = phone.text().to_string();
                if ctcode.parse::<u64>().is_ok() && phone.parse::<u64>().is_ok() {
                    sender
                        .send_blocking(Action::GetCaptcha(ctcode, phone))
                        .unwrap();
                } else {
                    sender
                        .send_blocking(Action::AddToast(gettext("Input format error!")))
                        .unwrap();
                }
            }
        ));

        let sender = self.sender.get().unwrap().clone();
        self.login_button.connect_clicked(clone!(
            #[weak(rename_to = ctcode)]
            self.ctcode_entry,
            #[weak(rename_to = phone)]
            self.phone_entry,
            #[weak(rename_to = captcha)]
            self.captcha_entry,
            move |_| {
                let ctcode = ctcode.text().to_string();
                let phone = phone.text().to_string();
                let captcha = captcha.text().to_string();
                if ctcode.parse::<u64>().is_ok()
                    && phone.parse::<u64>().is_ok()
                    && !captcha.is_empty()
                {
                    sender
                        .send_blocking(Action::CaptchaLogin(ctcode, phone, captcha))
                        .unwrap();
                } else {
                    sender
                        .send_blocking(Action::AddToast(gettext("Input format error!")))
                        .unwrap();
                }
            }
        ));

        let sender = self.sender.get().unwrap().clone();
        self.logout_button.connect_clicked(move |_| {
            sender.send_blocking(Action::Logout).unwrap();
        });
    }

    pub fn set_qrimage(&self, path: PathBuf) {
        self.qrimage.set_opacity(1.0);
        self.qrimage.set_from_file(Some(path));
        self.refresh_button.set_visible(false);
    }

    pub fn set_qrimage_timeout(&self) {
        self.qrimage.set_opacity(0.3);
        self.refresh_button.set_visible(true);
    }

    pub fn set_user_avatar(&self, url: String, path: PathBuf) {
        self.avatar
            .set_from_net(url, path, (50, 50), self.sender.get().unwrap());
    }

    pub fn set_user_name(&self, name: String) {
        self.user_name.set_text(&name);
    }

    pub fn is_menu_active(&self, menu: UserMenuChild) -> bool {
        self.current_menu.get() == menu
    }

    pub fn switch_menu(&self, new_menu: UserMenuChild, popover: &PopoverMenu) {
        fn get_box(_self: &UserMenus, menu: UserMenuChild) -> &impl IsA<Widget> {
            match menu {
                UserMenuChild::Qr => &_self.qrbox,
                UserMenuChild::Phone => &_self.phonebox,
                UserMenuChild::User => &_self.userbox,
            }
        }
        let current = self.current_menu.get();
        if current != new_menu {
            popover.remove_child(get_box(self, current));
            popover.add_child(get_box(self, new_menu), "user_popover");
            popover.notify("child");

            self.current_menu.replace(new_menu);
        }
    }
}
