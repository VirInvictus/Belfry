gnome = import('gnome')

# GSettings schema
install_data(
    'io.github.virinvictus.Belfry.gschema.xml',
    install_dir: get_option('datadir') / 'glib-2.0' / 'schemas',
)
gnome.compile_schemas(
    depend_files: ['io.github.virinvictus.Belfry.gschema.xml'],
)

# AppStream metainfo
install_data(
    'io.github.virinvictus.Belfry.metainfo.xml',
    install_dir: get_option('datadir') / 'metainfo',
)

# Desktop file
install_data(
    'io.github.virinvictus.Belfry.desktop',
    install_dir: get_option('datadir') / 'applications',
)
