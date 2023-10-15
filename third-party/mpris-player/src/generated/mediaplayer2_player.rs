#![allow(dead_code)]
use dbus as dbus;
use dbus::arg;
use dbus::tree;

pub trait OrgMprisMediaPlayer2Player {
    type Err;
    fn next(&self) -> Result<(), Self::Err>;
    fn previous(&self) -> Result<(), Self::Err>;
    fn pause(&self) -> Result<(), Self::Err>;
    fn play_pause(&self) -> Result<(), Self::Err>;
    fn stop(&self) -> Result<(), Self::Err>;
    fn play(&self) -> Result<(), Self::Err>;
    fn seek(&self, offset: i64) -> Result<(), Self::Err>;
    fn set_position(&self, track_id: dbus::Path, position: i64) -> Result<(), Self::Err>;
    fn open_uri(&self, uri: &str) -> Result<(), Self::Err>;
    fn get_playback_status(&self) -> Result<String, Self::Err>;
    fn get_loop_status(&self) -> Result<String, Self::Err>;
    fn set_loop_status(&self, value: String) -> Result<(), Self::Err>;
    fn get_rate(&self) -> Result<f64, Self::Err>;
    fn set_rate(&self, value: f64) -> Result<(), Self::Err>;
    fn get_shuffle(&self) -> Result<bool, Self::Err>;
    fn set_shuffle(&self, value: bool) -> Result<(), Self::Err>;
    fn get_metadata(&self) -> Result<::std::collections::HashMap<String, arg::Variant<Box<arg::RefArg + 'static>>>, Self::Err>;
    fn get_volume(&self) -> Result<f64, Self::Err>;
    fn set_volume(&self, value: f64) -> Result<(), Self::Err>;
    fn get_position(&self) -> Result<i64, Self::Err>;
    fn get_minimum_rate(&self) -> Result<f64, Self::Err>;
    fn get_maximum_rate(&self) -> Result<f64, Self::Err>;
    fn get_can_go_next(&self) -> Result<bool, Self::Err>;
    fn get_can_go_previous(&self) -> Result<bool, Self::Err>;
    fn get_can_play(&self) -> Result<bool, Self::Err>;
    fn get_can_pause(&self) -> Result<bool, Self::Err>;
    fn get_can_seek(&self) -> Result<bool, Self::Err>;
    fn get_can_control(&self) -> Result<bool, Self::Err>;
}

impl<'a, C: ::std::ops::Deref<Target=dbus::Connection>> OrgMprisMediaPlayer2Player for dbus::ConnPath<'a, C> {
    type Err = dbus::Error;

    fn next(&self) -> Result<(), Self::Err> {
        let mut m = try!(self.method_call_with_args(&"org.mpris.MediaPlayer2.Player".into(), &"Next".into(), |_| {
        }));
        try!(m.as_result());
        Ok(())
    }

    fn previous(&self) -> Result<(), Self::Err> {
        let mut m = try!(self.method_call_with_args(&"org.mpris.MediaPlayer2.Player".into(), &"Previous".into(), |_| {
        }));
        try!(m.as_result());
        Ok(())
    }

    fn pause(&self) -> Result<(), Self::Err> {
        let mut m = try!(self.method_call_with_args(&"org.mpris.MediaPlayer2.Player".into(), &"Pause".into(), |_| {
        }));
        try!(m.as_result());
        Ok(())
    }

    fn play_pause(&self) -> Result<(), Self::Err> {
        let mut m = try!(self.method_call_with_args(&"org.mpris.MediaPlayer2.Player".into(), &"PlayPause".into(), |_| {
        }));
        try!(m.as_result());
        Ok(())
    }

    fn stop(&self) -> Result<(), Self::Err> {
        let mut m = try!(self.method_call_with_args(&"org.mpris.MediaPlayer2.Player".into(), &"Stop".into(), |_| {
        }));
        try!(m.as_result());
        Ok(())
    }

    fn play(&self) -> Result<(), Self::Err> {
        let mut m = try!(self.method_call_with_args(&"org.mpris.MediaPlayer2.Player".into(), &"Play".into(), |_| {
        }));
        try!(m.as_result());
        Ok(())
    }

    fn seek(&self, offset: i64) -> Result<(), Self::Err> {
        let mut m = try!(self.method_call_with_args(&"org.mpris.MediaPlayer2.Player".into(), &"Seek".into(), |msg| {
            let mut i = arg::IterAppend::new(msg);
            i.append(offset);
        }));
        try!(m.as_result());
        Ok(())
    }

    fn set_position(&self, track_id: dbus::Path, position: i64) -> Result<(), Self::Err> {
        let mut m = try!(self.method_call_with_args(&"org.mpris.MediaPlayer2.Player".into(), &"SetPosition".into(), |msg| {
            let mut i = arg::IterAppend::new(msg);
            i.append(track_id);
            i.append(position);
        }));
        try!(m.as_result());
        Ok(())
    }

    fn open_uri(&self, uri: &str) -> Result<(), Self::Err> {
        let mut m = try!(self.method_call_with_args(&"org.mpris.MediaPlayer2.Player".into(), &"OpenUri".into(), |msg| {
            let mut i = arg::IterAppend::new(msg);
            i.append(uri);
        }));
        try!(m.as_result());
        Ok(())
    }

    fn get_playback_status(&self) -> Result<String, Self::Err> {
        <Self as dbus::stdintf::org_freedesktop_dbus::Properties>::get(&self, "org.mpris.MediaPlayer2.Player", "PlaybackStatus")
    }

    fn get_loop_status(&self) -> Result<String, Self::Err> {
        <Self as dbus::stdintf::org_freedesktop_dbus::Properties>::get(&self, "org.mpris.MediaPlayer2.Player", "LoopStatus")
    }

    fn get_rate(&self) -> Result<f64, Self::Err> {
        <Self as dbus::stdintf::org_freedesktop_dbus::Properties>::get(&self, "org.mpris.MediaPlayer2.Player", "Rate")
    }

    fn get_shuffle(&self) -> Result<bool, Self::Err> {
        <Self as dbus::stdintf::org_freedesktop_dbus::Properties>::get(&self, "org.mpris.MediaPlayer2.Player", "Shuffle")
    }

    fn get_metadata(&self) -> Result<::std::collections::HashMap<String, arg::Variant<Box<arg::RefArg + 'static>>>, Self::Err> {
        <Self as dbus::stdintf::org_freedesktop_dbus::Properties>::get(&self, "org.mpris.MediaPlayer2.Player", "Metadata")
    }

    fn get_volume(&self) -> Result<f64, Self::Err> {
        <Self as dbus::stdintf::org_freedesktop_dbus::Properties>::get(&self, "org.mpris.MediaPlayer2.Player", "Volume")
    }

    fn get_position(&self) -> Result<i64, Self::Err> {
        <Self as dbus::stdintf::org_freedesktop_dbus::Properties>::get(&self, "org.mpris.MediaPlayer2.Player", "Position")
    }

    fn get_minimum_rate(&self) -> Result<f64, Self::Err> {
        <Self as dbus::stdintf::org_freedesktop_dbus::Properties>::get(&self, "org.mpris.MediaPlayer2.Player", "MinimumRate")
    }

    fn get_maximum_rate(&self) -> Result<f64, Self::Err> {
        <Self as dbus::stdintf::org_freedesktop_dbus::Properties>::get(&self, "org.mpris.MediaPlayer2.Player", "MaximumRate")
    }

    fn get_can_go_next(&self) -> Result<bool, Self::Err> {
        <Self as dbus::stdintf::org_freedesktop_dbus::Properties>::get(&self, "org.mpris.MediaPlayer2.Player", "CanGoNext")
    }

    fn get_can_go_previous(&self) -> Result<bool, Self::Err> {
        <Self as dbus::stdintf::org_freedesktop_dbus::Properties>::get(&self, "org.mpris.MediaPlayer2.Player", "CanGoPrevious")
    }

    fn get_can_play(&self) -> Result<bool, Self::Err> {
        <Self as dbus::stdintf::org_freedesktop_dbus::Properties>::get(&self, "org.mpris.MediaPlayer2.Player", "CanPlay")
    }

    fn get_can_pause(&self) -> Result<bool, Self::Err> {
        <Self as dbus::stdintf::org_freedesktop_dbus::Properties>::get(&self, "org.mpris.MediaPlayer2.Player", "CanPause")
    }

    fn get_can_seek(&self) -> Result<bool, Self::Err> {
        <Self as dbus::stdintf::org_freedesktop_dbus::Properties>::get(&self, "org.mpris.MediaPlayer2.Player", "CanSeek")
    }

    fn get_can_control(&self) -> Result<bool, Self::Err> {
        <Self as dbus::stdintf::org_freedesktop_dbus::Properties>::get(&self, "org.mpris.MediaPlayer2.Player", "CanControl")
    }

    fn set_loop_status(&self, value: String) -> Result<(), Self::Err> {
        <Self as dbus::stdintf::org_freedesktop_dbus::Properties>::set(&self, "org.mpris.MediaPlayer2.Player", "LoopStatus", value)
    }

    fn set_rate(&self, value: f64) -> Result<(), Self::Err> {
        <Self as dbus::stdintf::org_freedesktop_dbus::Properties>::set(&self, "org.mpris.MediaPlayer2.Player", "Rate", value)
    }

    fn set_shuffle(&self, value: bool) -> Result<(), Self::Err> {
        <Self as dbus::stdintf::org_freedesktop_dbus::Properties>::set(&self, "org.mpris.MediaPlayer2.Player", "Shuffle", value)
    }

    fn set_volume(&self, value: f64) -> Result<(), Self::Err> {
        <Self as dbus::stdintf::org_freedesktop_dbus::Properties>::set(&self, "org.mpris.MediaPlayer2.Player", "Volume", value)
    }
}

pub fn org_mpris_media_player2_player_server<F, T, D>(factory: &tree::Factory<tree::MTFn<D>, D>, data: D::Interface, f: F) -> tree::Interface<tree::MTFn<D>, D>
where
    D: tree::DataType,
    D::Method: Default,
    D::Property: Default,
    D::Signal: Default,
    T: OrgMprisMediaPlayer2Player<Err=tree::MethodErr>,
    F: 'static + for <'z> Fn(& 'z tree::MethodInfo<tree::MTFn<D>, D>) -> & 'z T,
{
    let i = factory.interface("org.mpris.MediaPlayer2.Player", data);
    let f = ::std::sync::Arc::new(f);
    let fclone = f.clone();
    let h = move |minfo: &tree::MethodInfo<tree::MTFn<D>, D>| {
        let d = fclone(minfo);
        try!(d.next());
        let rm = minfo.msg.method_return();
        Ok(vec!(rm))
    };
    let m = factory.method("Next", Default::default(), h);
    let i = i.add_m(m);

    let fclone = f.clone();
    let h = move |minfo: &tree::MethodInfo<tree::MTFn<D>, D>| {
        let d = fclone(minfo);
        try!(d.previous());
        let rm = minfo.msg.method_return();
        Ok(vec!(rm))
    };
    let m = factory.method("Previous", Default::default(), h);
    let i = i.add_m(m);

    let fclone = f.clone();
    let h = move |minfo: &tree::MethodInfo<tree::MTFn<D>, D>| {
        let d = fclone(minfo);
        try!(d.pause());
        let rm = minfo.msg.method_return();
        Ok(vec!(rm))
    };
    let m = factory.method("Pause", Default::default(), h);
    let i = i.add_m(m);

    let fclone = f.clone();
    let h = move |minfo: &tree::MethodInfo<tree::MTFn<D>, D>| {
        let d = fclone(minfo);
        try!(d.play_pause());
        let rm = minfo.msg.method_return();
        Ok(vec!(rm))
    };
    let m = factory.method("PlayPause", Default::default(), h);
    let i = i.add_m(m);

    let fclone = f.clone();
    let h = move |minfo: &tree::MethodInfo<tree::MTFn<D>, D>| {
        let d = fclone(minfo);
        try!(d.stop());
        let rm = minfo.msg.method_return();
        Ok(vec!(rm))
    };
    let m = factory.method("Stop", Default::default(), h);
    let i = i.add_m(m);

    let fclone = f.clone();
    let h = move |minfo: &tree::MethodInfo<tree::MTFn<D>, D>| {
        let d = fclone(minfo);
        try!(d.play());
        let rm = minfo.msg.method_return();
        Ok(vec!(rm))
    };
    let m = factory.method("Play", Default::default(), h);
    let i = i.add_m(m);

    let fclone = f.clone();
    let h = move |minfo: &tree::MethodInfo<tree::MTFn<D>, D>| {
        let mut i = minfo.msg.iter_init();
        let offset: i64 = try!(i.read());
        let d = fclone(minfo);
        try!(d.seek(offset));
        let rm = minfo.msg.method_return();
        Ok(vec!(rm))
    };
    let m = factory.method("Seek", Default::default(), h);
    let m = m.in_arg(("Offset", "x"));
    let i = i.add_m(m);

    let fclone = f.clone();
    let h = move |minfo: &tree::MethodInfo<tree::MTFn<D>, D>| {
        let mut i = minfo.msg.iter_init();
        let track_id: dbus::Path = try!(i.read());
        let position: i64 = try!(i.read());
        let d = fclone(minfo);
        try!(d.set_position(track_id, position));
        let rm = minfo.msg.method_return();
        Ok(vec!(rm))
    };
    let m = factory.method("SetPosition", Default::default(), h);
    let m = m.in_arg(("TrackId", "o"));
    let m = m.in_arg(("Position", "x"));
    let i = i.add_m(m);

    let fclone = f.clone();
    let h = move |minfo: &tree::MethodInfo<tree::MTFn<D>, D>| {
        let mut i = minfo.msg.iter_init();
        let uri: &str = try!(i.read());
        let d = fclone(minfo);
        try!(d.open_uri(uri));
        let rm = minfo.msg.method_return();
        Ok(vec!(rm))
    };
    let m = factory.method("OpenUri", Default::default(), h);
    let m = m.in_arg(("Uri", "s"));
    let i = i.add_m(m);

    let p = factory.property::<&str, _>("PlaybackStatus", Default::default());
    let p = p.access(tree::Access::Read);
    let fclone = f.clone();
    let p = p.on_get(move |a, pinfo| {
        let minfo = pinfo.to_method_info();
        let d = fclone(&minfo);
        a.append(try!(d.get_playback_status()));
        Ok(())
    });
    let i = i.add_p(p);

    let p = factory.property::<&str, _>("LoopStatus", Default::default());
    let p = p.access(tree::Access::ReadWrite);
    let fclone = f.clone();
    let p = p.on_get(move |a, pinfo| {
        let minfo = pinfo.to_method_info();
        let d = fclone(&minfo);
        a.append(try!(d.get_loop_status()));
        Ok(())
    });
    let fclone = f.clone();
    let p = p.on_set(move |iter, pinfo| {
        let minfo = pinfo.to_method_info();
        let d = fclone(&minfo);
        try!(d.set_loop_status(try!(iter.read())));
        Ok(())
    });
    let i = i.add_p(p);

    let p = factory.property::<f64, _>("Rate", Default::default());
    let p = p.access(tree::Access::ReadWrite);
    let fclone = f.clone();
    let p = p.on_get(move |a, pinfo| {
        let minfo = pinfo.to_method_info();
        let d = fclone(&minfo);
        a.append(try!(d.get_rate()));
        Ok(())
    });
    let fclone = f.clone();
    let p = p.on_set(move |iter, pinfo| {
        let minfo = pinfo.to_method_info();
        let d = fclone(&minfo);
        try!(d.set_rate(try!(iter.read())));
        Ok(())
    });
    let i = i.add_p(p);

    let p = factory.property::<bool, _>("Shuffle", Default::default());
    let p = p.access(tree::Access::ReadWrite);
    let fclone = f.clone();
    let p = p.on_get(move |a, pinfo| {
        let minfo = pinfo.to_method_info();
        let d = fclone(&minfo);
        a.append(try!(d.get_shuffle()));
        Ok(())
    });
    let fclone = f.clone();
    let p = p.on_set(move |iter, pinfo| {
        let minfo = pinfo.to_method_info();
        let d = fclone(&minfo);
        try!(d.set_shuffle(try!(iter.read())));
        Ok(())
    });
    let i = i.add_p(p);

    let p = factory.property::<::std::collections::HashMap<&str, arg::Variant<Box<arg::RefArg>>>, _>("Metadata", Default::default());
    let p = p.access(tree::Access::Read);
    let fclone = f.clone();
    let p = p.on_get(move |a, pinfo| {
        let minfo = pinfo.to_method_info();
        let d = fclone(&minfo);
        a.append(try!(d.get_metadata()));
        Ok(())
    });
    let i = i.add_p(p);

    let p = factory.property::<f64, _>("Volume", Default::default());
    let p = p.access(tree::Access::ReadWrite);
    let fclone = f.clone();
    let p = p.on_get(move |a, pinfo| {
        let minfo = pinfo.to_method_info();
        let d = fclone(&minfo);
        a.append(try!(d.get_volume()));
        Ok(())
    });
    let fclone = f.clone();
    let p = p.on_set(move |iter, pinfo| {
        let minfo = pinfo.to_method_info();
        let d = fclone(&minfo);
        try!(d.set_volume(try!(iter.read())));
        Ok(())
    });
    let i = i.add_p(p);

    let p = factory.property::<i64, _>("Position", Default::default());
    let p = p.access(tree::Access::Read);
    let fclone = f.clone();
    let p = p.on_get(move |a, pinfo| {
        let minfo = pinfo.to_method_info();
        let d = fclone(&minfo);
        a.append(try!(d.get_position()));
        Ok(())
    });
    let i = i.add_p(p);

    let p = factory.property::<f64, _>("MinimumRate", Default::default());
    let p = p.access(tree::Access::Read);
    let fclone = f.clone();
    let p = p.on_get(move |a, pinfo| {
        let minfo = pinfo.to_method_info();
        let d = fclone(&minfo);
        a.append(try!(d.get_minimum_rate()));
        Ok(())
    });
    let i = i.add_p(p);

    let p = factory.property::<f64, _>("MaximumRate", Default::default());
    let p = p.access(tree::Access::Read);
    let fclone = f.clone();
    let p = p.on_get(move |a, pinfo| {
        let minfo = pinfo.to_method_info();
        let d = fclone(&minfo);
        a.append(try!(d.get_maximum_rate()));
        Ok(())
    });
    let i = i.add_p(p);

    let p = factory.property::<bool, _>("CanGoNext", Default::default());
    let p = p.access(tree::Access::Read);
    let fclone = f.clone();
    let p = p.on_get(move |a, pinfo| {
        let minfo = pinfo.to_method_info();
        let d = fclone(&minfo);
        a.append(try!(d.get_can_go_next()));
        Ok(())
    });
    let i = i.add_p(p);

    let p = factory.property::<bool, _>("CanGoPrevious", Default::default());
    let p = p.access(tree::Access::Read);
    let fclone = f.clone();
    let p = p.on_get(move |a, pinfo| {
        let minfo = pinfo.to_method_info();
        let d = fclone(&minfo);
        a.append(try!(d.get_can_go_previous()));
        Ok(())
    });
    let i = i.add_p(p);

    let p = factory.property::<bool, _>("CanPlay", Default::default());
    let p = p.access(tree::Access::Read);
    let fclone = f.clone();
    let p = p.on_get(move |a, pinfo| {
        let minfo = pinfo.to_method_info();
        let d = fclone(&minfo);
        a.append(try!(d.get_can_play()));
        Ok(())
    });
    let i = i.add_p(p);

    let p = factory.property::<bool, _>("CanPause", Default::default());
    let p = p.access(tree::Access::Read);
    let fclone = f.clone();
    let p = p.on_get(move |a, pinfo| {
        let minfo = pinfo.to_method_info();
        let d = fclone(&minfo);
        a.append(try!(d.get_can_pause()));
        Ok(())
    });
    let i = i.add_p(p);

    let p = factory.property::<bool, _>("CanSeek", Default::default());
    let p = p.access(tree::Access::Read);
    let fclone = f.clone();
    let p = p.on_get(move |a, pinfo| {
        let minfo = pinfo.to_method_info();
        let d = fclone(&minfo);
        a.append(try!(d.get_can_seek()));
        Ok(())
    });
    let i = i.add_p(p);

    let p = factory.property::<bool, _>("CanControl", Default::default());
    let p = p.access(tree::Access::Read);
    let fclone = f.clone();
    let p = p.on_get(move |a, pinfo| {
        let minfo = pinfo.to_method_info();
        let d = fclone(&minfo);
        a.append(try!(d.get_can_control()));
        Ok(())
    });
    let i = i.add_p(p);
    let s = factory.signal("Seeked", Default::default());
    let s = s.arg(("Position", "x"));
    let i = i.add_s(s);
    i
}

#[derive(Debug, Default)]
pub struct OrgMprisMediaPlayer2PlayerSeeked {
    pub position: i64,
}

impl dbus::SignalArgs for OrgMprisMediaPlayer2PlayerSeeked {
    const NAME: &'static str = "Seeked";
    const INTERFACE: &'static str = "org.mpris.MediaPlayer2.Player";
    fn append(&self, i: &mut arg::IterAppend) {
        (&self.position as &arg::RefArg).append(i);
    }
    fn get(&mut self, i: &mut arg::Iter) -> Result<(), arg::TypeMismatchError> {
        self.position = try!(i.read());
        Ok(())
    }
}

#[derive(Debug, Default)]
pub struct OrgFreedesktopDBusPropertiesPropertiesChanged {
    pub interface_name: String,
    pub changed_properties: ::std::collections::HashMap<String, arg::Variant<Box<arg::RefArg + 'static>>>,
    pub invalidated_properties: Vec<String>,
}

impl dbus::SignalArgs for OrgFreedesktopDBusPropertiesPropertiesChanged {
    const NAME: &'static str = "PropertiesChanged";
    const INTERFACE: &'static str = "org.freedesktop.DBus.Properties";
    fn append(&self, i: &mut arg::IterAppend) {
        (&self.interface_name as &arg::RefArg).append(i);
        (&self.changed_properties as &arg::RefArg).append(i);
        (&self.invalidated_properties as &arg::RefArg).append(i);
    }
    fn get(&mut self, i: &mut arg::Iter) -> Result<(), arg::TypeMismatchError> {
        self.interface_name = try!(i.read());
        self.changed_properties = try!(i.read());
        self.invalidated_properties = try!(i.read());
        Ok(())
    }
}
