use glib::Sender;
use glib::{
    clone, closure_local, subclass::Signal, ParamSpec, ParamSpecObject, SignalHandlerId, Value,
};
pub(crate) use gtk::{glib, prelude::*, subclass::prelude::*, *};
use once_cell::sync::Lazy;

use crate::{application::Action, path::CACHE};
use once_cell::sync::OnceCell;
use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

static PAINTABLE_LOADER_REF: OnceCell<glib::SendWeakRef<NcmPaintableLoader>> = OnceCell::new();

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NcmImageSource {
    SongList(u64, String), // id, url
    Banner(u64, String),
    TopList(u64, String),
    Singer(u64, String),
    UserAvatar(u64, String),
}

impl NcmImageSource {
    pub fn to_gobj(&self) -> NcmImageSourceObject {
        NcmImageSourceObject::new(self.clone())
    }
    pub fn to_path(&self) -> PathBuf {
        CACHE.join(format!("{}", self.id()))
    }

    pub fn id(&self) -> String {
        match self {
            Self::SongList(id, ..) => format!("songlist-{}", id),
            Self::Banner(id, ..) => format!("banner-{}", id),
            Self::TopList(id, ..) => format!("toplist-{}", id),
            Self::Singer(id, ..) => format!("singer-{}", id),
            Self::UserAvatar(id, ..) => format!("user-{}", id),
        }
    }

    pub fn size(&self) -> (u16, u16) {
        match self {
            Self::SongList(..) | Self::TopList(..) | Self::Singer(..) => (140, 140),
            Self::Banner(..) => (730, 283),
            Self::UserAvatar(..) => (100, 100),
        }
    }
}

glib::wrapper! {
    pub struct NcmPaintableLoader(ObjectSubclass<imp::NcmPaintableLoader>);
}

impl NcmPaintableLoader {
    pub fn init(sender: Sender<Action>) -> Self {
        let s: Self = glib::Object::builder().build();

        s.imp().sender.set(sender).unwrap();

        PAINTABLE_LOADER_REF
            .set(glib::SendWeakRef::from(s.downgrade()))
            .unwrap();
        s
    }

    pub fn reg_new_texture(&self, source: NcmImageSource, tex: gdk::Texture) {
        let id = source.id();
        let ncm_tex = {
            let mut textures = self.imp().textures.borrow_mut();
            textures.get(id.as_str()).cloned().unwrap_or_else(|| {
                let t = NcmTexture::new();
                textures.insert(id.clone(), t.clone());
                t
            })
        };
        ncm_tex.set_texture(&tex);

        // remove when dispose
        let id = id.to_string();
        tex.add_weak_ref_notify_local(clone!(@weak self as s =>  move || {
            let mut textures = s.imp().textures.borrow_mut();
            if let Some(ncm_tex) = textures.get(&id) {
                if ncm_tex.texture().is_none() {
                    textures.remove(&id);
                }
            }
        }));
    }

    pub fn look_up(&self, paintable: &NcmPaintable, source: NcmImageSource) {
        let sender = self.imp().sender.get().unwrap();
        let id = source.id();
        let ncm_tex = {
            let mut textures = self.imp().textures.borrow_mut();
            textures.get(id.as_str()).cloned().unwrap_or_else(|| {
                let t = NcmTexture::new();
                textures.insert(id.clone(), t.clone());
                t
            })
        };

        paintable.disconnect_lookup();

        if let Some(tex) = ncm_tex.texture() {
            paintable.set_texture(tex);
        } else {
            let connected = ncm_tex.connect_texture_loaded(
                closure_local!(@watch paintable => move |_: NcmTexture, tex: gdk::Texture| {
                    if paintable.source().map(|s| s.id()) == Some(id.clone()) {
                        paintable.set_texture(tex);
                    }
                    paintable.disconnect_lookup();
                }),
            );
            let connected = Rc::new(RefCell::new(Some(connected)));
            paintable.set_disconnect_lookup(closure_local!(@watch ncm_tex => move|| {
                ncm_tex.disconnect(connected.replace(None).unwrap());
            }));

            sender.send(Action::BgDownloadImage(source)).unwrap();
        }
    }
}

glib::wrapper! {
    pub struct NcmPaintable(ObjectSubclass<imp::NcmPaintable>)
    @implements gdk::Paintable;
}

glib::wrapper! { pub struct NcmImageSourceObject(ObjectSubclass<imp::NcmImageSourceObject>); }

glib::wrapper! { pub struct NcmTexture(ObjectSubclass<imp::NcmTexture>); }

impl NcmPaintable {
    pub fn new(display: &gdk::Display) -> Self {
        let s: Self = glib::Object::builder().build();
        s.imp()
            .icon_theme
            .replace(Some(gtk::IconTheme::for_display(display)));
        s
    }

    pub fn source(&self) -> Option<NcmImageSource> {
        self.property::<Option<NcmImageSourceObject>>("source")
            .map(|s| s.source())
    }

    pub fn set_source(&self, source: NcmImageSource) {
        self.set_property("source", source.to_gobj());
    }

    pub fn set_texture(&self, tex: gdk::Texture) {
        self.set_property("texture", tex);
    }

    pub fn disconnect_lookup(&self) {
        if let Some(dis_fn) = self.imp().disconnect_lookup.replace(None) {
            dis_fn.invoke::<()>(&[]);
        }
    }

    pub fn set_disconnect_lookup(&self, closure: glib::RustClosure) {
        self.imp().disconnect_lookup.replace(Some(closure));
    }

    pub fn emit_texture_loaded(&self, tex: &gdk::Texture) {
        self.emit_by_name::<()>("texture-loaded", &[&tex]);
    }
    pub fn connect_texture_loaded(&self, closure: glib::RustClosure) -> SignalHandlerId {
        self.connect_closure("texture-loaded", false, closure)
    }
}

impl NcmImageSourceObject {
    pub fn new(source: NcmImageSource) -> Self {
        let s: Self = glib::Object::builder().build();
        s.imp().source.set(source).unwrap();
        s
    }

    pub fn source(&self) -> NcmImageSource {
        self.imp().source.get().unwrap().clone()
    }
}

impl NcmTexture {
    pub fn new() -> Self {
        glib::Object::builder().build()
    }
    pub fn texture(&self) -> Option<gdk::Texture> {
        self.imp().texture.borrow().upgrade()
    }
    pub fn set_texture(&self, tex: &gdk::Texture) {
        self.imp().texture.replace(tex.downgrade());
        self.emit_texture_loaded(&tex);
    }
    pub fn emit_texture_loaded(&self, tex: &gdk::Texture) {
        self.emit_by_name::<()>("texture-loaded", &[&tex]);
    }
    pub fn connect_texture_loaded(&self, closure: glib::RustClosure) -> SignalHandlerId {
        self.connect_closure("texture-loaded", false, closure)
    }
}

mod imp {

    use super::*;

    // NcmPaintableLoader imp

    #[derive(Default)]
    pub struct NcmPaintableLoader {
        pub textures: RefCell<std::collections::HashMap<String, super::NcmTexture>>,

        pub sender: OnceCell<Sender<Action>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for NcmPaintableLoader {
        const NAME: &'static str = "NcmPaintableLoader";
        type Type = super::NcmPaintableLoader;
    }

    impl ObjectImpl for NcmPaintableLoader {
        fn constructed(&self) {
            self.parent_constructed();
            let _obj = self.obj();
        }
    }

    // NcmTexture imp

    #[derive(Default)]
    pub struct NcmTexture {
        pub texture: RefCell<glib::WeakRef<gdk::Texture>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for NcmTexture {
        const NAME: &'static str = "NcmTexture";
        type Type = super::NcmTexture;
    }

    impl ObjectImpl for NcmTexture {
        fn signals() -> &'static [Signal] {
            static SIGNALS: Lazy<Vec<Signal>> = Lazy::new(|| {
                vec![Signal::builder("texture-loaded")
                    .param_types([gdk::Texture::static_type()])
                    .build()]
            });
            SIGNALS.as_ref()
        }
    }

    // NcmImageSourceObject imp

    #[derive(Default)]
    pub struct NcmImageSourceObject {
        pub source: OnceCell<NcmImageSource>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for NcmImageSourceObject {
        const NAME: &'static str = "NcmImageSourceObject";
        type Type = super::NcmImageSourceObject;
    }
    impl ObjectImpl for NcmImageSourceObject {}

    // NcmPaintable imp

    #[derive(Default)]
    pub struct NcmPaintable {
        pub icon_theme: RefCell<Option<gtk::IconTheme>>,
        pub loader: OnceCell<super::NcmPaintableLoader>,
        pub disconnect_lookup: RefCell<Option<glib::RustClosure>>,

        texture: RefCell<Option<gdk::Texture>>,
        source: RefCell<Option<super::NcmImageSourceObject>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for NcmPaintable {
        const NAME: &'static str = "NcmPaintable";
        type Type = super::NcmPaintable;
        type Interfaces = (gdk::Paintable,);
    }

    impl ObjectImpl for NcmPaintable {
        fn constructed(&self) {
            self.parent_constructed();

            self.loader
                .set(PAINTABLE_LOADER_REF.get().unwrap().upgrade().unwrap())
                .unwrap();

            let _obj = self.obj();
        }

        fn properties() -> &'static [ParamSpec] {
            static PROPERTIES: Lazy<Vec<ParamSpec>> = Lazy::new(|| {
                vec![
                    ParamSpecObject::builder::<gdk::Texture>("texture").build(),
                    ParamSpecObject::builder::<super::NcmImageSourceObject>("source").build(),
                ]
            });
            PROPERTIES.as_ref()
        }

        fn set_property(&self, _id: usize, value: &Value, pspec: &ParamSpec) {
            match pspec.name() {
                "texture" => {
                    let val: Option<gdk::Texture> = value.get().unwrap();
                    if let Some(val) = &val {
                        self.obj().emit_texture_loaded(val);
                    }
                    self.texture.replace(val);
                    self.obj().invalidate_contents();
                }
                "source" => {
                    let val: Option<super::NcmImageSourceObject> = value.get().unwrap();

                    let changed = {
                        self.source.borrow().as_ref().map(|s| s.source())
                            != val.as_ref().map(|s| s.source())
                    };

                    if changed {
                        self.source.replace(val.clone());
                        self.texture.replace(None);
                        self.obj().invalidate_contents();

                        if let Some(val) = val {
                            let loader = self.loader.get().unwrap();
                            loader.look_up(&self.obj(), val.source());
                        }
                    }
                }
                n => unimplemented!("{}", n),
            }
        }

        fn property(&self, _id: usize, pspec: &ParamSpec) -> Value {
            match pspec.name() {
                "texture" => self.texture.borrow().to_value(),
                "source" => self.source.borrow().to_value(),
                n => unimplemented!("{}", n),
            }
        }
        fn signals() -> &'static [Signal] {
            static SIGNALS: Lazy<Vec<Signal>> = Lazy::new(|| {
                vec![Signal::builder("texture-loaded")
                    .param_types([gdk::Texture::static_type()])
                    .build()]
            });
            SIGNALS.as_ref()
        }
    }

    impl PaintableImpl for NcmPaintable {
        fn snapshot(&self, snapshot: &gdk::Snapshot, width: f64, height: f64) {
            if let Some(tex) = self.texture.borrow().as_ref() {
                let rect = gtk::graphene::Rect::new(0.0, 0.0, width as f32, height as f32);
                snapshot.append_texture(tex, &rect);
            } else if let Some(icon_theme) = self.icon_theme.borrow().as_ref() {
                let (iw, ih) = {
                    let size = width.min(height) * 0.9;
                    (size, size)
                };
                let icon = icon_theme.lookup_icon(
                    "image-missing",
                    &[],
                    iw.max(ih) as i32,
                    1,
                    TextDirection::Ltr,
                    IconLookupFlags::PRELOAD,
                );
                snapshot.translate(&gtk::graphene::Point::new(
                    (width - iw) as f32 / 2.0,
                    (height - ih) as f32 / 2.0,
                ));
                icon.snapshot(snapshot, iw, ih);
            }
        }
        fn intrinsic_width(&self) -> i32 {
            if let Some(source) = self.source.borrow().as_ref().map(|s| s.source()) {
                source.size().0 as i32
            } else {
                self.parent_intrinsic_width()
            }
        }

        fn intrinsic_height(&self) -> i32 {
            if let Some(source) = self.source.borrow().as_ref().map(|s| s.source()) {
                source.size().1 as i32
            } else {
                self.parent_intrinsic_height()
            }
        }
    }
}
