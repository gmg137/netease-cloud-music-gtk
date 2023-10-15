# mpris-player
#### A Rust library for creating MPRIS2 media players over D-Bus

- [Open homepage](https://gitlab.gnome.org/World/Rust/mpris-player)
- [Report issue](https://gitlab.gnome.org/World/Rust/mpris-player/issues/new)
- [Open documentation](https://docs.rs/mpris-player)

## What is MPRIS?
The Media Player Remote Interfacing Specification is a standard D-Bus interface which aims to provide a common programmatic API for controlling media players.

It provides a mechanism for discovery, querying and basic playback control of compliant media players, as well as a tracklist interface which is used to add context to the active media item.

## Which MRPIS interfaces does this crate implement?
- [x] org.mpris.MediaPlayer2
- [x] org.mpris.MediaPlayer2.Player
- [ ] org.mpris.MediaPlayer2.TrackList
- [ ] org.mpris.MediaPlayer2.Playlists

