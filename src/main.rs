extern crate gstreamer as gst;
extern crate gstreamer_player as gst_player;

#[macro_use]
extern crate log;

mod app;
mod data;
mod model;
mod musicapi;
mod utils;
mod view;
mod widgets;
use crate::app::App;

static APP_VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() {
    loggerv::init_with_level(log::Level::Info).expect("Error initializing loggerv.");

    gtk::init().expect("Error initializing gtk.");
    gst::init().expect("Error initializing gstreamer");

    App::run();
}
