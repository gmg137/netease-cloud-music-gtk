mod application;
mod audio;
mod config;
mod gui;
mod model;
mod ncmapi;
mod path;
mod window;

use self::application::NeteaseCloudMusicGtk4Application;
use self::window::NeteaseCloudMusicGtk4Window;

use config::{GETTEXT_PACKAGE, LOCALEDIR, PKGDATADIR};
use gettextrs::{bind_textdomain_codeset, bindtextdomain, textdomain};
use gtk::gio;
use gtk::glib;
use gtk::prelude::*;

const APP_ID: &str = "com.gitee.gmg137.NeteaseCloudMusicGtk4";

fn main() {
    // Initialize gstreamer
    gstreamer::init().expect("Error initializing gstreamer");

    // Initialize paths
    path::init().expect("Unable to create paths.");
    // Set up gettext translations
    bindtextdomain(GETTEXT_PACKAGE, LOCALEDIR).expect("Unable to bind the text domain");
    bind_textdomain_codeset(GETTEXT_PACKAGE, "UTF-8")
        .expect("Unable to set the text domain encoding");
    textdomain(GETTEXT_PACKAGE).expect("Unable to switch to the text domain");

    // Load resources
    let resources =
        gio::Resource::load(PKGDATADIR.to_owned() + "/netease-cloud-music-gtk4.gresource")
            .expect("Could not load resources");
    gio::resources_register(&resources);

    // Create a new GtkApplication. The application manages our main loop,
    // application windows, integration with the window manager/compositor, and
    // desktop features such as file opening and single-instance applications.
    let app = NeteaseCloudMusicGtk4Application::new(APP_ID, &gio::ApplicationFlags::empty());

    let ctx = glib::MainContext::default();
    let _guard = ctx.acquire().unwrap();

    // Run the application. This function will block until the application
    // exits. Upon return, we have our exit code to return to the shell. (This
    // is the code you see when you do `echo $?` after running a command in a
    // terminal.
    std::process::exit(app.run());
}
