use dbus::arg::{Variant, RefArg};
use dbus::{Connection, BusType, tree, Path, SignalArgs};
use dbus::tree::{Interface, MTFn, Factory};
use std::collections::HashMap;

use std::rc::Rc;
use std::cell::Cell;
use std::cell::RefCell;
use std::sync::Arc;

use crate::generated::mediaplayer2::org_mpris_media_player2_server;
use crate::generated::mediaplayer2_player::{org_mpris_media_player2_player_server, OrgFreedesktopDBusPropertiesPropertiesChanged};

use crate::OrgMprisMediaPlayer2Player;
use crate::OrgMprisMediaPlayer2;

use crate::Metadata;
use crate::PlaybackStatus;
use crate::LoopStatus;

pub struct MprisPlayer{
    connection: Arc<Connection>,
    factory: Arc<Factory<MTFn<TData>, TData>>,

    // OrgMprisMediaPlayer2         Type
    can_quit: Cell<bool>,           // R
    fullscreen: Cell<bool>,         // R/W
    can_set_fullscreen: Cell<bool>, // R
    can_raise: Cell<bool>,          // R
    has_track_list: Cell<bool>,     // R
    identify: String,               // R
    desktop_entry: String,          // R
    supported_uri_schemes: RefCell<Vec<String>>, // R
    supported_mime_types: RefCell<Vec<String>>,  // R

    // OrgMprisMediaPlayer2Player   Type
    playback_status: Cell<PlaybackStatus>, // R
    loop_status: Cell<LoopStatus>,  // R/W
    rate: Cell<f64>,                // R/W
    shuffle: Cell<bool>,            // R/W
    metadata: RefCell<Metadata>,    // R
    volume: Cell<f64>,              // R/W
    position: Cell<i64>,            // R
    minimum_rate: Cell<f64>,        // R
    maximum_rate: Cell<f64>,        // R
    can_go_next: Cell<bool>,        // R
    can_go_previous: Cell<bool>,    // R
    can_play: Cell<bool>,           // R
    can_pause: Cell<bool>,          // R
    can_seek: Cell<bool>,           // R
    can_control: Cell<bool>,        // R

    // Callbacks
    raise_cb: RefCell<Vec<Rc<RefCell<dyn FnMut()>>>>,
    quit_cb: RefCell<Vec<Rc<RefCell<dyn FnMut()>>>>,
    next_cb: RefCell<Vec<Rc<RefCell<dyn FnMut()>>>>,
    previous_cb: RefCell<Vec<Rc<RefCell<dyn FnMut()>>>>,
    pause_cb: RefCell<Vec<Rc<RefCell<dyn FnMut()>>>>,
    play_pause_cb: RefCell<Vec<Rc<RefCell<dyn FnMut()>>>>,
    stop_cb: RefCell<Vec<Rc<RefCell<dyn FnMut()>>>>,
    play_cb: RefCell<Vec<Rc<RefCell<dyn FnMut()>>>>,
    seek_cb: RefCell<Vec<Rc<RefCell<dyn FnMut(i64)>>>>,
    open_uri_cb: RefCell<Vec<Rc<RefCell<dyn FnMut(&str)>>>>,
    fullscreen_cb: RefCell<Vec<Rc<RefCell<dyn FnMut(bool)>>>>,
    loop_status_cb: RefCell<Vec<Rc<RefCell<dyn FnMut(LoopStatus)>>>>,
    rate_cb: RefCell<Vec<Rc<RefCell<dyn FnMut(f64)>>>>,
    shuffle_cb: RefCell<Vec<Rc<RefCell<dyn FnMut(bool)>>>>,
    volume_cb: RefCell<Vec<Rc<RefCell<dyn FnMut(f64)>>>>,
}

impl MprisPlayer{
    pub fn new(mpris_name: String, identify: String, desktop_entry: String) -> Arc<Self>{
        let connection = Arc::new(Connection::get_private(BusType::Session).unwrap());
        let factory = Arc::new(Factory::new_fn());

        let mpris_player = Arc::new(MprisPlayer{
            connection,
            factory,

            can_quit: Cell::new(false),
            fullscreen: Cell::new(false),
            can_set_fullscreen: Cell::new(false),
            can_raise: Cell::new(false),
            has_track_list: Cell::new(false),
            identify,
            desktop_entry,
            supported_uri_schemes: RefCell::new(Vec::new()),
            supported_mime_types: RefCell::new(Vec::new()),

            playback_status: Cell::new(PlaybackStatus::Paused),
            loop_status: Cell::new(LoopStatus::None),
            rate: Cell::new(0_f64),
            shuffle: Cell::new(false),
            metadata: RefCell::new(Metadata::new()),
            volume: Cell::new(0_f64),
            position: Cell::new(0),
            minimum_rate: Cell::new(0_f64),
            maximum_rate: Cell::new(0_f64),
            can_go_next: Cell::new(true),
            can_go_previous: Cell::new(true),
            can_play: Cell::new(true),
            can_pause: Cell::new(true),
            can_seek: Cell::new(false),
            can_control: Cell::new(true),

            raise_cb: RefCell::new(Vec::new()),
            quit_cb: RefCell::new(Vec::new()),
            next_cb: RefCell::new(Vec::new()),
            previous_cb: RefCell::new(Vec::new()),
            pause_cb: RefCell::new(Vec::new()),
            play_pause_cb: RefCell::new(Vec::new()),
            stop_cb: RefCell::new(Vec::new()),
            play_cb: RefCell::new(Vec::new()),
            seek_cb: RefCell::new(Vec::new()),
            open_uri_cb: RefCell::new(Vec::new()),
            fullscreen_cb: RefCell::new(Vec::new()),
            loop_status_cb: RefCell::new(Vec::new()),
            rate_cb: RefCell::new(Vec::new()),
            shuffle_cb: RefCell::new(Vec::new()),
            volume_cb: RefCell::new(Vec::new()),
        });

        // Create OrgMprisMediaPlayer2 interface
        let root_iface: Interface<MTFn<TData>, TData> = org_mpris_media_player2_server(&mpris_player.factory, (), |m| {
            let a: &Arc<MprisPlayer> = m.path.get_data();
            let b: &MprisPlayer = &a;
            b
        });

        // Create OrgMprisMediaPlayer2Player interface
        let player_iface: Interface<MTFn<TData>, TData> = org_mpris_media_player2_player_server(&mpris_player.factory, (), |m| {
            let a: &Arc<MprisPlayer> = m.path.get_data();
            let b: &MprisPlayer = &a;
            b
        });

        // Create dbus tree
        let mut tree = mpris_player.factory.tree(());
        tree = tree.add(mpris_player.factory.object_path("/org/mpris/MediaPlayer2", mpris_player.clone())
            .introspectable()
            .add(root_iface)
            .add(player_iface)
        );

        // Setup dbus connection
        mpris_player.connection.register_name(&format!("org.mpris.MediaPlayer2.{}", mpris_name), 0).unwrap();
        tree.set_registered(&mpris_player.connection, true).unwrap();
        mpris_player.connection.add_handler(tree);

        let connection = mpris_player.connection.clone();
        glib::source::timeout_add_local(250, move||{
            connection.incoming(5).next();
            glib::Continue(true)
        });

        mpris_player
    }

    pub fn property_changed<T: 'static>(&self, name: String, value: T) where T: dbus::arg::RefArg {
        let mut changed_properties = HashMap::new();
        let x = Box::new(value) as Box<dyn RefArg>;
        changed_properties.insert(name, Variant(x));

        let signal = OrgFreedesktopDBusPropertiesPropertiesChanged {
            changed_properties,
            interface_name: "org.mpris.MediaPlayer2.Player".to_string(),
            invalidated_properties: Vec::new(),
        };

        self.connection.send(signal.to_emit_message(&Path::new("/org/mpris/MediaPlayer2").unwrap())).unwrap();
    }

    pub fn seeked(&self, value: i64) {
        self.position.set(value);
        let signal = dbus::Message::signal(
            &Path::new("/org/mpris/MediaPlayer2").unwrap(),
            &dbus::Interface::new("org.mpris.MediaPlayer2.Player").unwrap(),
            &dbus::Member::new("Seeked").unwrap(),
        )
        .append(value);
        self.connection.send(signal).unwrap();
    }

    //
    // OrgMprisMediaPlayer2 setters...
    //

    pub fn set_supported_mime_types(&self, value: Vec<String>){
        *self.supported_mime_types.borrow_mut() = value;
        self.property_changed("SupportedMimeTypes".to_string(), self.get_supported_mime_types().unwrap());
    }

    pub fn set_supported_uri_schemes(&self, value: Vec<String>){
        *self.supported_uri_schemes.borrow_mut() = value;
        self.property_changed("SupportedUriSchemes".to_string(), self.get_supported_uri_schemes().unwrap());
    }

    pub fn set_can_quit(&self, value: bool){
        self.can_quit.set(value);
        self.property_changed("CanQuit".to_string(), self.get_can_quit().unwrap());
    }

    pub fn set_can_raise(&self, value: bool){
        self.can_raise.set(value);
        self.property_changed("CanRaise".to_string(), self.get_can_raise().unwrap());
    }

    pub fn set_can_set_fullscreen(&self, value: bool){
        self.can_set_fullscreen.set(value);
        self.property_changed("CanSetFullscreen".to_string(), self.get_can_set_fullscreen().unwrap());
    }

    pub fn set_has_track_list(&self, value: bool){
        self.has_track_list.set(value);
        self.property_changed("HasTrackList".to_string(), self.get_has_track_list().unwrap());
    }


    //
    // OrgMprisMediaPlayer2Player setters...
    //

    pub fn set_playback_status(&self, value: PlaybackStatus){
        self.playback_status.set(value);
        self.property_changed("PlaybackStatus".to_string(), self.get_playback_status().unwrap());
    }

    pub fn set_loop_status(&self, value: LoopStatus){
        self.loop_status.set(value);
        self.property_changed("LoopStatus".to_string(), self.get_loop_status().unwrap());
    }

    pub fn set_metadata(&self, metadata: Metadata){
        *self.metadata.borrow_mut() = metadata;
        self.property_changed("Metadata".to_string(), self.get_metadata().unwrap());
    }

    pub fn set_position(&self, value: i64){
        self.position.set(value);
    }

    pub fn set_minimum_rate(&self, value: f64){
        self.minimum_rate.set(value);
        self.property_changed("MinimumRate".to_string(), self.get_minimum_rate().unwrap());
    }

    pub fn set_maximum_rate(&self, value: f64){
        self.maximum_rate.set(value);
        self.property_changed("MaximumRate".to_string(), self.get_maximum_rate().unwrap());
    }

    pub fn set_can_go_next(&self, value: bool){
        self.can_go_next.set(value);
        self.property_changed("CanGoNext".to_string(), self.get_can_go_next().unwrap());
    }

    pub fn set_can_go_previous(&self, value: bool){
        self.can_go_previous.set(value);
        self.property_changed("CanPrevious".to_string(), self.get_can_go_previous().unwrap());
    }

    pub fn set_can_play(&self, value: bool){
        self.can_play.set(value);
        self.property_changed("CanPlay".to_string(), self.get_can_play().unwrap());
    }

    pub fn set_can_pause(&self, value: bool){
        self.can_pause.set(value);
        self.property_changed("CanPause".to_string(), self.get_can_pause().unwrap());
    }

    pub fn set_can_seek(&self, value: bool){
        self.can_seek.set(value);
        self.property_changed("CanSeek".to_string(), self.get_can_seek().unwrap());
    }

    pub fn set_can_control(&self, value: bool){
        self.can_control.set(value);
        self.property_changed("CanControl".to_string(), self.get_can_control().unwrap());
    }


    //
    // Callbacks
    //

    pub fn connect_raise<F: FnMut()+'static>(&self, callback: F) {
        let cell = Rc::new(RefCell::new(callback));
        self.raise_cb.borrow_mut().push(cell);
    }

    pub fn connect_quit<F: FnMut()+'static>(&self, callback: F) {
        let cell = Rc::new(RefCell::new(callback));
        self.quit_cb.borrow_mut().push(cell);
    }

    pub fn connect_next<F: FnMut()+'static>(&self, callback: F) {
        let cell = Rc::new(RefCell::new(callback));
        self.next_cb.borrow_mut().push(cell);
    }

    pub fn connect_previous<F: FnMut()+'static>(&self, callback: F) {
        let cell = Rc::new(RefCell::new(callback));
        self.previous_cb.borrow_mut().push(cell);
    }

    pub fn connect_pause<F: FnMut()+'static>(&self, callback: F) {
        let cell = Rc::new(RefCell::new(callback));
        self.pause_cb.borrow_mut().push(cell);
    }

    pub fn connect_play_pause<F: FnMut()+'static>(&self, callback: F) {
        let cell = Rc::new(RefCell::new(callback));
        self.play_pause_cb.borrow_mut().push(cell);
    }

    pub fn connect_stop<F: FnMut()+'static>(&self, callback: F) {
        let cell = Rc::new(RefCell::new(callback));
        self.stop_cb.borrow_mut().push(cell);
    }

    pub fn connect_play<F: FnMut()+'static>(&self, callback: F) {
        let cell = Rc::new(RefCell::new(callback));
        self.play_cb.borrow_mut().push(cell);
    }

    pub fn connect_seek<F: FnMut(i64)+'static>(&self, callback: F) {
        let cell = Rc::new(RefCell::new(callback));
        self.seek_cb.borrow_mut().push(cell);
    }

    pub fn connect_open_uri<F: FnMut(&str)+'static>(&self, callback: F) {
        let cell = Rc::new(RefCell::new(callback));
        self.open_uri_cb.borrow_mut().push(cell);
    }

    pub fn connect_fullscreen<F: FnMut(bool)+'static>(&self, callback: F) {
        let cell = Rc::new(RefCell::new(callback));
        self.fullscreen_cb.borrow_mut().push(cell);
    }

    pub fn connect_loop_status<F: FnMut(LoopStatus)+'static>(&self, callback: F) {
        let cell = Rc::new(RefCell::new(callback));
        self.loop_status_cb.borrow_mut().push(cell);
    }

    pub fn connect_rate<F: FnMut(f64)+'static>(&self, callback: F) {
        let cell = Rc::new(RefCell::new(callback));
        self.rate_cb.borrow_mut().push(cell);
    }

    pub fn connect_shuffle<F: FnMut(bool)+'static>(&self, callback: F) {
        let cell = Rc::new(RefCell::new(callback));
        self.shuffle_cb.borrow_mut().push(cell);
    }

    pub fn connect_volume<F: FnMut(f64)+'static>(&self, callback: F) {
        let cell = Rc::new(RefCell::new(callback));
        self.volume_cb.borrow_mut().push(cell);
    }
}

impl OrgMprisMediaPlayer2 for MprisPlayer {
    type Err = tree::MethodErr;

    fn raise(&self) -> Result<(), Self::Err> {
        for callback in self.raise_cb.borrow_mut().iter() {
            let mut closure = callback.borrow_mut(); (&mut *closure)();
        }
        Ok(())
    }

    fn quit(&self) -> Result<(), Self::Err> {
        for callback in self.quit_cb.borrow_mut().iter() {
            let mut closure = callback.borrow_mut(); (&mut *closure)();
        }
        Ok(())
    }

    fn get_can_quit(&self) -> Result<bool, Self::Err> {
        Ok(self.can_quit.get())
    }

    fn get_fullscreen(&self) -> Result<bool, Self::Err> {
        Ok(self.fullscreen.get())
    }

    fn set_fullscreen(&self, value: bool) -> Result<(), Self::Err> {
        self.fullscreen.set(value);
        self.property_changed("Fullscreen".to_string(), self.get_fullscreen().unwrap());
        for callback in self.fullscreen_cb.borrow_mut().iter() {
            let mut closure = callback.borrow_mut(); (&mut *closure)(value);
        }
        Ok(())
    }

    fn get_can_set_fullscreen(&self) -> Result<bool, Self::Err> {
        Ok(self.can_set_fullscreen.get())
    }

    fn get_can_raise(&self) -> Result<bool, Self::Err> {
        Ok(self.can_raise.get())
    }

    fn get_has_track_list(&self) -> Result<bool, Self::Err> {
        Ok(self.has_track_list.get())
    }

    fn get_identity(&self) -> Result<String, Self::Err> {
        Ok(self.identify.clone())
    }

    fn get_desktop_entry(&self) -> Result<String, Self::Err> {
        Ok(self.desktop_entry.clone())
    }

    fn get_supported_uri_schemes(&self) -> Result<Vec<String>, Self::Err> {
        Ok(self.supported_uri_schemes.borrow().to_vec())
    }

    fn get_supported_mime_types(&self) -> Result<Vec<String>, Self::Err> {
        Ok(self.supported_mime_types.borrow().to_vec())
    }
}

impl OrgMprisMediaPlayer2Player for MprisPlayer {
    type Err = tree::MethodErr;

    fn next(&self) -> Result<(), Self::Err> {
        for callback in self.next_cb.borrow_mut().iter() {
            let mut closure = callback.borrow_mut(); (&mut *closure)();
        }
        Ok(())
    }

    fn previous(&self) -> Result<(), Self::Err> {
        for callback in self.previous_cb.borrow_mut().iter() {
            let mut closure = callback.borrow_mut(); (&mut *closure)();
        }
        Ok(())
    }

    fn pause(&self) -> Result<(), Self::Err> {
        for callback in self.pause_cb.borrow_mut().iter() {
            let mut closure = callback.borrow_mut(); (&mut *closure)();
        }
        Ok(())
    }

    fn play_pause(&self) -> Result<(), Self::Err> {
        for callback in self.play_pause_cb.borrow_mut().iter() {
            let mut closure = callback.borrow_mut(); (&mut *closure)();
        }
        Ok(())
    }

    fn stop(&self) -> Result<(), Self::Err> {
        for callback in self.stop_cb.borrow_mut().iter() {
            let mut closure = callback.borrow_mut(); (&mut *closure)();
        }
        Ok(())
    }

    fn play(&self) -> Result<(), Self::Err> {
        for callback in self.play_cb.borrow_mut().iter() {
            let mut closure = callback.borrow_mut(); (&mut *closure)();
        }
        Ok(())
    }

    fn seek(&self, offset: i64) -> Result<(), Self::Err> {
        for callback in self.seek_cb.borrow_mut().iter() {
            let mut closure = callback.borrow_mut(); (&mut *closure)(offset);
        }
        Ok(())
    }

    fn set_position(&self, _track_id: dbus::Path, position: i64) -> Result<(), Self::Err> {
        self.position.set(position);
        self.property_changed("Position".to_string(), self.get_position().unwrap());
        Ok(())
    }

    fn open_uri(&self, uri: &str) -> Result<(), Self::Err> {
        for callback in self.open_uri_cb.borrow_mut().iter() {
            let mut closure = callback.borrow_mut(); (&mut *closure)(uri);
        }
        Ok(())
    }

    fn get_playback_status(&self) -> Result<String, Self::Err> {
        Ok(self.playback_status.get().value())
    }

    fn get_loop_status(&self) -> Result<String, Self::Err> {
        Ok(self.loop_status.get().value())
    }

    fn set_loop_status(&self, value: String) -> Result<(), Self::Err> {
        let ls = match value.as_ref() {
            "Track" => LoopStatus::Track,
            "Playlist" => LoopStatus::Playlist,
            _ => LoopStatus::None,
        };
        self.loop_status.set(ls.clone());
        self.property_changed("LoopStatus".to_string(), self.get_loop_status().unwrap());
        for callback in self.loop_status_cb.borrow_mut().iter() {
            let mut closure = callback.borrow_mut(); (&mut *closure)(ls);
        }
        Ok(())
    }

    fn get_rate(&self) -> Result<f64, Self::Err> {
        Ok(self.rate.get())
    }

    fn set_rate(&self, value: f64) -> Result<(), Self::Err> {
        self.rate.set(value);
        self.property_changed("Rate".to_string(), self.get_rate().unwrap());
        for callback in self.rate_cb.borrow_mut().iter() {
            let mut closure = callback.borrow_mut(); (&mut *closure)(value);
        }
        Ok(())
    }

    fn get_shuffle(&self) -> Result<bool, Self::Err> {
        Ok(self.shuffle.get())
    }

    fn set_shuffle(&self, value: bool) -> Result<(), Self::Err> {
        self.shuffle.set(value);
        self.property_changed("Shuffle".to_string(), self.get_volume().unwrap());
        for callback in self.shuffle_cb.borrow_mut().iter() {
            let mut closure = callback.borrow_mut(); (&mut *closure)(value);
        }
        Ok(())
    }

    fn get_metadata(&self) -> Result<HashMap<String, Variant<Box<dyn RefArg + 'static>>>, Self::Err> {
        let metadata = self.metadata.borrow().to_hashmap().unwrap();
        Ok(metadata)
    }

    fn get_volume(&self) -> Result<f64, Self::Err> {
        Ok(self.volume.get())
    }

    fn set_volume(&self, value: f64) -> Result<(), Self::Err> {
        self.volume.set(value);
        self.property_changed("Volume".to_string(), self.get_volume().unwrap());
        for callback in self.volume_cb.borrow_mut().iter() {
            let mut closure = callback.borrow_mut(); (&mut *closure)(value);
        }
        Ok(())
    }

    fn get_position(&self) -> Result<i64, Self::Err> {
        Ok(self.position.get())
    }

    fn get_minimum_rate(&self) -> Result<f64, Self::Err> {
        Ok(self.minimum_rate.get())
    }

    fn get_maximum_rate(&self) -> Result<f64, Self::Err> {
        Ok(self.maximum_rate.get())
    }

    fn get_can_go_next(&self) -> Result<bool, Self::Err> {
        Ok(self.can_go_next.get())
    }

    fn get_can_go_previous(&self) -> Result<bool, Self::Err> {
        Ok(self.can_go_previous.get())
    }

    fn get_can_play(&self) -> Result<bool, Self::Err> {
        Ok(self.can_play.get())
    }

    fn get_can_pause(&self) -> Result<bool, Self::Err> {
        Ok(self.can_pause.get())
    }

    fn get_can_seek(&self) -> Result<bool, Self::Err> {
        Ok(self.can_seek.get())
    }

    fn get_can_control(&self) -> Result<bool, Self::Err> {
        Ok(self.can_control.get())
    }
}

impl ::std::fmt::Debug for MprisPlayer{
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result  {
        write!(f, "mprisplayer")
    }
}

#[derive(Copy, Clone, Default, Debug)]
pub struct TData;
impl tree::DataType for TData {
    type Tree = ();
    type ObjectPath = Arc<MprisPlayer>;
    type Property = ();
    type Interface = ();
    type Method = ();
    type Signal = ();
}
