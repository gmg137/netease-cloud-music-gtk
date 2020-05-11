use crate::app::Action;
use glib::Sender;
use gtk::{GtkMenuItemExt, Menu, MenuItem, MenuShellExt, WidgetExt};
use libappindicator::{AppIndicator, AppIndicatorStatus};

pub struct Tray {
    menu: Menu,
}

impl Tray {
    pub(crate) fn new(sender: &Sender<Action>) -> Tray {
        let mut indicator = AppIndicator::new("Netease Cloud Music", "netease-cloud-music-gtk");
        indicator.set_status(AppIndicatorStatus::Active);

        let mut menu = Menu::new();
        indicator.set_menu(&mut menu);

        let sen = sender.clone();
        let m = MenuItem::new_with_label("显示");
        m.connect_activate(move |_| sen.send(Action::ActivateApp).unwrap());
        menu.append(&m);

        let sen = sender.clone();
        let m = MenuItem::new_with_label("退出");
        m.connect_activate(move |_| sen.send(Action::QuitMain).unwrap());
        menu.append(&m);

        Tray { menu }
    }

    pub fn show_all(&self) {
        self.menu.show_all();
    }
}
