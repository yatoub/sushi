Name:           susshi
Version:        0.14.0
Release:        1%{?dist}
Summary:        Modern terminal-based SSH connection manager
License:        MIT
URL:            https://github.com/yatoub/susshi
Source0:        https://github.com/yatoub/susshi/archive/refs/tags/v%{version}.tar.gz

BuildRequires:  cargo
BuildRequires:  openssl-devel
Requires:       openssh-clients

%description
susshi is a modern TUI SSH connection manager with Catppuccin theme,
supporting direct, jump, and Wallix bastion connections.

%prep
%autosetup -n %{name}-%{version}
export RUSTUP_TOOLCHAIN=stable
cargo fetch --locked

%build
export RUSTUP_TOOLCHAIN=stable
cargo build --frozen --release

%check
export RUSTUP_TOOLCHAIN=stable
cargo test --frozen

%install
install -Dm0755 target/release/%{name} %{buildroot}%{_bindir}/%{name}

%files
%license LICENCE
%{_bindir}/%{name}

%changelog
* Wed Mar 17 2026 yatoub <yatoub@users.noreply.github.com> - 0.14.0-1
- Initial RPM packaging for susshi
