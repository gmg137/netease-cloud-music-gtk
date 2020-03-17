//
// fm.rs
// Copyright (C) 2019 gmg137 <gmg137@live.com>
// Distributed under terms of the GPLv3 license.
//
use crate::{
    app::Action,
    model::NCM_CACHE,
    musicapi::model::{Parse, SongInfo, SongList},
    utils::*,
};
use async_std::sync::Arc;
use gdk_pixbuf::{InterpType, Pixbuf};
use glib::clone;
use glib::Sender;
use gtk::{prelude::*, Builder, Button, EventBox, Frame, Grid, Image, Label, ShadowType};
use std::{cell::RefCell, rc::Rc};

#[derive(Clone)]
pub struct FmView {
    image: Image,
    like: Button,
    dislike: Button,
    play: Button,
    pause: Button,
    title: Label,
    singer: Label,
    recommend: Grid,
    nowplay: Rc<RefCell<Option<SongInfo>>>,
    sender: Sender<Action>,
}

impl FmView {
    pub(crate) fn new(mine_login_fm_builder: &Builder, sender: Sender<Action>) -> Self {
        let image: Image = mine_login_fm_builder
            .get_object("mine_fm_image")
            .expect("无法获取 mine_fm_image .");
        let like: Button = mine_login_fm_builder
            .get_object("mine_fm_like_button")
            .expect("无法获取 mine_fm_like_button .");
        let dislike: Button = mine_login_fm_builder
            .get_object("mine_fm_dislike_button")
            .expect("无法获取 mine_fm_dislike_button .");
        let play: Button = mine_login_fm_builder
            .get_object("mine_fm_play_button")
            .expect("无法获取 mine_fm_play_button .");
        let pause: Button = mine_login_fm_builder
            .get_object("mine_fm_pause_button")
            .expect("无法获取 mine_fm_play_button .");
        let title: Label = mine_login_fm_builder
            .get_object("mine_fm_title")
            .expect("无法获取 mine_fm_title .");
        let singer: Label = mine_login_fm_builder
            .get_object("mine_fm_singer")
            .expect("无法获取 mine_fm_singer .");
        let recommend: Grid = mine_login_fm_builder
            .get_object("recommend_resource_grid")
            .expect("无法获取 recommend_resource_grid 窗口.");
        let fmview = FmView {
            image,
            like,
            dislike,
            play,
            pause,
            title,
            singer,
            recommend,
            nowplay: Rc::new(RefCell::new(None)),
            sender,
        };

        Self::init(&fmview);
        fmview
    }

    fn init(s: &Self) {
        s.play.show();
        s.pause.hide();

        let sender = s.sender.clone();
        s.play.connect_clicked(clone!(@weak s.pause as pause => move |play| {
            play.hide();
            pause.show();
            sender.send(Action::PlayerFm).unwrap_or(());
        }));

        let sender = s.sender.clone();
        s.pause.connect_clicked(clone!(@weak s.play as play => move |pause| {
            pause.hide();
            play.show();
            sender.send(Action::PauseFm).unwrap_or(());
        }));

        let sender = s.sender.clone();
        s.like.connect_clicked(move |_| {
            sender.send(Action::FmLike).unwrap_or(());
        });

        let sender = s.sender.clone();
        s.dislike.connect_clicked(move |_| {
            sender.send(Action::FmDislike).unwrap_or(());
        });
    }

    pub(crate) fn update_fm_view(&self, song_info: &SongInfo) {
        // 更新 FM
        let image_path = format!("{}{}.jpg", crate::model::NCM_CACHE.to_string_lossy(), &song_info.id);
        if let Ok(image) = Pixbuf::new_from_file(&image_path) {
            let image = image.scale_simple(140, 140, InterpType::Bilinear);
            self.image.set_from_pixbuf(image.as_ref());
        };
        self.title.set_text(&song_info.name);
        self.singer.set_text(&song_info.singer);
    }

    pub(crate) fn update_recommend_view(&self, rr: Arc<Vec<SongList>>) {
        // 更新个性推荐
        self.recommend.foreach(|w| {
            self.recommend.remove(w);
        });
        self.recommend.hide();
        if !rr.is_empty() {
            let mut l = 0;
            for sl in rr.iter() {
                let event_box = EventBox::new();
                let boxs = gtk::Box::new(gtk::Orientation::Vertical, 0);
                let label = Label::new(Some(&sl.name[..]));
                let frame = Frame::new(None);
                frame.set_shadow_type(ShadowType::EtchedOut);
                label.set_lines(2);
                label.set_max_width_chars(16);
                label.set_ellipsize(pango::EllipsizeMode::End);
                label.set_line_wrap(true);
                let image_path = format!("{}{}.jpg", NCM_CACHE.to_string_lossy(), &sl.id);
                let image = if let Ok(image) = Pixbuf::new_from_file(&image_path) {
                    let image = image.scale_simple(140, 140, InterpType::Bilinear);
                    Image::new_from_pixbuf(image.as_ref())
                } else {
                    let image = Image::new_from_icon_name(Some("media-optical"), gtk::IconSize::Button);
                    image.set_pixel_size(140);
                    image
                };
                frame.add(&image);
                boxs.add(&frame);
                boxs.add(&label);
                event_box.add(&boxs);

                // 处理点击事件
                let id = sl.id;
                let name = sl.name.to_owned();
                let sender = self.sender.clone();
                event_box.connect_button_press_event(move |_, _| {
                    sender
                        .send(Action::SwitchStackSub(
                            (id, name.to_owned(), image_path.to_owned()),
                            Parse::USL,
                        ))
                        .unwrap_or(());
                    Inhibit(false)
                });
                let mut left = l;
                let top = if l >= 4 {
                    left = l % 4;
                    l / 4
                } else {
                    0
                };

                // 添加到容器
                self.recommend.attach(&event_box, left, top, 1, 1);
                l += 1;
            }
            self.recommend.set_no_show_all(false);
            self.recommend.show_all();
        }
    }

    pub(crate) fn set_recommend_image(&self, left: i32, top: i32, song_list: SongList) {
        if let Some(w) = self.recommend.get_child_at(left, top) {
            self.recommend.remove(&w);
        }
        let event_box = EventBox::new();
        let boxs = gtk::Box::new(gtk::Orientation::Vertical, 0);
        let label = Label::new(Some(&song_list.name[..]));
        let frame = Frame::new(None);
        frame.set_shadow_type(ShadowType::EtchedOut);
        label.set_lines(2);
        label.set_max_width_chars(16);
        label.set_ellipsize(pango::EllipsizeMode::End);
        label.set_line_wrap(true);
        let image_path = format!("{}{}.jpg", NCM_CACHE.to_string_lossy(), &song_list.id);
        let image = if let Ok(image) = Pixbuf::new_from_file(&image_path) {
            let image = image.scale_simple(140, 140, InterpType::Bilinear);
            Image::new_from_pixbuf(image.as_ref())
        } else {
            let image = Image::new_from_icon_name(Some("media-optical"), gtk::IconSize::Button);
            image.set_pixel_size(140);
            image
        };
        frame.add(&image);
        boxs.add(&frame);
        boxs.add(&label);
        event_box.add(&boxs);
        self.recommend.attach(&event_box, left, top, 1, 1);

        let id = song_list.id;
        let name = song_list.name;
        let sender = self.sender.clone();
        event_box.connect_button_press_event(move |_, _| {
            sender
                .send(Action::SwitchStackSub(
                    (id, name.to_owned(), image_path.to_owned()),
                    Parse::USL,
                ))
                .unwrap_or(());
            Inhibit(false)
        });
        self.recommend.show_all();
    }

    pub(crate) fn set_now_play(&self, si: SongInfo) {
        *self.nowplay.borrow_mut() = Some(si);
    }

    pub(crate) fn get_now_play(&self) -> Option<SongInfo> {
        self.nowplay.borrow().to_owned()
    }

    pub(crate) fn play_fm(&self) {
        let sender = self.sender.clone();
        if let Some(si) = self.nowplay.borrow().clone() {
            sender.send(Action::PlayerInit(si, PlayerTypes::Fm)).unwrap_or(());
        };
    }

    pub(crate) fn switch_play(&self) {
        self.pause.hide();
        self.play.show();
    }

    pub(crate) fn switch_pause(&self) {
        self.play.hide();
        self.pause.show();
    }
}
