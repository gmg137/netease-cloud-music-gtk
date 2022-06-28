//
// discover.rs
// Copyright (C) 2022 gmg137 <gmg137 AT live.com>
// Distributed under terms of the GPL-3.0-or-later license.
//
use glib::{clone, Continue, MainContext, Sender, PRIORITY_DEFAULT};
use gtk::{glib, prelude::*, subclass::prelude::*, CompositeTemplate, *};
use ncm_api::{BannersInfo, SongInfo, SongList};
use once_cell::sync::OnceCell;
use std::sync::{Arc, RwLock};

use crate::{application::Action, path::CACHE};

glib::wrapper! {
    pub struct Discover(ObjectSubclass<imp::Discover>)
        @extends gtk::Widget, gtk::Box,
        @implements gtk::Accessible, gtk::Buildable,gtk::ConstraintTarget, gtk::Orientable;
}

impl Discover {
    pub fn new() -> Self {
        let discover: Discover = glib::Object::new(&[]).expect("Failed to create Discover");
        discover
    }

    pub fn set_sender(&self, sender: Sender<Action>) {
        self.imp().sender.set(sender).unwrap();
    }

    pub fn init_page(&self) {
        self.init_carousel();
        self.init_top_picks();
        self.init_new_albums();
    }

    pub fn init_carousel(&self) {
        let sender = self.imp().sender.get().unwrap();
        sender.send(Action::InitCarousel).unwrap();
    }

    pub fn init_top_picks(&self) {
        let sender = self.imp().sender.get().unwrap();
        sender.send(Action::InitTopPicks).unwrap();
    }

    pub fn init_new_albums(&self) {
        let sender = self.imp().sender.get().unwrap();
        sender.send(Action::InitNewAlbums).unwrap();
    }

    pub fn setup_top_picks(&self, song_list: Vec<SongList>) {
        let sender = self.imp().sender.get().unwrap();
        let top_picks = self.imp().top_picks.get();
        let mut row = 1;
        let mut col = 1;
        for sl in song_list {
            let mut path = CACHE.clone();
            path.push(format!("{}-songlist.jpg", sl.id));
            if path.exists() {
                let image = gtk::Image::from_file(path);
                let boxs = gtk::Box::new(gtk::Orientation::Vertical, 0);
                image.set_pixel_size(140);
                let frame = gtk::Frame::new(None);
                frame.set_halign(gtk::Align::Center);
                frame.set_child(Some(&image));
                boxs.append(&frame);
                let label = gtk::Label::new(Some(&sl.name));
                label.set_lines(2);
                label.set_margin_start(20);
                label.set_margin_end(20);
                label.set_width_chars(1);
                label.set_max_width_chars(1);
                label.set_ellipsize(gtk::pango::EllipsizeMode::End);
                label.set_wrap(true);
                boxs.append(&label);
                top_picks.attach(&boxs, col, row, 1, 1);
                col += 1;
                if col == 5 {
                    col = 1;
                    row = 2;
                }
                let gesture_click = GestureClick::new();
                image.add_controller(&gesture_click);
                let sender = sender.clone();
                gesture_click.connect_pressed(move |_, _, _, _| {
                    sender.send(Action::ToSongListPage(sl.clone())).unwrap();
                });
            } else {
                sender.send(Action::InitTopPicks).unwrap();
                break;
            }
        }
    }

    pub fn setup_new_albums(&self, song_list: Vec<SongList>) {
        let sender = self.imp().sender.get().unwrap();
        let new_albums = self.imp().new_albums.get();
        let mut row = 1;
        let mut col = 1;
        for sl in song_list {
            let mut path = CACHE.clone();
            path.push(format!("{}-songlist.jpg", sl.id));
            if path.exists() {
                let image = gtk::Image::from_file(path);
                let boxs = gtk::Box::new(gtk::Orientation::Vertical, 0);
                image.set_pixel_size(140);
                let frame = gtk::Frame::new(None);
                frame.set_halign(gtk::Align::Center);
                frame.set_child(Some(&image));
                boxs.append(&frame);
                let label = gtk::Label::new(Some(&sl.name));
                label.set_lines(2);
                label.set_margin_start(20);
                label.set_margin_end(20);
                label.set_width_chars(1);
                label.set_max_width_chars(1);
                label.set_ellipsize(gtk::pango::EllipsizeMode::End);
                label.set_wrap(true);
                boxs.append(&label);
                new_albums.attach(&boxs, col, row, 1, 1);
                col += 1;
                if col == 5 {
                    col = 1;
                    row = 2;
                }
                let gesture_click = GestureClick::new();
                image.add_controller(&gesture_click);
                let sender = sender.clone();
                gesture_click.connect_pressed(move |_, _, _, _| {
                    sender.send(Action::ToAlbumPage(sl.clone())).unwrap();
                });
            } else {
                sender.send(Action::InitNewAlbums).unwrap();
                break;
            }
        }
    }

    pub fn add_carousel(&self, banner: BannersInfo) {
        let carousel = self.imp().carousel.get();

        if carousel.n_pages() == 2 {
            let widget = carousel.nth_page(1);
            carousel.scroll_to(&widget, false);
        }

        let mut path = CACHE.clone();
        path.push(format!("{}-banner.jpg", banner.id));

        // 图片加载方式已验证，必须这样才能实现。
        let image = gtk::gdk_pixbuf::Pixbuf::from_file(path).unwrap();
        let image = image
            .scale_simple(730, 283, gtk::gdk_pixbuf::InterpType::Bilinear)
            .unwrap();
        let image = gtk::Picture::for_pixbuf(&image);
        image.set_halign(gtk::Align::Center);
        image.set_valign(gtk::Align::Fill);
        image.set_width_request(730);
        image.set_can_shrink(true);
        carousel.append(&image);
        self.imp().banners.borrow_mut().push(banner);
    }
}

impl Default for Discover {
    fn default() -> Self {
        Self::new()
    }
}

mod imp {

    use std::cell::RefCell;

    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/gitee/gmg137/NeteaseCloudMusicGtk4/gtk/discover.ui")]
    pub struct Discover {
        #[template_child]
        pub carousel: TemplateChild<adw::Carousel>,
        #[template_child]
        pub previous_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub next_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub top_picks: TemplateChild<gtk::Grid>,
        #[template_child]
        pub new_albums: TemplateChild<gtk::Grid>,
        pub rotation_timer_id: Arc<RwLock<bool>>,
        pub timeout_sender: OnceCell<Sender<()>>,
        pub sender: OnceCell<Sender<Action>>,
        pub banners: RefCell<Vec<BannersInfo>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Discover {
        const NAME: &'static str = "Discover";
        type Type = super::Discover;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[gtk::template_callbacks]
    impl Discover {
        #[template_callback]
        fn carousel_notify_position_cb(&self) {
            let f = { *self.rotation_timer_id.read().unwrap() };
            if !f {
                self.stop_rotation_timer();
                self.start_rotation_timer();
            }
        }

        #[template_callback]
        fn next_button_clicked_cb(&self) {
            Self::show_relative_page(self.carousel.get(), 1.)
        }

        #[template_callback]
        fn previous_button_clicked_cb(&self) {
            Self::show_relative_page(self.carousel.get(), -1.)
        }

        // 单击轮播图事件
        #[template_callback]
        fn carousel_pressed_cb(&self) {
            let position = self.carousel.position();
            if let Some(banner) = self.banners.borrow().get((position as f64) as usize) {
                let song_info = SongInfo {
                    id: banner.id.to_owned(),
                    name: banner.name.to_owned(),
                    singer: banner.singer.to_owned(),
                    album: banner.album.to_owned(),
                    album_id: banner.id.to_owned(),
                    pic_url: banner.pic_url.to_owned(),
                    duration: banner.duration.to_owned(),
                    song_url: "".to_owned(),
                };
                let sender = self.sender.get().unwrap();
                sender.send(Action::AddPlay(song_info)).unwrap();
            }
        }

        #[template_callback]
        fn top_picks_cb(&self) {
            let sender = self.sender.get().unwrap();
            sender.send(Action::InitTopPicksSongList).unwrap();
        }

        #[template_callback]
        fn new_albums_cb(&self) {
            let sender = self.sender.get().unwrap();
            sender.send(Action::InitAllAlbums).unwrap();
        }

        fn show_relative_page(carousel: adw::Carousel, delta: f64) {
            let current_page = carousel.position();
            let n_pages = carousel.n_pages();
            let mut animate = true;
            if n_pages == 0 {
                return;
            }
            let new_page = (current_page + delta + n_pages as f64) % n_pages as f64;
            let widget = carousel.nth_page(new_page as u32);
            if (new_page == 0.0 && delta > 0.) || (new_page as u32 == n_pages - 1 && delta < 0.) {
                animate = false;
            }
            carousel.scroll_to(&widget, animate);
        }

        fn start_rotation_timer(&self) {
            let f = { *self.rotation_timer_id.read().unwrap() };
            if f {
                let rotation_timer_id = self.rotation_timer_id.clone();
                if let Some(sender) = self.timeout_sender.get() {
                    let sender = sender.clone();
                    glib::timeout_add_seconds(5, move || {
                        let mut rotation_timer_id = rotation_timer_id.write().unwrap();
                        *rotation_timer_id = false;
                        sender.send(()).unwrap();
                        Continue(false)
                    });
                }
            }
        }

        fn stop_rotation_timer(&self) {
            let f = { *self.rotation_timer_id.read().unwrap() };
            if !f {
                let mut rotation_timer_id = self.rotation_timer_id.write().unwrap();
                *rotation_timer_id = true;
            }
        }
    }

    impl ObjectImpl for Discover {
        fn constructed(&self, obj: &Self::Type) {
            self.parent_constructed(obj);

            self.banners.replace(Vec::new());

            // 自动轮播
            let (sender, receiver) = MainContext::channel(PRIORITY_DEFAULT);
            self.timeout_sender.set(sender).unwrap();
            let carousel = self.carousel.get();
            receiver.attach(
                None,
                clone!(@weak carousel => @default-return Continue(false), move |_| {
                    let current_page = carousel.position();
                    let n_pages = carousel.n_pages();
                    let mut animate = true;
                    if n_pages == 0 {
                        return Continue(false);
                    }
                    let new_page = (current_page + 1. + n_pages as f64) % n_pages as f64;
                    let widget = carousel.nth_page(new_page as u32);
                    if (new_page == 0.0 && 1. > 0.) || (new_page as u32 == n_pages - 1 && 1. < 0.) {
                        animate = false;
                    }
                    carousel.scroll_to(&widget, animate);
                    Continue(true)
                }),
            );
            Self::show_relative_page(self.carousel.get(), 0.);
        }
    }
    impl WidgetImpl for Discover {}
    impl BoxImpl for Discover {}
}
