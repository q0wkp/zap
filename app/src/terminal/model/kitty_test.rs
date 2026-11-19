use super::*;

fn control_data(control: &str) -> KittyControlData {
    parse_kitty_chunk(control.as_bytes().to_vec()).control_data
}

#[test]
fn zero_ids_mean_no_id_was_given() {
    let parsed = control_data("a=p,i=0,p=0");
    assert_eq!(parsed.image_id, None);
    assert_eq!(parsed.placement_id, None);

    let parsed = control_data("a=p,i=7,p=3");
    assert_eq!(parsed.image_id, Some(7));
    assert_eq!(parsed.placement_id, Some(3));
}

#[test]
fn ok_reply_echoes_the_placement_id() {
    assert_eq!(create_kitty_ok_reply(7, None), b"\x1b_Gi=7;OK\x1b\\");
    assert_eq!(create_kitty_ok_reply(7, Some(3)), b"\x1b_Gi=7,p=3;OK\x1b\\");
}

#[test]
fn error_reply_starts_with_a_protocol_error_code() {
    let reply = |err: KittyError| {
        String::from_utf8(create_kitty_error_reply(7, None, err)).expect("reply should be UTF-8")
    };

    assert!(reply(StorageError::UnknownId { id: 7 }.into()).starts_with("\x1b_Gi=7;ENOENT:"));
    assert!(reply(InvalidControlData::IdMissing.into()).starts_with("\x1b_Gi=7;EINVAL:"));
    assert!(reply(InvalidKittyAction::UnsupportedAction.into()).starts_with("\x1b_Gi=7;ENOTSUPP:"));
    assert!(
        reply(KittyDecodeError::InvalidBase64("bad".to_string()).into())
            .starts_with("\x1b_Gi=7;EINVAL:")
    );
    assert!(reply(
        InvalidKittyPayload::FileError(FileError::FileReadError("gone".to_string())).into()
    )
    .starts_with("\x1b_Gi=7;EBADF:"));
    assert!(reply(KittyError::KittyFeatureDisabled).ends_with("\x1b\\"));
}
