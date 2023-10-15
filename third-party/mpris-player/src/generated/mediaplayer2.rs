#![allow(dead_code)]
use dbus as dbus;
use dbus::tree;

pub trait OrgMprisMediaPlayer2 {
    type Err;
    fn raise(&self) -> Result<(), Self::Err>;
    fn quit(&self) -> Result<(), Self::Err>;
    fn get_can_quit(&self) -> Result<bool, Self::Err>;
    fn get_fullscreen(&self) -> Result<bool, Self::Err>;
    fn set_fullscreen(&self, value: bool) -> Result<(), Self::Err>;
    fn get_can_set_fullscreen(&self) -> Result<bool, Self::Err>;
    fn get_can_raise(&self) -> Result<bool, Self::Err>;
    fn get_has_track_list(&self) -> Result<bool, Self::Err>;
    fn get_identity(&self) -> Result<String, Self::Err>;
    fn get_desktop_entry(&self) -> Result<String, Self::Err>;
    fn get_supported_uri_schemes(&self) -> Result<Vec<String>, Self::Err>;
    fn get_supported_mime_types(&self) -> Result<Vec<String>, Self::Err>;
}

impl<'a, C: ::std::ops::Deref<Target=dbus::Connection>> OrgMprisMediaPlayer2 for dbus::ConnPath<'a, C> {
    type Err = dbus::Error;

    fn raise(&self) -> Result<(), Self::Err> {
        let mut m = try!(self.method_call_with_args(&"org.mpris.MediaPlayer2".into(), &"Raise".into(), |_| {
        }));
        try!(m.as_result());
        Ok(())
    }

    fn quit(&self) -> Result<(), Self::Err> {
        let mut m = try!(self.method_call_with_args(&"org.mpris.MediaPlayer2".into(), &"Quit".into(), |_| {
        }));
        try!(m.as_result());
        Ok(())
    }

    fn get_can_quit(&self) -> Result<bool, Self::Err> {
        <Self as dbus::stdintf::org_freedesktop_dbus::Properties>::get(&self, "org.mpris.MediaPlayer2", "CanQuit")
    }

    fn get_fullscreen(&self) -> Result<bool, Self::Err> {
        <Self as dbus::stdintf::org_freedesktop_dbus::Properties>::get(&self, "org.mpris.MediaPlayer2", "Fullscreen")
    }

    fn get_can_set_fullscreen(&self) -> Result<bool, Self::Err> {
        <Self as dbus::stdintf::org_freedesktop_dbus::Properties>::get(&self, "org.mpris.MediaPlayer2", "CanSetFullscreen")
    }

    fn get_can_raise(&self) -> Result<bool, Self::Err> {
        <Self as dbus::stdintf::org_freedesktop_dbus::Properties>::get(&self, "org.mpris.MediaPlayer2", "CanRaise")
    }

    fn get_has_track_list(&self) -> Result<bool, Self::Err> {
        <Self as dbus::stdintf::org_freedesktop_dbus::Properties>::get(&self, "org.mpris.MediaPlayer2", "HasTrackList")
    }

    fn get_identity(&self) -> Result<String, Self::Err> {
        <Self as dbus::stdintf::org_freedesktop_dbus::Properties>::get(&self, "org.mpris.MediaPlayer2", "Identity")
    }

    fn get_desktop_entry(&self) -> Result<String, Self::Err> {
        <Self as dbus::stdintf::org_freedesktop_dbus::Properties>::get(&self, "org.mpris.MediaPlayer2", "DesktopEntry")
    }

    fn get_supported_uri_schemes(&self) -> Result<Vec<String>, Self::Err> {
        <Self as dbus::stdintf::org_freedesktop_dbus::Properties>::get(&self, "org.mpris.MediaPlayer2", "SupportedUriSchemes")
    }

    fn get_supported_mime_types(&self) -> Result<Vec<String>, Self::Err> {
        <Self as dbus::stdintf::org_freedesktop_dbus::Properties>::get(&self, "org.mpris.MediaPlayer2", "SupportedMimeTypes")
    }

    fn set_fullscreen(&self, value: bool) -> Result<(), Self::Err> {
        <Self as dbus::stdintf::org_freedesktop_dbus::Properties>::set(&self, "org.mpris.MediaPlayer2", "Fullscreen", value)
    }
}

pub fn org_mpris_media_player2_server<F, T, D>(factory: &tree::Factory<tree::MTFn<D>, D>, data: D::Interface, f: F) -> tree::Interface<tree::MTFn<D>, D>
where
    D: tree::DataType,
    D::Method: Default,
    D::Property: Default,
    T: OrgMprisMediaPlayer2<Err=tree::MethodErr>,
    F: 'static + for <'z> Fn(& 'z tree::MethodInfo<tree::MTFn<D>, D>) -> & 'z T,
{
    let i = factory.interface("org.mpris.MediaPlayer2", data);
    let f = ::std::sync::Arc::new(f);
    let fclone = f.clone();
    let h = move |minfo: &tree::MethodInfo<tree::MTFn<D>, D>| {
        let d = fclone(minfo);
        try!(d.raise());
        let rm = minfo.msg.method_return();
        Ok(vec!(rm))
    };
    let m = factory.method("Raise", Default::default(), h);
    let i = i.add_m(m);

    let fclone = f.clone();
    let h = move |minfo: &tree::MethodInfo<tree::MTFn<D>, D>| {
        let d = fclone(minfo);
        try!(d.quit());
        let rm = minfo.msg.method_return();
        Ok(vec!(rm))
    };
    let m = factory.method("Quit", Default::default(), h);
    let i = i.add_m(m);

    let p = factory.property::<bool, _>("CanQuit", Default::default());
    let p = p.access(tree::Access::Read);
    let fclone = f.clone();
    let p = p.on_get(move |a, pinfo| {
        let minfo = pinfo.to_method_info();
        let d = fclone(&minfo);
        a.append(try!(d.get_can_quit()));
        Ok(())
    });
    let i = i.add_p(p);

    let p = factory.property::<bool, _>("Fullscreen", Default::default());
    let p = p.access(tree::Access::ReadWrite);
    let fclone = f.clone();
    let p = p.on_get(move |a, pinfo| {
        let minfo = pinfo.to_method_info();
        let d = fclone(&minfo);
        a.append(try!(d.get_fullscreen()));
        Ok(())
    });
    let fclone = f.clone();
    let p = p.on_set(move |iter, pinfo| {
        let minfo = pinfo.to_method_info();
        let d = fclone(&minfo);
        try!(d.set_fullscreen(try!(iter.read())));
        Ok(())
    });
    let i = i.add_p(p);

    let p = factory.property::<bool, _>("CanSetFullscreen", Default::default());
    let p = p.access(tree::Access::Read);
    let fclone = f.clone();
    let p = p.on_get(move |a, pinfo| {
        let minfo = pinfo.to_method_info();
        let d = fclone(&minfo);
        a.append(try!(d.get_can_set_fullscreen()));
        Ok(())
    });
    let i = i.add_p(p);

    let p = factory.property::<bool, _>("CanRaise", Default::default());
    let p = p.access(tree::Access::Read);
    let fclone = f.clone();
    let p = p.on_get(move |a, pinfo| {
        let minfo = pinfo.to_method_info();
        let d = fclone(&minfo);
        a.append(try!(d.get_can_raise()));
        Ok(())
    });
    let i = i.add_p(p);

    let p = factory.property::<bool, _>("HasTrackList", Default::default());
    let p = p.access(tree::Access::Read);
    let fclone = f.clone();
    let p = p.on_get(move |a, pinfo| {
        let minfo = pinfo.to_method_info();
        let d = fclone(&minfo);
        a.append(try!(d.get_has_track_list()));
        Ok(())
    });
    let i = i.add_p(p);

    let p = factory.property::<&str, _>("Identity", Default::default());
    let p = p.access(tree::Access::Read);
    let fclone = f.clone();
    let p = p.on_get(move |a, pinfo| {
        let minfo = pinfo.to_method_info();
        let d = fclone(&minfo);
        a.append(try!(d.get_identity()));
        Ok(())
    });
    let i = i.add_p(p);

    let p = factory.property::<&str, _>("DesktopEntry", Default::default());
    let p = p.access(tree::Access::Read);
    let fclone = f.clone();
    let p = p.on_get(move |a, pinfo| {
        let minfo = pinfo.to_method_info();
        let d = fclone(&minfo);
        a.append(try!(d.get_desktop_entry()));
        Ok(())
    });
    let i = i.add_p(p);

    let p = factory.property::<Vec<&str>, _>("SupportedUriSchemes", Default::default());
    let p = p.access(tree::Access::Read);
    let fclone = f.clone();
    let p = p.on_get(move |a, pinfo| {
        let minfo = pinfo.to_method_info();
        let d = fclone(&minfo);
        a.append(try!(d.get_supported_uri_schemes()));
        Ok(())
    });
    let i = i.add_p(p);

    let p = factory.property::<Vec<&str>, _>("SupportedMimeTypes", Default::default());
    let p = p.access(tree::Access::Read);
    let fclone = f.clone();
    let p = p.on_get(move |a, pinfo| {
        let minfo = pinfo.to_method_info();
        let d = fclone(&minfo);
        a.append(try!(d.get_supported_mime_types()));
        Ok(())
    });
    let i = i.add_p(p);
    i
}
