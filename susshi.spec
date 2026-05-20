Name:           susshi
Version:        0.14.0
Release:        1%{?dist}
Summary:        Modern terminal-based SSH connection manager
License:        MIT
URL:            https://github.com/yatoub/susshi
Source0:        https://github.com/yatoub/susshi/archive/refs/tags/v%{version}.tar.gz

BuildRequires:  cargo
BuildRequires:  openssl-devel
BuildRequires:  zlib-devel
Requires:       openssh-clients

%description
susshi is a modern TUI SSH connection manager with Catppuccin theme,
supporting direct, jump, and Wallix bastion connections.

%prep
%autosetup -n %{name}-%{version}
export RUSTUP_TOOLCHAIN=stable
export LIBZ_SYS_USE_PKG_CONFIG=1
cargo fetch --locked

%build
export RUSTUP_TOOLCHAIN=stable
export LIBZ_SYS_USE_PKG_CONFIG=1
cargo build --frozen --release

%check
export RUSTUP_TOOLCHAIN=stable
export LIBZ_SYS_USE_PKG_CONFIG=1
cargo test --frozen

%install
install -Dm0755 target/release/%{name} %{buildroot}%{_bindir}/%{name}
install -Dm0644 target/man/%{name}.1 %{buildroot}%{_mandir}/man1/%{name}.1
install -Dm0644 README.md %{buildroot}%{_docdir}/%{name}/README.md
cp -r docs/ %{buildroot}%{_docdir}/%{name}/docs/
cp -r examples/ %{buildroot}%{_docdir}/%{name}/examples/

%files
%license LICENCE
%{_bindir}/%{name}
%{_mandir}/man1/%{name}.1*
%doc %{_docdir}/%{name}/README.md
%doc %{_docdir}/%{name}/docs/
%doc %{_docdir}/%{name}/examples/

%changelog
* Wed Mar 17 2026 yatoub <yatoub@users.noreply.github.com> - 0.14.0-1
- Initial RPM packaging for susshi
