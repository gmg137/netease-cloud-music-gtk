pkgdatadir = join_paths(get_option('prefix'), get_option('datadir'), meson.project_name())

conf = configuration_data()
conf.set_quoted('VERSION', meson.project_version())
conf.set_quoted('GETTEXT_PACKAGE', 'netease-cloud-music-gtk4')
conf.set_quoted('LOCALEDIR', join_paths(get_option('prefix'), get_option('localedir')))
conf.set_quoted('PKGDATADIR', pkgdatadir)

configure_file(
    input: 'config.rs.in',
    output: 'config.rs',
    configuration: conf
)

# Copy the config.rs output to the source directory.
run_command(
  'cp',
  join_paths(meson.project_build_root(), 'src', 'config.rs'),
  join_paths(meson.project_source_root(), 'src', 'config.rs'),
  check: true
)

rust_sources = files(
  'application.rs',
  'config.rs',
  'main.rs',
  'window.rs',
  'model.rs',
  'ncmapi.rs',
  'path.rs',
  'utils.rs',
  'audio/mod.rs',
  'audio/mpris.rs',
  'audio/playlist.rs',
  'gui/mod.rs',
  'gui/discover.rs',
  'gui/my_page.rs',
  'gui/player_controls.rs',
  'gui/playlist_lyrics.rs',
  'gui/preferences.rs',
  'gui/search_singer_page.rs',
  'gui/search_song_page.rs',
  'gui/search_songlist_page.rs',
  'gui/songlist_grid_item.rs',
  'gui/songlist_page.rs',
  'gui/songlist_row.rs',
  'gui/songlist_view.rs',
  'gui/theme_selector.rs',
  'gui/toplist.rs',
  'gui/user_menus.rs',
)

sources = [cargo_sources, rust_sources]

cargo_script = find_program(join_paths(meson.project_source_root(), 'build-aux/cargo.sh'))
cargo_release = custom_target(
  'cargo-build',
  build_by_default: true,
  input: sources,
  output: meson.project_name(),
  console: true,
  install: true,
  install_dir: get_option('bindir'),
  command: [
    cargo_script,
    meson.project_build_root(),
    meson.project_source_root(),
    '@OUTPUT@',
    get_option('buildtype'),
    meson.project_name(),
  ]
)
