%define __spec_install_post %{nil}
%define __os_install_post %{_dbpath}/brp-compress
%define debug_package %{nil}

Name: netease-cloud-music-gtk
Summary: Linux 平台下基于 Rust + GTK 开发的网易云音乐播放器
Version: @@VERSION@@
Release: 1
License: GPL v3
Group: Productivity/Multimedia/Sound/Players
URL: https://github.com/gmg137/netease-cloud-music-gtk
Source0: %{name}-%{version}.tar.gz

BuildRoot: %{_tmppath}/%{name}-%{version}-%{release}-root

BuildRequires: (gstreamer-devel or gstreamer1-devel)
BuildRequires: (gstreamer-plugins-bad-devel or gstreamer1-plugins-bad-free-devel)
BuildRequires: (gstreamer-plugins-base-devel or gstreamer1-plugins-base-devel)
Requires: openssl
Requires: (gstreamer or gstreamer1)
Requires: (gstreamer-plugins-bad or gstreamer1-plugins-bad-free)
Requires: (gstreamer-plugins-ugly or gstreamer1-plugins-ugly)
Requires: (gstreamer-plugins-base or gstreamer1-plugins-base)
Requires: (gstreamer-plugins-good or gstreamer1-plugins-good)
Requires: (gstreamer-plugins-libav or gstreamer1-libav)

%description
%{summary}

%prep
%setup -q

%install
rm -rf %{buildroot}
mkdir -p %{buildroot}
cp -a * %{buildroot}
mkdir -p %{buildroot}/usr/share/applications
mkdir -p %{buildroot}/usr/share/pixmaps
cp -a ../../../../../icons/* %{buildroot}/usr/share/pixmaps/
cp -a ../../../../../*.desktop %{buildroot}/usr/share/applications/
strip %{buildroot}/usr/bin/%{name}

%clean
rm -rf %{buildroot}

%files
%defattr(-,root,root,-)
%{_bindir}/*
%{_datadir}/applications/%{name}.desktop
%{_datadir}/pixmaps/%{name}.svg
