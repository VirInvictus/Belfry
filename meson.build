project(
    'belfry',
    version: '0.0.0',
    license: 'GPL-3.0-or-later',
    meson_version: '>= 1.4.0',
)

# Belfry is built with Cargo. Meson is the wrapper for Flatpak packaging
# and distribution layout — handles GSettings schema compilation,
# AppStream metainfo, .desktop file, and (later) icon install. Direct
# development uses cargo. Real Cargo integration ships with the
# v0.1 / v0.5 phases (see roadmap.md); this scaffold establishes the
# directory layout and dependency checks.

dependency('gtk4', version: '>= 4.16')
dependency('libadwaita-1', version: '>= 1.7')
dependency('mpv', version: '>= 0.36')
dependency('sqlite3', version: '>= 3.38')

subdir('data')
