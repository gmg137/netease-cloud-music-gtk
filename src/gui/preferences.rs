//
// preferences.rs
// Copyright (C) 2022 gmg137 <gmg137 AT live.com>
// Distributed under terms of the GPL-3.0-or-later license.
//

use adw::subclass::prelude::PreferencesWindowImpl;
use gio::Settings;
use gtk::gio::SettingsBindFlags;
use gtk::{glib, prelude::*, subclass::prelude::*, CompositeTemplate, *};
use once_cell::sync::OnceCell;

glib::wrapper! {
    pub struct NeteaseCloudMusicGtk4Preferences(ObjectSubclass<imp::NeteaseCloudMusicGtk4Preferences>)
        @extends Widget, Window, adw::Window,
        @implements Accessible, Buildable, ConstraintTarget, Native, Root, ShortcutManager;
}

impl NeteaseCloudMusicGtk4Preferences {
    pub fn new() -> Self {
        glib::Object::new(&[]).expect("Failed to create NeteaseCloudMusicGtk4Preferences")
    }

    fn setup_settings(&self) {
        let settings = Settings::new(crate::APP_ID);
        self.imp()
            .settings
            .set(settings)
            .expect("Could not set `Settings`.");
    }

    fn settings(&self) -> &Settings {
        self.imp().settings.get().expect("Could not get settings.")
    }

    fn bind_settings(&self) {
        let switch = self.imp().exit_switch.get();
        self.settings()
            .bind("exit-switch", &switch, "state")
            .flags(SettingsBindFlags::DEFAULT)
            .build();

        let entry = self.imp().proxy_entry.get();
        self.settings()
            .bind("proxy-address", &entry, "text")
            .flags(SettingsBindFlags::DEFAULT)
            .build();

        let rate = self.imp().switch_rate.get();
        self.settings()
            .bind("music-rate", &rate, "selected")
            .flags(SettingsBindFlags::DEFAULT)
            .build();

        let cache_clear = self.imp().cache_clear.get();
        self.settings()
            .bind("cache-clear", &cache_clear, "selected")
            .flags(SettingsBindFlags::DEFAULT)
            .build();
    }
}

impl Default for NeteaseCloudMusicGtk4Preferences {
    fn default() -> Self {
        Self::new()
    }
}

mod imp {

    use adw::subclass::prelude::AdwWindowImpl;

    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/gitee/gmg137/NeteaseCloudMusicGtk4/gtk/preferences.ui")]
    pub struct NeteaseCloudMusicGtk4Preferences {
        pub settings: OnceCell<Settings>,
        #[template_child]
        pub exit_switch: TemplateChild<Switch>,
        #[template_child]
        pub proxy_entry: TemplateChild<Entry>,
        #[template_child]
        pub switch_rate: TemplateChild<adw::ComboRow>,
        #[template_child]
        pub cache_clear: TemplateChild<adw::ComboRow>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for NeteaseCloudMusicGtk4Preferences {
        const NAME: &'static str = "NeteaseCloudMusicGtk4Preferences";
        type Type = super::NeteaseCloudMusicGtk4Preferences;
        type ParentType = adw::PreferencesWindow;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for NeteaseCloudMusicGtk4Preferences {
        fn constructed(&self, obj: &Self::Type) {
            self.parent_constructed(obj);

            obj.setup_settings();
            obj.bind_settings();
        }
    }
    impl WidgetImpl for NeteaseCloudMusicGtk4Preferences {}
    impl WindowImpl for NeteaseCloudMusicGtk4Preferences {}
    impl AdwWindowImpl for NeteaseCloudMusicGtk4Preferences {}
    impl PreferencesWindowImpl for NeteaseCloudMusicGtk4Preferences {}
}
