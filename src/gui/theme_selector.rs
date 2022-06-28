//
// theme_selector.rs
// Copyright (C) 2022 gmg137 <gmg137 AT live.com>
// Distributed under terms of the GPL-3.0-or-later license.
//
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::{gdk, glib, CompositeTemplate, CssProvider, StyleContext};

glib::wrapper! {
    pub struct ThemeSelector(ObjectSubclass<imp::ThemeSelector>)
        @extends gtk::Widget,
        @implements gtk::Accessible;
}

impl Default for ThemeSelector {
    fn default() -> Self {
        Self::new()
    }
}

impl ThemeSelector {
    pub fn new() -> Self {
        glib::Object::new(&[]).expect("Failed to create Button")
    }
}

mod imp {

    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/gitee/gmg137/NeteaseCloudMusicGtk4/gtk/theme-selector.ui")]
    pub struct ThemeSelector {
        #[template_child(id = "box")]
        pub gbox: TemplateChild<gtk::Box>,
        #[template_child(id = "system")]
        pub system: TemplateChild<gtk::ToggleButton>,
        #[template_child(id = "dark")]
        pub dark: TemplateChild<gtk::ToggleButton>,
        #[template_child(id = "light")]
        pub light: TemplateChild<gtk::ToggleButton>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ThemeSelector {
        const NAME: &'static str = "ThemeSelector";
        type Type = super::ThemeSelector;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
            load_css();
            klass.set_css_name("themeselector");
            klass.set_layout_manager_type::<gtk::BinLayout>();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ThemeSelector {
        fn dispose(&self, _obj: &Self::Type) {
            self.gbox.unparent();
        }
    }
    impl WidgetImpl for ThemeSelector {}
}

fn load_css() {
    // Load the CSS file and add it to the provider
    let provider = CssProvider::new();
    provider
        .load_from_resource("/com/gitee/gmg137/NeteaseCloudMusicGtk4/themes/themesselector.css");

    // Add the provider to the default screen
    StyleContext::add_provider_for_display(
        &gdk::Display::default().expect("Could not connect to a display."),
        &provider,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
}
