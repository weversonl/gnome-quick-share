Name:           gnome-quick-share
Version:        %{?version_override}%{!?version_override:1.3.0}
Release:        1%{?dist}
Summary:        GNOME Quick Share client
License:        AGPL-3.0-only
URL:            https://github.com/weversonl/gnome-quick-share
Source0:        %{name}-%{version}.tar.gz

BuildRequires:  cargo
BuildRequires:  rust
BuildRequires:  meson
BuildRequires:  ninja-build
BuildRequires:  gettext
BuildRequires:  glib2-devel
BuildRequires:  gtk4-devel
BuildRequires:  libadwaita-devel
BuildRequires:  gtk3-devel
BuildRequires:  libayatana-appindicator-devel
BuildRequires:  dbus-devel

Requires:       gtk4
Requires:       libadwaita
Requires:       gtk3
Requires:       libayatana-appindicator
Requires:       dbus

%description
GnomeQS is a GTK4 and Libadwaita desktop client for nearby file sharing.

%prep
%autosetup

%build
meson setup \
    --prefix=%{_prefix} \
    --bindir=%{_bindir} \
    --datadir=%{_datadir} \
    --buildtype=release \
    _build

ninja %{?_smp_mflags} -C _build

%install
DESTDIR=%{buildroot} meson install -C _build

%post
/usr/bin/glib-compile-schemas %{_datadir}/glib-2.0/schemas >/dev/null 2>&1 || :
/usr/bin/gtk-update-icon-cache -q %{_datadir}/icons/hicolor >/dev/null 2>&1 || :

%postun
/usr/bin/glib-compile-schemas %{_datadir}/glib-2.0/schemas >/dev/null 2>&1 || :
/usr/bin/gtk-update-icon-cache -q %{_datadir}/icons/hicolor >/dev/null 2>&1 || :

%files
%license
%{_bindir}/gnomeqs
%{_bindir}/gnomeqs-tray
%{_datadir}/applications/io.github.weversonl.GnomeQuickShare.desktop
%{_datadir}/metainfo/io.github.weversonl.GnomeQuickShare.metainfo.xml
%{_datadir}/glib-2.0/schemas/io.github.weversonl.GnomeQuickShare.gschema.xml
%{_datadir}/icons/hicolor/32x32/apps/io.github.weversonl.GnomeQuickShare.png
%{_datadir}/icons/hicolor/128x128/apps/io.github.weversonl.GnomeQuickShare.png
%{_datadir}/icons/hicolor/256x256@2/apps/io.github.weversonl.GnomeQuickShare.png
%{_datadir}/icons/hicolor/32x32/apps/io.github.weversonl.GnomeQuickShare-symbolic.png
%{_datadir}/icons/hicolor/scalable/actions/io.github.weversonl.GnomeQuickShare-airdrop-symbolic.svg
%{_datadir}/icons/hicolor/scalable/status/io.github.weversonl.GnomeQuickShare-tray-symbolic.svg
%{_datadir}/locale/pt_BR/LC_MESSAGES/gnomeqs.mo
