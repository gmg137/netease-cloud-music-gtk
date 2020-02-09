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
use crossbeam_channel::Sender;
use gdk_pixbuf::{InterpType, Pixbuf};
use gtk::prelude::*;
use gtk::{Builder, Button, EventBox, Grid, Image, Label};
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Clone)]
pub struct FmView {
    image: Image,
    like: Button,
    dislike: Button,
    play: Button,
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
            title,
            singer,
            recommend,
            nowplay: Rc::new(RefCell::new(None)),
            sender: sender.clone(),
        };

        Self::init(&fmview);
        fmview
    }

    fn init(s: &Self) {
        let sender = s.sender.clone();
        s.play.connect_clicked(move |_| {
            sender.send(Action::PlayerFm).unwrap_or(());
        });

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

    pub(crate) fn update_recommend_view(&self, rr: Vec<SongList>) {
        // 更新个性推荐
        self.recommend.foreach(|w| {
            self.recommend.remove(w);
        });
        self.recommend.hide();
        if !rr.is_empty() {
            let mut l = 0;
            let mut t = 0;
            for sl in rr.iter() {
                //if l < 4 {
                let event_box = EventBox::new();
                let boxs = gtk::Box::new(gtk::Orientation::Vertical, 0);
                let label = Label::new(Some(&sl.name[..]));
                label.set_lines(2);
                label.set_max_width_chars(16);
                label.set_ellipsize(pango::EllipsizeMode::End);
                label.set_line_wrap(true);
                let image_path = format!("{}{}.jpg", NCM_CACHE.to_string_lossy(), &sl.id);
                if let Ok(image) = Pixbuf::new_from_file(&image_path) {
                    let image = image.scale_simple(140, 140, InterpType::Bilinear);
                    let image = Image::new_from_pixbuf(image.as_ref());
                    boxs.add(&image);
                } else {
                    let image = Image::new_from_icon_name(Some("media-optical"), gtk::IconSize::Button);
                    image.set_pixel_size(140);
                    boxs.add(&image);
                };
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

                // 添加到容器
                self.recommend.attach(&event_box, l, t, 1, 1);
                //}
                l += 1;
                if l >= 4 {
                    l = 0;
                    t = 1;
                }
            }
            self.recommend.set_no_show_all(false);
            self.recommend.show_all();
        }
    }

    pub(crate) fn set_now_play(&self, si: SongInfo) {
        *self.nowplay.borrow_mut() = Some(si);
    }

    pub(crate) fn get_now_play(&self) -> Option<SongInfo> {
        self.nowplay.borrow().to_owned()
    }

    pub(crate) fn play_fm(&self) {
        let sender = self.sender.clone();
        self.nowplay.borrow().clone().map(|si| {
            sender
                .send(Action::PlayerInit(si.to_owned(), PlayerTypes::Fm))
                .unwrap_or(());
        });
    }
}
