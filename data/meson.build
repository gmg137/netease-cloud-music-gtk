pkgdatadir = join_paths(get_option('prefix'), get_option('datadir'), meson.project_name())
gnome = import('gnome')

gnome.compile_resources('netease-cloud-music-gtk4',
  'netease_cloud_music_gtk4.gresource.xml',
  gresource_bundle: true,
  install: true,
  install_dir: pkgdatadir,
)

desktop_file = i18n.merge_file(
  input: 'com.gitee.gmg137.NeteaseCloudMusicGtk4.desktop.in',
  output: 'com.gitee.gmg137.NeteaseCloudMusicGtk4.desktop',
  type: 'desktop',
  po_dir: '../po',
  install: true,
  install_dir: join_paths(get_option('datadir'), 'applications')
)

desktop_utils = find_program('desktop-file-validate', required: false)
if desktop_utils.found()
  test('Validate desktop file', desktop_utils,
    args: [desktop_file]
  )
endif

appstream_file = i18n.merge_file(
  input: 'com.gitee.gmg137.NeteaseCloudMusicGtk4.metainfo.xml.in',
  output: 'com.gitee.gmg137.NeteaseCloudMusicGtk4.metainfo.xml',
  po_dir: '../po',
  install: true,
  install_dir: join_paths(get_option('datadir'), 'metainfo')
)

appstreamcli = find_program('appstreamcli', required: false, disabler: true)
test('Validate appstream file', appstreamcli,
     args: ['validate', '--no-net', '--explain', appstream_file])

install_data('com.gitee.gmg137.NeteaseCloudMusicGtk4.gschema.xml',
  install_dir: join_paths(get_option('datadir'), 'glib-2.0/schemas')
)

compile_schemas = find_program('glib-compile-schemas', required: false)
if compile_schemas.found()
  test('Validate schema file', compile_schemas,
    args: ['--strict', '--dry-run', meson.current_source_dir()]
  )
endif

subdir('icons')
