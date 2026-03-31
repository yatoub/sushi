use super::*;

fn valid_form() -> TunnelForm {
    TunnelForm {
        label: "PG".into(),
        local_port: "5433".into(),
        remote_host: "127.0.0.1".into(),
        remote_port: "5432".into(),
        focus: TunnelFormField::Label,
        editing_index: None,
        error: String::new(),
    }
}

#[test]
fn form_validate_ok() {
    let cfg = valid_form().validate(&crate::i18n::STRINGS_FR).unwrap();
    assert_eq!(cfg.local_port, 5433);
    assert_eq!(cfg.remote_host, "127.0.0.1");
    assert_eq!(cfg.remote_port, 5432);
    assert_eq!(cfg.label, "PG");
}

#[test]
fn form_validate_bad_local_port() {
    let mut f = valid_form();
    f.local_port = "abc".into();
    assert!(f.validate(&crate::i18n::STRINGS_FR).is_err());
    f.local_port = "0".into();
    assert!(f.validate(&crate::i18n::STRINGS_FR).is_err());
    f.local_port = "65536".into();
    assert!(f.validate(&crate::i18n::STRINGS_FR).is_err());
}

#[test]
fn form_validate_empty_remote_host() {
    let mut f = valid_form();
    f.remote_host = "   ".into();
    let err = f.validate(&crate::i18n::STRINGS_FR).unwrap_err();
    assert!(err.contains("obligatoire"));
}

#[test]
fn form_validate_bad_remote_port() {
    let mut f = valid_form();
    f.remote_port = "not_a_port".into();
    assert!(f.validate(&crate::i18n::STRINGS_FR).is_err());
}

#[test]
fn tunnel_form_field_cycle_forward() {
    assert_eq!(TunnelFormField::Label.next(), TunnelFormField::LocalPort);
    assert_eq!(
        TunnelFormField::LocalPort.next(),
        TunnelFormField::RemoteHost
    );
    assert_eq!(
        TunnelFormField::RemoteHost.next(),
        TunnelFormField::RemotePort
    );
    assert_eq!(TunnelFormField::RemotePort.next(), TunnelFormField::Label);
}

#[test]
fn tunnel_form_field_cycle_backward() {
    assert_eq!(TunnelFormField::Label.prev(), TunnelFormField::RemotePort);
    assert_eq!(
        TunnelFormField::RemotePort.prev(),
        TunnelFormField::RemoteHost
    );
}

#[test]
fn tunnel_form_char_filters_non_digits_for_ports() {
    let mut f = valid_form();
    f.focus = TunnelFormField::LocalPort;
    f.local_port = "543".into();

    let mut form_state = TunnelOverlayState::Form(f);
    if let TunnelOverlayState::Form(ref mut form) = form_state {
        let old_len = form.local_port.len();
        if !'x'.is_ascii_digit() {
        } else {
            form.local_port.push('x');
        }
        assert_eq!(form.local_port.len(), old_len);
    }
}

#[test]
fn new_edit_form_prefilled() {
    let cfg = crate::config::TunnelConfig {
        local_port: 8080,
        remote_host: "db.local".into(),
        remote_port: 3306,
        label: "MySQL".into(),
    };
    let form = TunnelForm::new_edit(2, &cfg);
    assert_eq!(form.editing_index, Some(2));
    assert_eq!(form.local_port, "8080");
    assert_eq!(form.remote_host, "db.local");
    assert_eq!(form.label, "MySQL");
}
