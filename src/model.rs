//
// model.rs
// Copyright (C) 2022 gmg137 <gmg137 AT live.com>
// Distributed under terms of the GPL-3.0-or-later license.
//
use crate::application::Action;
use glib::{SendWeakRef, Sender};
use gtk::{gdk_pixbuf, glib, prelude::*, Image, Picture};
use ncm_api::{SingerInfo, SongInfo, SongList};
use std::{cell::RefCell, path::PathBuf, rc::Rc, sync::Arc};

#[derive(Default)]
pub struct UserInfo {
    pub uid: u64,
    pub like_songs: std::collections::HashSet<u64>,
}

#[derive(Debug)]
pub struct PageStack {
    gtk_stack: gtk::Stack, // add, remove, set_visible

    // this is needed, as we can't remove and set_visible to gtk_stack at the same time
    // use this to keep a clear stack for every operation
    stack: Rc<RefCell<Vec<gtk::StackPage>>>, // push, pop, remove
}

impl PageStack {
    pub fn new(gtk_stack: gtk::Stack) -> PageStack {
        let pages: Vec<gtk::StackPage> = gtk_stack
            .pages()
            .iter::<gtk::StackPage>()
            .unwrap()
            .map(|p| p.unwrap())
            .collect();
        PageStack {
            gtk_stack,
            stack: Rc::new(RefCell::new(pages)),
        }
    }

    fn set_gtk_stack_visible(&self, stack_page: &gtk::StackPage) {
        self.gtk_stack.set_visible_child(&stack_page.child());
    }

    fn remove_from_gtk_stack(&self, stack_page: gtk::StackPage) {
        // delay remove for animation
        let gtk_stack = self.gtk_stack.clone();
        let stack = self.stack.clone();
        let ctx = glib::MainContext::default();
        let page = stack_page.child();
        ctx.spawn_local(async move {
            glib::timeout_future(std::time::Duration::from_millis(500)).await;
            if page.parent().is_some() && !stack.borrow().iter().any(|p| p.child() == page) {
                gtk_stack.remove(&page);
            }
        });
    }

    pub fn new_page(&self, page: &impl glib::IsA<gtk::Widget>) -> gtk::StackPage {
        let mut stack = self.stack.borrow_mut();
        let page = page.clone().upcast::<gtk::Widget>();
        let stack_page = if let Some(idx) = stack.iter().position(|p| p.child() == page) {
            stack.remove(idx)
        } else if page.parent().is_none() {
            self.gtk_stack.add_child(&page)
        } else {
            self.gtk_stack.page(&page)
        };

        stack.push(stack_page.clone());
        self.set_gtk_stack_visible(&stack_page);
        stack_page
    }

    pub fn new_page_with_name(
        &self,
        page: &impl glib::IsA<gtk::Widget>,
        name: &str,
    ) -> gtk::StackPage {
        let stack = &self.stack;
        let old_page_idx = stack.borrow().iter().position(|p| {
            let has_name = p.name() == Some(glib::GString::from(name));
            if has_name {
                p.set_name("");
            }
            has_name
        });

        let stack_page = self.new_page(page);
        stack_page.set_name(name);

        if let Some(old_page_idx) = old_page_idx {
            let old_page = stack.borrow().get(old_page_idx).unwrap().clone();
            if old_page != stack_page {
                stack.borrow_mut().remove(old_page_idx);
                self.remove_from_gtk_stack(old_page);
            }
        }

        stack_page
    }

    pub fn back_page(&self) {
        let mut stack = self.stack.borrow_mut();
        // keep bottom page
        if stack.len() > 1 {
            let stack_page = stack.pop().unwrap();
            let pre_page = stack.last().unwrap().clone();

            self.set_gtk_stack_visible(&pre_page);
            self.remove_from_gtk_stack(stack_page);
        }
    }

    pub fn top_page(&self) -> gtk::StackPage {
        self.stack.borrow().last().unwrap().clone()
    }

    pub fn set_transition_type(&self, transition: gtk::StackTransitionType) {
        self.gtk_stack.set_transition_type(transition);
    }

    pub fn set_transition_duration(&self, milliseconds: u32) {
        self.gtk_stack.set_transition_duration(milliseconds);
    }

    pub fn len(&self) -> usize {
        self.stack.borrow().len()
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum UserMenuChild {
    Qr,
    Phone,
    User,
}

#[derive(Debug, Clone)]
pub enum DiscoverSubPage {
    SongList,
    Album,
}

#[derive(Debug, Clone)]
pub enum SongListDetail {
    PlayList(ncm_api::PlayListDetail, ncm_api::PlayListDetailDynamic),
    Album(ncm_api::AlbumDetail, ncm_api::AlbumDetailDynamic),
}

impl SongListDetail {
    pub fn sis(&self) -> &Vec<SongInfo> {
        match self {
            Self::PlayList(d, ..) => &d.songs,
            Self::Album(d, ..) => &d.songs,
        }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, glib::Enum)]
#[repr(i32)]
#[enum_type(name = "SearchType")]
pub enum SearchType {
    // 搜索歌曲
    Song,
    // 搜索歌手
    Singer,
    // 搜索专辑
    Album,
    // 搜索歌词
    Lyrics,
    // 搜索歌单
    SongList,
    // 搜索歌手歌曲
    SingerSongs,
    // 搜索热门歌单
    TopPicks,
    // 搜索全部专辑
    AllAlbums,
    // 搜索每日推荐歌曲
    DailyRec,
    // 我喜欢的音乐
    Heartbeat,
    // 云盘音乐
    CloudDisk,
    // 每人FM
    Fm,
    // 收藏的专辑
    LikeAlbums,
    // 收藏的歌单
    LikeSongList,
}

#[derive(Debug, Clone)]
pub enum SearchResult {
    Songs(Vec<SongInfo>, Vec<bool>),
    Singers(Vec<SingerInfo>),
    SongLists(Vec<SongList>),
}

impl Default for SearchType {
    fn default() -> Self {
        SearchType::Song
    }
}

impl Default for UserMenuChild {
    fn default() -> Self {
        UserMenuChild::Qr
    }
}

pub trait ImageDownloadImpl {
    // 参数
    // url: 图片链接
    // path: 要保存的图片本地路径
    // size: 图片宽高象素
    fn set_from_net(&self, url: String, path: PathBuf, size: (u16, u16), sender: &Sender<Action>);
}

impl ImageDownloadImpl for Image {
    fn set_from_net(&self, url: String, path: PathBuf, size: (u16, u16), sender: &Sender<Action>) {
        let image = glib::SendWeakRef::from(self.downgrade());
        sender
            .send(Action::DownloadImage(
                url,
                path.to_owned(),
                size.0,
                size.1,
                Some(Arc::new(move |_| {
                    image.upgrade().unwrap().set_from_file(Some(&path));
                })),
            ))
            .unwrap();
    }
}

impl ImageDownloadImpl for Picture {
    fn set_from_net(&self, url: String, path: PathBuf, size: (u16, u16), sender: &Sender<Action>) {
        let picture = glib::SendWeakRef::from(self.downgrade());
        sender
            .send(Action::DownloadImage(
                url,
                path.to_owned(),
                size.0,
                size.1,
                Some(Arc::new(move |_| {
                    let image = gtk::gdk_pixbuf::Pixbuf::from_file(&path).unwrap();
                    let image = image
                        .scale_simple(
                            size.0 as i32,
                            size.1 as i32,
                            gtk::gdk_pixbuf::InterpType::Bilinear,
                        )
                        .unwrap();
                    picture.upgrade().unwrap().set_pixbuf(Some(&image));
                })),
            ))
            .unwrap();
    }
}

impl ImageDownloadImpl for adw::Avatar {
    fn set_from_net(&self, url: String, path: PathBuf, size: (u16, u16), sender: &Sender<Action>) {
        let avatar = SendWeakRef::from(self.downgrade());
        sender
            .send(Action::DownloadImage(
                url,
                path.to_owned(),
                size.0,
                size.1,
                Some(Arc::new(move |_| {
                    if let Ok(pixbuf) = gdk_pixbuf::Pixbuf::from_file(&path) {
                        let image = Image::from_pixbuf(Some(&pixbuf));
                        if let Some(paintable) = image.paintable() {
                            avatar.upgrade().unwrap().set_custom_image(Some(&paintable));
                        }
                    }
                })),
            ))
            .unwrap();
    }
}
