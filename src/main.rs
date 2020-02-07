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
use app::App;

static APP_VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() {
    let args = std::env::args();
    args.for_each(|args| {
        if args.eq("--debug") {
            loggerv::Logger::new()
                .module_path_filters(vec![module_path!().to_owned()])
                .max_level(log::Level::Debug)
                .line_numbers(true)
                .init()
                .expect("Error initializing loggerv.");
        }
    });

    info!("配置目录: {:?}", *model::NCM_CONFIG);
    info!("数据目录: {:?}", *model::NCM_DATA);
    info!("缓存目录: {:?}", *model::NCM_CACHE);

    gtk::init().expect("Error initializing gtk.");
    gst::init().expect("Error initializing gstreamer");

    App::run();
}
