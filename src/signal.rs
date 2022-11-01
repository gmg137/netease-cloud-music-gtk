//
// songlist_row.rs
// Copyright (C) 2022 gmg137 <gmg137 AT live.com>
// Distributed under terms of the GPL-3.0-or-later license.
//
//

use glib::subclass::Signal;
use glib::{ ParamSpec, RustClosure, SignalHandlerId, Value};
use gtk::glib;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use once_cell::sync::{Lazy, OnceCell};

pub static NCM_GSIGNAL: OnceCell<NcmGSignalWrapper> = OnceCell::new();
static _NCM_GSIGNAL: OnceCell<NcmGSignal> = OnceCell::new();

#[derive(Debug)]
pub struct NcmGSignalWrapper {
    sig: glib::SendWeakRef<NcmGSignal>,
}

impl NcmGSignalWrapper {
    pub fn init_global() {
        let sig = NcmGSignal::new();
        let s = Self {
            sig: glib::SendWeakRef::from(sig.downgrade()),
        };
        NCM_GSIGNAL.set(s).unwrap();
        _NCM_GSIGNAL.set(sig).unwrap();
    }

    pub fn emit_play(&self, id: u64, album_id: u64, mix_id: u64) {
        self.sig
            .upgrade()
            .unwrap()
            .emit_by_name::<()>("ncm-play", &[&id, &album_id, &mix_id]);
    }
    pub fn emit_like(&self, id: u64, val: bool) {
        self.sig
            .upgrade()
            .unwrap()
            .emit_by_name::<()>("ncm-like", &[&id, &val]);
    }

    pub fn connect_play(&self, f: RustClosure) -> SignalHandlerId {
        self.sig
            .upgrade()
            .unwrap()
            .connect_closure("ncm-play", false, f)
    }
    pub fn connect_like(&self, f: RustClosure) -> SignalHandlerId {
        self.sig
            .upgrade()
            .unwrap()
            .connect_closure("ncm-like", false, f)
    }

    pub fn connect_logout(&self, f: RustClosure) -> SignalHandlerId {
        self.sig
            .upgrade()
            .unwrap()
            .connect_closure("ncm-logout", false, f)
    }
}

glib::wrapper! {
    pub struct NcmGSignal(ObjectSubclass<imp::NcmGSignal>);
}

impl NcmGSignal {
    pub fn new() -> Self {
        glib::Object::builder().build()
    }
}

mod imp {

    use super::*;

    #[derive(Default)]
    pub struct NcmGSignal {}

    #[glib::object_subclass]
    impl ObjectSubclass for NcmGSignal {
        const NAME: &'static str = "NcmGSignal";
        type Type = super::NcmGSignal;
    }

    impl ObjectImpl for NcmGSignal {
        fn signals() -> &'static [Signal] {
            static SIGNALS: Lazy<Vec<Signal>> = Lazy::new(|| {
                vec![
                    Signal::builder("ncm-play")
                        .param_types([u64::static_type(), u64::static_type(), u64::static_type()])
                        .build(),
                    Signal::builder("ncm-like")
                        .param_types([u64::static_type(), bool::static_type()])
                        .build(),
                    Signal::builder("ncm-logout").build(),
                ]
            });
            SIGNALS.as_ref()
        }

        fn properties() -> &'static [ParamSpec] {
            static PROPERTIES: Lazy<Vec<ParamSpec>> = Lazy::new(|| vec![]);
            PROPERTIES.as_ref()
        }

        fn set_property(&self, _id: usize, _value: &Value, pspec: &ParamSpec) {
            match pspec.name() {
                _ => unimplemented!(),
            }
        }

        fn property(&self, _id: usize, pspec: &ParamSpec) -> Value {
            match pspec.name() {
                _ => unimplemented!(),
            }
        }
    }
}
