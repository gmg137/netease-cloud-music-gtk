//
// home.rs
// Copyright (C) 2019 gmg137 <gmg137@live.com>
// Distributed under terms of the GPLv3 license.
//

use crate::app::Action;
use crate::model::NCM_CACHE;
use crate::musicapi::model::*;
use crossbeam_channel::Sender;
use gdk_pixbuf::{InterpType, Pixbuf};
use gtk::prelude::*;
use gtk::{Builder, EventBox, Frame, Grid, Image, Label, ShadowType};

#[derive(Clone)]
pub(crate) struct Home {
    up_grid: Grid,
    low_grid: Grid,
    up_title: gtk::Box,
    low_title: gtk::Box,
    sender: Sender<Action>,
}

impl Home {
    pub(crate) fn new(builder: &Builder, sender: Sender<Action>) -> Self {
        let up_grid: Grid = builder
            .get_object("top_song_list_grid")
            .expect("无法获取 top_song_list_grid 窗口.");
        let low_grid: Grid = builder
            .get_object("recommend_resource_grid")
            .expect("无法获取 recommend_resource_grid 窗口.");
        let up_title: gtk::Box = builder.get_object("home_top_title").expect("无法获取 top title.");
        let low_title: gtk::Box = builder
            .get_object("home_recommend_title")
            .expect("无法获取 recommend title.");
        up_title.hide();
        low_title.hide();
        let home = Home {
            up_grid,
            low_grid,
            up_title,
            low_title,
            sender: sender.clone(),
        };
        home.init(sender.clone());
        home
    }

    fn init(&self, sender: Sender<Action>) {
        sender.send(Action::RefreshHome).unwrap_or(());
    }

    pub(crate) fn update(&mut self, tsl: Vec<SongList>, na: Vec<SongList>) {
        self.up_grid.foreach(|w| {
            self.up_grid.remove(w);
        });
        self.low_grid.foreach(|w| {
            self.low_grid.remove(w);
        });
        self.up_title.hide();
        self.low_title.hide();
        if !tsl.is_empty() {
            let mut l = 0;
            let mut t = 0;

            tsl.iter().for_each(|sl| {
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
                self.up_grid.attach(&event_box, l, t, 1, 1);
                l += 1;
                if l >= 4 {
                    l = 0;
                    t = 1;
                }

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
            });
            if !na.is_empty() {
                let mut l = 0;
                for sl in na.iter() {
                    if l < 4 {
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
                                    Parse::ALBUM,
                                ))
                                .unwrap_or(());
                            Inhibit(false)
                        });

                        // 添加到容器
                        self.low_grid.attach(&event_box, l, 0, 1, 1);
                    }
                    l += 1;
                }
                self.low_title.set_no_show_all(false);
                self.low_title.show_all();
                self.low_grid.show_all();
            }
            self.up_title.set_no_show_all(false);
            self.up_title.show_all();
            self.up_grid.show_all();
        }
    }

    pub(crate) fn set_up_image(&self, left: i32, top: i32, song_list: SongList) {
        if let Some(w) = self.up_grid.get_child_at(left, top) {
            self.up_grid.remove(&w);
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
        self.up_grid.attach(&event_box, left, top, 1, 1);

        let id = song_list.id;
        let name = song_list.name.to_owned();
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
        self.up_grid.show_all();
    }

    pub(crate) fn set_low_image(&self, left: i32, top: i32, song_list: SongList) {
        if let Some(w) = self.low_grid.get_child_at(left, top) {
            self.low_grid.remove(&w);
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
        self.low_grid.attach(&event_box, left, top, 1, 1);

        let id = song_list.id;
        let name = song_list.name.to_owned();
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
        self.low_grid.show_all();
    }
}
