// SPDX-FileCopyrightText: 2026 Tianyu (Sky) Lu
// SPDX-License-Identifier: AGPL-3.0-only

use super::*;

fn control_data(control: &str) -> KittyControlData {
    parse_kitty_chunk(control.as_bytes().to_vec()).control_data
}

fn reply_ids(image_id: u32) -> KittyReplyIds {
    KittyReplyIds {
        image_id,
        image_number: None,
        placement_id: None,
    }
}

fn delete_action(control: &str) -> Result<KittyAction, KittyError> {
    KittyAction::try_from(KittyMessage {
        control_data: control_data(control),
        payload: vec![],
    })
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
    assert_eq!(create_kitty_ok_reply(reply_ids(7)), b"\x1b_Gi=7;OK\x1b\\");
    assert_eq!(
        create_kitty_ok_reply(KittyReplyIds {
            placement_id: Some(3),
            ..reply_ids(7)
        }),
        b"\x1b_Gi=7,p=3;OK\x1b\\"
    );
}

#[test]
fn ok_reply_echoes_the_image_number() {
    assert_eq!(
        create_kitty_ok_reply(KittyReplyIds {
            image_number: Some(4),
            placement_id: Some(3),
            ..reply_ids(7)
        }),
        b"\x1b_Gi=7,I=4,p=3;OK\x1b\\"
    );
}

#[test]
fn error_reply_starts_with_a_protocol_error_code() {
    let reply = |err: KittyError| {
        String::from_utf8(create_kitty_error_reply(reply_ids(7), err))
            .expect("reply should be UTF-8")
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

#[test]
fn delete_specifiers_select_what_the_protocol_says() {
    let deletion_type = |control: &str| match delete_action(control) {
        Ok(KittyAction::Delete { deletion_type, .. }) => deletion_type,
        other => panic!("expected a delete action for {control:?}, got {other:?}"),
    };

    assert!(matches!(deletion_type("a=d"), DeletionType::DeleteAll));
    assert!(matches!(deletion_type("a=d,d=a"), DeletionType::DeleteAll));
    assert!(matches!(
        deletion_type("a=d,d=i,i=5,p=2"),
        DeletionType::DeleteById(DeleteById {
            image_id: 5,
            placement_id: Some(2)
        })
    ));
    assert!(matches!(deletion_type("a=d,d=c"), DeletionType::AtCursor));
    assert!(matches!(
        deletion_type("a=d,d=p,x=3,y=4"),
        DeletionType::AtCell {
            column: 3,
            row: 4,
            z_index: None
        }
    ));
    assert!(matches!(
        deletion_type("a=d,d=q,x=3,y=4,z=-1"),
        DeletionType::AtCell {
            column: 3,
            row: 4,
            z_index: Some(-1)
        }
    ));
    assert!(matches!(
        deletion_type("a=d,d=x,x=3"),
        DeletionType::InColumn(3)
    ));
    assert!(matches!(
        deletion_type("a=d,d=y,y=4"),
        DeletionType::InRow(4)
    ));
    assert!(matches!(
        deletion_type("a=d,d=z,z=-1"),
        DeletionType::WithZIndex(-1)
    ));
    assert!(matches!(
        deletion_type("a=d,d=r,x=2,y=6"),
        DeletionType::InIdRange { first: 2, last: 6 }
    ));
}

#[test]
fn uppercase_delete_specifier_also_frees_the_image() {
    let placements_only = |control: &str| match delete_action(control) {
        Ok(KittyAction::Delete {
            delete_placements_only,
            ..
        }) => delete_placements_only,
        other => panic!("expected a delete action for {control:?}, got {other:?}"),
    };

    assert!(placements_only("a=d"));
    assert!(placements_only("a=d,d=c"));
    assert!(!placements_only("a=d,d=C"));
    assert!(!placements_only("a=d,d=Z,z=1"));
}

/// A specifier this terminal doesn't know used to fall back to the default, `d=a`, and so
/// deleted every image.
#[test]
fn unknown_delete_specifier_is_an_error() {
    let err = delete_action("a=d,d=w").expect_err("d=w is no delete specifier");
    assert_eq!(err.code(), "EINVAL");

    // Animation frames are not stored.
    let err = delete_action("a=d,d=f,i=1").expect_err("frames can't be deleted");
    assert_eq!(err.code(), "ENOTSUPP");
}

#[test]
fn unicode_placeholder_placement_is_accepted() {
    let action = KittyAction::try_from(KittyMessage {
        control_data: control_data("a=p,U=1,i=5,c=4,r=2"),
        payload: vec![],
    });

    let Ok(KittyAction::DisplayStoredImage(action)) = action else {
        panic!("expected a display action, got {action:?}");
    };
    assert!(action.placement_data.unicode_placeholder);
    assert_eq!(action.placement_data.cols, Some(4));
    assert_eq!(action.placement_data.rows, Some(2));
}
