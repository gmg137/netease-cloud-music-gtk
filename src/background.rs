use crate::{application::Action, gui::NcmImageSource, ncmapi::NcmClient};
use glib::{MainContext, Receiver, Sender};
use gtk::glib;
use std::path::PathBuf;
use std::rc::Rc;

pub type FgSender = Sender<Action>;
pub type BgSender = Sender<BgAction>;
pub type BgReceiver = Receiver<BgAction>;

pub enum BgAction {
    DownloadImage(NcmClient, NcmImageSource),
}

#[derive(Debug, Clone)]
pub struct Background {
    fg_sender: FgSender,
    sender: BgSender,
}

struct ThreadData {}

impl Background {
    pub fn new(fg_sender: FgSender) -> Self {
        let (sender, receiver) = MainContext::channel(glib::PRIORITY_LOW);

        let s = Background { fg_sender, sender };
        let s_ = s.clone();
        std::thread::spawn(move || {
            MainContext::new()
                .with_thread_default(move || {
                    s_.run(receiver);
                })
                .unwrap();
        });
        s
    }

    pub fn sender(&self) -> BgSender {
        self.sender.clone()
    }

    fn run(&self, receiver: BgReceiver) {
        let cnt = MainContext::thread_default().unwrap();
        let mainloop = glib::MainLoop::new(Some(&cnt), false);

        let tdata = Rc::new(ThreadData {});

        let s = self.clone();
        receiver.attach(Some(&cnt), move |action| {
            s.process_action(action, tdata.clone())
        });

        mainloop.run();
    }

    fn process_action(&self, action: BgAction, _tdata: Rc<ThreadData>) -> glib::Continue {
        let ctx = MainContext::thread_default().unwrap();
        let fg_sender = self.fg_sender.clone();

        match action {
            BgAction::DownloadImage(ncmapi, source) => {
                let (w, h) = source.size();
                let path = source.to_path();

                let source_ = source.clone();
                let load_from_file = move |path: PathBuf| {
                    let file = gtk::gio::File::for_path(&path);
                    match gtk::gdk::Texture::from_file(&file) {
                        Ok(tex) => {
                            fg_sender
                                .send(Action::ImageDownloaded(source_, tex))
                                .unwrap();
                        }
                        Err(err) => log::error!("{:?}", err),
                    };
                };
                ctx.spawn_local(async move {
                    let source_ = source.clone();
                    if path.exists() {
                        load_from_file(path);
                    } else {
                        match source_ {
                            NcmImageSource::SongList(_, url)
                            | NcmImageSource::Banner(_, url)
                            | NcmImageSource::TopList(_, url)
                            | NcmImageSource::Singer(_, url)
                            | NcmImageSource::UserAvatar(_, url) => {
                                if ncmapi
                                    .client
                                    .download_img(url.clone(), path.clone(), w, h)
                                    .await
                                    .is_ok()
                                {
                                    load_from_file(path);
                                };
                            }
                        };
                    }
                });
            }
        }
        glib::Continue(true)
    }
}
