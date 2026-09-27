# Virtual song list regression test

The GTK regression test is ignored by default because it opens a window and
requires a display, compiled resources, and the application GSettings schema.
It uses synthetic songs and an in-memory settings backend; it does not log in
or send requests to NetEase.

From the repository root, after installing the build dependencies in README.md:

```sh
meson setup build --buildtype=release
meson compile -C build
glib-compile-schemas --targetdir=build data
GSETTINGS_SCHEMA_DIR="$PWD/build" \
GSETTINGS_BACKEND=memory \
NCM_TEST_RESOURCE="$PWD/build/data/netease-cloud-music-gtk4.gresource" \
CARGO_TARGET_DIR="$PWD/build/target" \
cargo test --release virtual_list_keeps_all_songs_with_bounded_widgets -- \
  --ignored --nocapture --test-threads=1
```

The window closes automatically. The test checks that 10,000 songs remain in
order while fewer than 300 rows are bound, including after scrolling to the
end. It also checks activation, playing indicators, delayed like callbacks
across row recycling, clearing, and appending songs.
