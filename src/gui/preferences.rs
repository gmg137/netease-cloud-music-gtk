//
// preferences.rs
// Copyright (C) 2022 gmg137 <gmg137 AT live.com>
// Distributed under terms of the GPL-3.0-or-later license.
//

use adw::prelude::ActionRowExt;
use gettextrs::gettext;
use gio::Settings;
use gtk::gio::SettingsBindFlags;
use gtk::{CompositeTemplate, glib, prelude::*, subclass::prelude::*, *};
use once_cell::sync::OnceCell;

glib::wrapper! {
    pub struct NeteaseCloudMusicGtk4Preferences(ObjectSubclass<imp::NeteaseCloudMusicGtk4Preferences>)
        @extends adw::PreferencesDialog, adw::Dialog, Widget,
        @implements Accessible, Buildable, ConstraintTarget, Native, Root, ShortcutManager;
}

impl NeteaseCloudMusicGtk4Preferences {
    pub fn new() -> Self {
        glib::Object::new()
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
            .bind("exit-switch", &switch, "active")
            .flags(SettingsBindFlags::DEFAULT)
            .build();

        let mute_start_switch = self.imp().mute_start_switch.get();
        self.settings()
            .bind("mute-start", &mute_start_switch, "active")
            .flags(SettingsBindFlags::DEFAULT)
            .build();

        let not_ignore_grey_switch = self.imp().not_ignore_grey_switch.get();
        self.settings()
            .bind("not-ignore-grey", &not_ignore_grey_switch, "active")
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

        let desktop_lyrics = self.imp().desktop_lyrics.get();
        self.settings()
            .bind("desktop-lyrics", &desktop_lyrics, "active")
            .flags(SettingsBindFlags::DEFAULT)
            .build();
    }

    fn setup_font(&self) {
        self.settings().connect_changed(
            Some("application-font-family"),
            glib::clone!(
                #[weak(rename_to = preferences)]
                self,
                move |_, _| preferences.update_font_row()
            ),
        );
        self.update_font_row();

        self.imp().choose_font.connect_clicked(glib::clone!(
            #[weak(rename_to = preferences)]
            self,
            move |_| preferences.choose_font()
        ));
        self.imp().reset_font.connect_clicked(glib::clone!(
            #[weak(rename_to = preferences)]
            self,
            move |_| preferences.settings().reset("application-font-family")
        ));
    }

    fn update_font_row(&self) {
        let family = self.settings().string("application-font-family");
        let subtitle = if family.is_empty() {
            gettext("System Default")
        } else {
            family.to_string()
        };
        self.imp().application_font.set_subtitle(&subtitle);
        self.imp().reset_font.set_sensitive(!family.is_empty());
    }

    fn choose_font(&self) {
        let Some(parent) = self.root().and_downcast::<Window>() else {
            return;
        };
        let settings = self.settings().clone();
        let family = settings.string("application-font-family");
        let initial_family = self
            .pango_context()
            .font_map()
            .and_then(|font_map| font_map.family(&family));
        let dialog = FontDialog::builder()
            .title(gettext("Application Font"))
            .modal(true)
            .build();
        dialog.choose_family(
            Some(&parent),
            initial_family.as_ref(),
            None::<&gio::Cancellable>,
            move |result| match result {
                Ok(family) => {
                    if let Err(err) = settings.set_string("application-font-family", &family.name())
                    {
                        log::warn!("Could not save application font: {err}");
                    }
                }
                Err(err)
                    if err.matches(DialogError::Dismissed)
                        || err.matches(DialogError::Cancelled) => {}
                Err(err) => log::warn!("Could not choose application font: {err}"),
            },
        );
    }

    pub fn set_cache_size_label(&self, size: f64, unit: String) {
        self.imp()
            .cache_clear
            .get()
            .set_property("subtitle", format!("{:.1} {}", size, unit));
    }
}

impl Default for NeteaseCloudMusicGtk4Preferences {
    fn default() -> Self {
        Self::new()
    }
}

mod imp {

    use adw::subclass::prelude::*;

    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/gitee/gmg137/NeteaseCloudMusicGtk4/gtk/preferences.ui")]
    pub struct NeteaseCloudMusicGtk4Preferences {
        pub settings: OnceCell<Settings>,
        #[template_child]
        pub application_font: TemplateChild<adw::ActionRow>,
        #[template_child]
        pub choose_font: TemplateChild<Button>,
        #[template_child]
        pub reset_font: TemplateChild<Button>,
        #[template_child]
        pub exit_switch: TemplateChild<Switch>,
        #[template_child]
        pub mute_start_switch: TemplateChild<Switch>,
        #[template_child]
        pub not_ignore_grey_switch: TemplateChild<Switch>,
        #[template_child]
        pub proxy_entry: TemplateChild<Entry>,
        #[template_child]
        pub switch_rate: TemplateChild<adw::ComboRow>,
        #[template_child]
        pub cache_clear: TemplateChild<adw::ComboRow>,
        #[template_child]
        pub desktop_lyrics: TemplateChild<Switch>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for NeteaseCloudMusicGtk4Preferences {
        const NAME: &'static str = "NeteaseCloudMusicGtk4Preferences";
        type Type = super::NeteaseCloudMusicGtk4Preferences;
        type ParentType = adw::PreferencesDialog;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for NeteaseCloudMusicGtk4Preferences {
        fn constructed(&self) {
            let obj = self.obj();
            self.parent_constructed();

            obj.setup_settings();
            obj.bind_settings();
            obj.setup_font();
        }
    }
    impl WidgetImpl for NeteaseCloudMusicGtk4Preferences {}
    impl AdwDialogImpl for NeteaseCloudMusicGtk4Preferences {}
    impl PreferencesDialogImpl for NeteaseCloudMusicGtk4Preferences {}
}
