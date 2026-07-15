# RPM spec for lmtt (Linux Multi-Theme Toggle). Built in COPR from a local
# SRPM produced by packaging/build-srpm.sh (source tarball from the git tag +
# vendored cargo deps as Source1 — no rust-*-devel packages needed, and the
# schema-tui git dependency is vendored too, so the build is fully offline).
# The test suite runs by default (cargo test over all workspace members).
# Disable for a one-off build with --without check; COPR builds run the suite.
%bcond_without check

Name:           lmtt
Version:        0.1.1
Release:        1%{?dist}
Summary:        Fast async theme switching for Hyprland/Wayland desktops
License:        MIT
URL:            https://github.com/MasonRhodesDev/linux-multi-theme-toggle
# The GitHub repo name differs from the package name; GitHub serves the
# archive under any basename, but its top-level dir would be
# linux-multi-theme-toggle-%%{version}. The SRPM never uses this URL:
# build-srpm.sh always regenerates Source0 with git archive using an
# lmtt-%%{version}/ prefix, which is what %%autosetup expects.
Source0:        %{url}/archive/v%{version}/%{name}-%{version}.tar.gz
Source1:        %{name}-%{version}-vendor.tar.xz

BuildRequires:  cargo-rpm-macros >= 24
# matugen (wallpaper-based Material You palettes) is optional at runtime and
# not packaged in Fedora/COPR default repos; lmtt falls back to built-in
# themes or custom JSON colors without it.

%description
Linux Multi-Theme Toggle: high-performance async light/dark theme switching
for Hyprland/Wayland desktops with Material You color schemes. Applies
themes in parallel across 14+ applications (Waybar, Hyprland, GTK, VSCode,
Wezterm, SwayNC, ...), auto-detects what is installed, and supports
user-defined custom modules. Ships the lmtt CLI and the lmtt-config TUI.
Configuration lives in ~/.config/lmtt and is not part of this package.

%prep
# -a1 unpacks the vendor tarball (vendor/ at its root) into the source dir.
%autosetup -p1 -a1
%cargo_prep -v vendor
# %%cargo_prep only redirects crates-io at the vendor/ directory. schema-tui
# is a git dependency, also vendored by build-srpm.sh; redirect its git
# source at the same vendored copy (this is the stanza cargo vendor printed
# when Source1 was created) so the build never touches the network.
cat >> .cargo/config.toml << 'EOF'

[source."git+https://github.com/MasonRhodesDev/schema-tui.git"]
git = "https://github.com/MasonRhodesDev/schema-tui.git"
replace-with = "vendored-sources"
EOF

%build
%cargo_build
%{cargo_license_summary}
%{cargo_license} > LICENSE.dependencies

%install
# The workspace root is a virtual manifest, so %%cargo_install (cargo
# install --path .) cannot be used; install the two workspace binaries from
# the rpm profile target dir that %%cargo_build populates.
install -Dpm0755 target/rpm/lmtt %{buildroot}%{_bindir}/lmtt
install -Dpm0755 target/rpm/lmtt-config %{buildroot}%{_bindir}/lmtt-config
install -Dpm0644 config-example.toml %{buildroot}%{_datadir}/lmtt/config-example.toml
install -Dpm0644 examples/README-modules.md examples/colors-dark.json \
    examples/colors-light.json -t %{buildroot}%{_datadir}/lmtt/examples
install -Dpm0644 examples/modules/*.toml -t %{buildroot}%{_datadir}/lmtt/examples/modules
install -Dpm0755 examples/scripts/*.sh -t %{buildroot}%{_datadir}/lmtt/examples/scripts

%if %{with check}
%check
%cargo_test
%endif

%files
%license LICENSE LICENSE.dependencies
%doc README.md
%{_bindir}/lmtt
%{_bindir}/lmtt-config
%{_datadir}/lmtt/

%changelog
* Wed Jul 15 2026 Mason Rhodes <mrhodesdev@gmail.com> - 0.1.1-1
- hyprland: emit tertiary + tertiary_container in lmtt-colors.{conf,lua}
  (used for the pinned-window border color)

* Thu Jul 02 2026 Mason Rhodes <mrhodesdev@gmail.com> - 0.1.0-1
- Initial packaged release: lmtt CLI + lmtt-config TUI, config example and
  custom-module examples under /usr/share/lmtt
