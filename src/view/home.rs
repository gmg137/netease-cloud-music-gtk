//
// home.rs
// Copyright (C) 2019 gmg137 <gmg137@live.com>
// Distributed under terms of the GPLv3 license.
//

use crate::{app::Action, model::NCM_CACHE, musicapi::model::*};
use async_std::sync::Arc;
use gdk_pixbuf::{InterpType, Pixbuf};
use gtk::glib::Sender;
use gtk::{prelude::*, Builder, Frame, GestureClick, Grid, Image, Label};

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
            .object("top_song_list_grid")
            .expect("无法获取 top_song_list_grid 窗口.");
        let low_grid: Grid = builder
            .object("recommend_resource_grid")
            .expect("无法获取 recommend_resource_grid 窗口.");
        let up_title: gtk::Box = builder.object("home_top_title").expect("无法获取 top title.");
        let low_title: gtk::Box = builder
            .object("home_recommend_title")
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
        home.init(sender);
        home
    }

    fn init(&self, sender: Sender<Action>) {
        sender.send(Action::RefreshHome).unwrap_or(());
    }

    pub(crate) fn update(&mut self, tsl: Arc<Vec<SongList>>, na: Arc<Vec<SongList>>) {
        while let Some(child) = self.up_grid.last_child() {
            self.up_grid.remove(&child);
        }
        while let Some(child) = self.low_grid.last_child() {
            self.low_grid.remove(&child);
        }
        self.up_title.hide();
        self.low_title.hide();
        if !tsl.is_empty() {
            let mut l = 0;
            let mut t = 0;

            tsl.iter().for_each(|sl| {
                let gesture_click = GestureClick::new();
                let boxs = gtk::Box::new(gtk::Orientation::Vertical, 0);
                let label = Label::new(Some(&sl.name[..]));
                let frame = Frame::new(None);
                label.set_lines(2);
                label.set_max_width_chars(16);
                label.set_ellipsize(pango::EllipsizeMode::End);
                label.set_wrap(true);
                let image_path = format!("{}{}.jpg", NCM_CACHE.to_string_lossy(), &sl.id);
                let image = if let Ok(image) = Pixbuf::from_file(&image_path) {
                    let image = image.scale_simple(140, 140, InterpType::Bilinear);
                    Image::from_pixbuf(image.as_ref())
                } else {
                    let image = Image::from_icon_name("media-optical");
                    image.set_pixel_size(140);
                    image
                };
                frame.set_child(Some(&image));
                boxs.append(&frame);
                boxs.append(&label);
                boxs.add_controller(&gesture_click);
                self.up_grid.attach(&boxs, l, t, 1, 1);
                l += 1;
                if l >= 4 {
                    l = 0;
                    t = 1;
                }

                let id = sl.id;
                let name = sl.name.to_owned();
                let sender = self.sender.clone();
                gesture_click.connect_pressed(move |_, _, _, _| {
                    sender
                        .send(Action::SwitchStackSub(
                            (id, name.to_owned(), image_path.to_owned()),
                            Parse::USL,
                        ))
                        .unwrap_or(());
                });
            });
            if !na.is_empty() {
                for (l, sl) in na.iter().enumerate() {
                    if l < 4 {
                        let gesture_click = GestureClick::new();
                        let boxs = gtk::Box::new(gtk::Orientation::Vertical, 0);
                        let label = Label::new(Some(&sl.name[..]));
                        let frame = Frame::new(None);
                        label.set_lines(2);
                        label.set_max_width_chars(16);
                        label.set_ellipsize(pango::EllipsizeMode::End);
                        label.set_wrap(true);
                        let image_path = format!("{}{}.jpg", NCM_CACHE.to_string_lossy(), &sl.id);
                        let image = if let Ok(image) = Pixbuf::from_file(&image_path) {
                            let image = image.scale_simple(140, 140, InterpType::Bilinear);
                            Image::from_pixbuf(image.as_ref())
                        } else {
                            let image = Image::from_icon_name("media-optical");
                            image.set_pixel_size(140);
                            image
                        };
                        frame.set_child(Some(&image));
                        boxs.append(&frame);
                        boxs.append(&label);
                        boxs.add_controller(&gesture_click);

                        // 处理点击事件
                        let id = sl.id;
                        let name = sl.name.to_owned();
                        let sender = self.sender.clone();
                        gesture_click.connect_pressed(move |_, _, _, _| {
                            sender
                                .send(Action::SwitchStackSub(
                                    (id, name.to_owned(), image_path.to_owned()),
                                    Parse::ALBUM,
                                ))
                                .unwrap_or(());
                        });

                        // 添加到容器
                        self.low_grid.attach(&boxs, l as i32, 0, 1, 1);
                    }
                }
                self.low_title.show();
                self.low_grid.show();
            }
            self.up_title.show();
            self.up_grid.show();
        }
    }

    pub(crate) fn set_up_image(&self, left: i32, top: i32, song_list: SongList) {
        if let Some(w) = self.up_grid.child_at(left, top) {
            self.up_grid.remove(&w);
        }
        let gesture_click = GestureClick::new();
        let boxs = gtk::Box::new(gtk::Orientation::Vertical, 0);
        let label = Label::new(Some(&song_list.name[..]));
        let frame = Frame::new(None);
        label.set_lines(2);
        label.set_max_width_chars(16);
        label.set_ellipsize(pango::EllipsizeMode::End);
        label.set_wrap(true);
        let image_path = format!("{}{}.jpg", NCM_CACHE.to_string_lossy(), &song_list.id);
        let image = if let Ok(image) = Pixbuf::from_file(&image_path) {
            let image = image.scale_simple(140, 140, InterpType::Bilinear);
            Image::from_pixbuf(image.as_ref())
        } else {
            let image = Image::from_icon_name("media-optical");
            image.set_pixel_size(140);
            image
        };
        frame.set_child(Some(&image));
        boxs.append(&frame);
        boxs.append(&label);
        boxs.add_controller(&gesture_click);
        self.up_grid.attach(&boxs, left, top, 1, 1);

        let id = song_list.id;
        let name = song_list.name;
        let sender = self.sender.clone();
        gesture_click.connect_pressed(move |_, _, _, _| {
            sender
                .send(Action::SwitchStackSub(
                    (id, name.to_owned(), image_path.to_owned()),
                    Parse::USL,
                ))
                .unwrap_or(());
        });
        self.up_grid.show();
    }

    pub(crate) fn set_low_image(&self, left: i32, top: i32, song_list: SongList) {
        if let Some(w) = self.low_grid.child_at(left, top) {
            self.low_grid.remove(&w);
        }
        let gesture_click = GestureClick::new();
        let boxs = gtk::Box::new(gtk::Orientation::Vertical, 0);
        let boxs = gtk::Box::new(gtk::Orientation::Vertical, 0);
        let label = Label::new(Some(&song_list.name[..]));
        let frame = Frame::new(None);
        label.set_lines(2);
        label.set_max_width_chars(16);
        label.set_ellipsize(pango::EllipsizeMode::End);
        label.set_wrap(true);
        let image_path = format!("{}{}.jpg", NCM_CACHE.to_string_lossy(), &song_list.id);
        let image = if let Ok(image) = Pixbuf::from_file(&image_path) {
            let image = image.scale_simple(140, 140, InterpType::Bilinear);
            Image::from_pixbuf(image.as_ref())
        } else {
            let image = Image::from_icon_name("media-optical");
            image.set_pixel_size(140);
            image
        };
        frame.set_child(Some(&image));
        boxs.append(&frame);
        boxs.append(&label);
        boxs.add_controller(&gesture_click);
        self.low_grid.attach(&boxs, left, top, 1, 1);

        let id = song_list.id;
        let name = song_list.name;
        let sender = self.sender.clone();
        gesture_click.connect_pressed(move |_, _, _, _| {
            sender
                .send(Action::SwitchStackSub(
                    (id, name.to_owned(), image_path.to_owned()),
                    Parse::ALBUM,
                ))
                .unwrap_or(());
        });
        self.low_grid.show();
    }
}
