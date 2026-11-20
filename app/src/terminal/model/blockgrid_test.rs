use std::collections::{HashMap, HashSet};

use pathfinder_geometry::vector::Vector2F;
use zap_ng_core::features::FeatureFlag;

use crate::terminal::event_listener::ChannelEventListener;
use crate::terminal::model::ansi::{self, Handler};
use crate::terminal::model::blockgrid::{BlockGrid, CursorDisplayPoint};
use crate::terminal::model::grid::grid_handler::PerformResetGridChecks;
use crate::terminal::model::grid::Dimensions;
use crate::terminal::model::image_map::StoredImageMetadata;
use crate::terminal::model::index::{Point, VisibleRow};
use crate::terminal::model::kitty::{
    CursorMovementPolicy, DeleteById, DeletionType, DisplayStoredImage, KittyAction, KittyImage,
    KittyImageMetadata, KittyPlacementData, StoreAndDisplay,
};
use crate::terminal::model::secrets::ObfuscateSecrets;
use crate::terminal::SizeInfo;
use crate::test_util::mock_blockgrid;

#[test]
pub fn test_finish_truncates_grid_basic() {
    let size = SizeInfo::new_without_font_metrics(10, 7);
    let mut block_grid = BlockGrid::new(
        size,
        1000, /* max_scroll_limit */
        ChannelEventListener::new_for_test(),
        ObfuscateSecrets::No,
        PerformResetGridChecks::default(),
    );

    for c in "hello".chars() {
        block_grid.input(c);
    }
    block_grid.linefeed();
    block_grid.finish();

    assert_eq!(block_grid.len(), 2);
    assert_eq!(block_grid.grid_handler().total_rows(), 2);
    assert_eq!(block_grid.grid_handler().columns(), size.columns);
}

#[test]
pub fn test_finish_truncates_grid_cursor_at_bottom() {
    let size = SizeInfo::new_without_font_metrics(10, 7);
    let mut block_grid = BlockGrid::new(
        size,
        1000, /* max_scroll_limit */
        ChannelEventListener::new_for_test(),
        ObfuscateSecrets::No,
        PerformResetGridChecks::default(),
    );

    for _ in 0..300 {
        block_grid.input('a');
        block_grid.linefeed();
        block_grid.carriage_return();
    }

    block_grid.finish();

    assert_eq!(block_grid.len(), 300);
    assert_eq!(block_grid.grid_handler().total_rows(), 301);
    assert_eq!(block_grid.grid_handler().columns(), size.columns);
}

#[test]
pub fn test_resize_finished_block() {
    let size = SizeInfo::new_without_font_metrics(10, 7);
    let mut block_grid = BlockGrid::new(
        size,
        1000, /* max_scroll_limit */
        ChannelEventListener::new_for_test(),
        ObfuscateSecrets::No,
        PerformResetGridChecks::default(),
    );

    for _ in 0..5 {
        for c in "hello".chars() {
            block_grid.input(c);
        }

        block_grid.linefeed();
        block_grid.carriage_return();
    }

    block_grid.finish();

    assert_eq!(block_grid.len(), 5);
    assert_eq!(block_grid.grid_handler().total_rows(), 6);

    block_grid.resize(SizeInfo::new_without_font_metrics(10, 10));

    // The len and total rows of the grid should be unchanged even after resize.
    assert_eq!(block_grid.len(), 5);
    assert_eq!(block_grid.grid_handler().total_rows(), 6);

    // Resize so that the contents of can no longer fit on one line. The length of the grid
    // should increase because each row is now soft-wrapped.
    block_grid.resize(SizeInfo::new_without_font_metrics(10, 3));
    assert_eq!(block_grid.len(), 10);
    // Each of the 5 "hello" rows should be split into "hel" and "lo" (for a total
    // of 10 rows), plus one final empty row at the end.
    assert_eq!(block_grid.grid_handler().total_rows(), 11);

    // Resizing the height in either direction should have no effect on the total grid height.
    block_grid.resize(SizeInfo::new_without_font_metrics(3, 7));
    assert_eq!(block_grid.len(), 5);
    assert_eq!(block_grid.grid_handler().total_rows(), 6);

    block_grid.resize(SizeInfo::new_without_font_metrics(300, 7));
    assert_eq!(block_grid.len(), 5);
    assert_eq!(block_grid.grid_handler().total_rows(), 6);
}

#[test]
pub fn test_resize_finished_softwrapped_block() {
    let size = SizeInfo::new_without_font_metrics(10, 3);
    let mut block_grid = BlockGrid::new(
        size,
        1000, /* max_scroll_limit */
        ChannelEventListener::new_for_test(),
        ObfuscateSecrets::No,
        PerformResetGridChecks::default(),
    );

    for _ in 0..5 {
        for c in "hello".chars() {
            block_grid.input(c);
        }

        block_grid.linefeed();
        block_grid.carriage_return();
    }

    block_grid.finish();

    assert_eq!(block_grid.len(), 10);
    assert_eq!(block_grid.grid_handler().total_rows(), 11);

    // Resize so each item can only fit on a given line.
    block_grid.resize(SizeInfo::new_without_font_metrics(10, 10));

    assert_eq!(block_grid.len(), 5);
    assert_eq!(block_grid.grid_handler().total_rows(), 6);
}

#[test]
pub fn test_content_summary() {
    let blockgrid = mock_blockgrid("1\r\n2\r\n3\r\n4\r\n5\r\n6\r\n7\r\n8\r\n9\r\n10\r\n");

    // Test a summary that omits the middle of the grid.
    let summary = blockgrid.content_summary(1, 2, false);
    assert_eq!(summary, "1\n...(truncated)...\n9\n10\n");

    // Test a summary that perfectly covers the entire grid.
    let summary = blockgrid.content_summary(7, 3, false);
    assert_eq!(summary, "1\n2\n3\n4\n5\n6\n7\n8\n9\n10\n");

    // Test a summary where the total range exceeds the grid size.
    let summary = blockgrid.content_summary(7, 6, false);
    assert_eq!(summary, "1\n2\n3\n4\n5\n6\n7\n8\n9\n10\n");
}

#[test]
pub fn test_trim_trailing_blank_rows_uses_active_floor_for_blank_started_grid() {
    let size = SizeInfo::new_without_font_metrics(10, 10);
    let mut block_grid = BlockGrid::new(
        size,
        1000,
        ChannelEventListener::new_for_test(),
        ObfuscateSecrets::No,
        PerformResetGridChecks::default(),
    );

    block_grid.start();
    block_grid.goto(VisibleRow(4), 0);
    block_grid.on_finish_byte_processing(&ansi::ProcessorInput::new(&[]));
    block_grid.set_trim_trailing_blank_rows(true);

    assert_eq!(block_grid.len(), 5);
    assert_eq!(block_grid.len_displayed(), 1);
    assert_eq!(
        block_grid.cursor_display_point(),
        Some(CursorDisplayPoint::HiddenCache(Point::new(0, 0)))
    );
}

fn kitty_test_blockgrid() -> BlockGrid {
    BlockGrid::new(
        SizeInfo::new_without_font_metrics(10, 10),
        1000,
        ChannelEventListener::new_for_test(),
        ObfuscateSecrets::No,
        PerformResetGridChecks::default(),
    )
}

fn kitty_metadata_with_size(
    image_id: u32,
    width: f32,
    height: f32,
) -> HashMap<u32, StoredImageMetadata> {
    HashMap::from([(
        image_id,
        StoredImageMetadata::Kitty(KittyImageMetadata {
            image_size: Vector2F::new(width, height),
            ..Default::default()
        }),
    )])
}

/// An image a thousand times wider than it is tall, shown one column wide, is less than a pixel
/// high. It must be skipped instead of scrolling the grid by `0 - 1` rows.
#[test]
pub fn test_kitty_image_scaled_to_nothing_is_not_placed() {
    let _kitty_images = FeatureFlag::KittyImages.override_enabled(true);
    let mut block_grid = kitty_test_blockgrid();
    let mut metadata = kitty_metadata_with_size(1, 1000.0, 1.0);
    let action = KittyAction::StoreAndDisplay(StoreAndDisplay {
        image: KittyImage {
            metadata: KittyImageMetadata {
                image_size: Vector2F::new(1000.0, 1.0),
                ..Default::default()
            },
            data: vec![0],
        },
        placement_data: KittyPlacementData {
            cols: Some(1),
            ..Default::default()
        },
        image_id: 1,
        placement_id: 1,
    });

    block_grid
        .handle_completed_kitty_action(action, &mut metadata)
        .expect("kitty action should be handled")
        .expect("kitty action should be ignored");

    assert_eq!(block_grid.grid_handler().cursor_point(), Point::new(0, 0));
    assert!(block_grid
        .grid_handler()
        .get_image_placement_data(1, 1)
        .is_none());
}

/// An image stored without pixels has no aspect ratio to scale by, so placing it later must be
/// ignored like placing it while it is stored.
#[test]
pub fn test_kitty_display_of_stored_image_without_pixels_is_ignored() {
    let _kitty_images = FeatureFlag::KittyImages.override_enabled(true);
    let mut block_grid = kitty_test_blockgrid();
    let mut metadata = kitty_metadata_with_size(1, 0.0, 0.0);
    let action = KittyAction::DisplayStoredImage(DisplayStoredImage {
        placement_data: KittyPlacementData {
            cols: Some(1),
            ..Default::default()
        },
        image_id: 1,
        placement_id: 1,
    });

    block_grid
        .handle_completed_kitty_action(action, &mut metadata)
        .expect("kitty action should be handled")
        .expect("kitty action should be ignored");

    assert_eq!(block_grid.grid_handler().cursor_point(), Point::new(0, 0));
    assert!(block_grid
        .grid_handler()
        .get_image_placement_data(1, 1)
        .is_none());
}

/// A virtual placement is shown through placeholder cells, so it takes no room in the grid.
#[test]
pub fn test_kitty_virtual_placement_is_not_placed_in_the_grid() {
    let _kitty_images = FeatureFlag::KittyImages.override_enabled(true);
    let mut block_grid = kitty_test_blockgrid();
    let mut metadata = kitty_metadata_with_size(1, 10.0, 10.0);
    let action = KittyAction::StoreAndDisplay(StoreAndDisplay {
        image: KittyImage {
            metadata: KittyImageMetadata {
                image_size: Vector2F::new(10.0, 10.0),
                ..Default::default()
            },
            data: vec![0],
        },
        placement_data: KittyPlacementData {
            cols: Some(1),
            rows: Some(1),
            unicode_placeholder: true,
            ..Default::default()
        },
        image_id: 1,
        placement_id: 1,
    });

    block_grid
        .handle_completed_kitty_action(action, &mut metadata)
        .expect("kitty action should be handled")
        .expect("kitty image should be stored");

    assert_eq!(block_grid.grid_handler().cursor_point(), Point::new(0, 0));
    assert!(block_grid
        .grid_handler()
        .get_image_placement_data(1, 1)
        .is_none());
}

/// Places a 2x2-cell Kitty image at the cursor, leaving the cursor where it is.
fn place_kitty_image(
    block_grid: &mut BlockGrid,
    metadata: &mut HashMap<u32, StoredImageMetadata>,
    image_id: u32,
    z_index: i32,
) {
    metadata.extend(kitty_metadata_with_size(image_id, 10.0, 10.0));
    let action = KittyAction::DisplayStoredImage(DisplayStoredImage {
        placement_data: KittyPlacementData {
            z_index,
            cols: Some(2),
            rows: Some(2),
            cursor_movement_policy: CursorMovementPolicy::DoNotMoveCursor,
            unicode_placeholder: false,
        },
        image_id,
        placement_id: 1,
    });

    block_grid
        .handle_completed_kitty_action(action, metadata)
        .expect("kitty action should be handled")
        .expect("kitty image should be placed");
}

/// A grid with image 1 over columns 1-2 and image 2, behind the text, over columns 5-6, both on
/// rows 1-2. The cursor ends on the first cell of image 2.
fn blockgrid_with_two_kitty_images() -> BlockGrid {
    let mut block_grid = kitty_test_blockgrid();
    let mut metadata = HashMap::new();
    place_kitty_image(&mut block_grid, &mut metadata, 1, 0);
    block_grid.goto(VisibleRow(0), 4);
    place_kitty_image(&mut block_grid, &mut metadata, 2, -1);
    block_grid
}

#[test]
pub fn test_kitty_delete_by_position() {
    let _kitty_images = FeatureFlag::KittyImages.override_enabled(true);
    let delete = |deletion_type: DeletionType| {
        let mut block_grid = blockgrid_with_two_kitty_images();
        block_grid
            .grid_handler_mut()
            .delete_kitty_placements(&deletion_type)
    };

    assert_eq!(delete(DeletionType::AtCursor), HashSet::from([2]));
    assert_eq!(
        delete(DeletionType::AtCell {
            column: 2,
            row: 2,
            z_index: None
        }),
        HashSet::from([1])
    );
    assert_eq!(
        delete(DeletionType::AtCell {
            column: 3,
            row: 1,
            z_index: None
        }),
        HashSet::new()
    );
    assert_eq!(
        delete(DeletionType::AtCell {
            column: 5,
            row: 1,
            z_index: Some(0)
        }),
        HashSet::new()
    );
    assert_eq!(
        delete(DeletionType::AtCell {
            column: 5,
            row: 1,
            z_index: Some(-1)
        }),
        HashSet::from([2])
    );
    assert_eq!(delete(DeletionType::InColumn(6)), HashSet::from([2]));
    assert_eq!(delete(DeletionType::InColumn(0)), HashSet::new());
    assert_eq!(delete(DeletionType::InRow(2)), HashSet::from([1, 2]));
    assert_eq!(delete(DeletionType::InRow(3)), HashSet::new());
}

#[test]
pub fn test_kitty_delete_by_id_and_z_index() {
    let _kitty_images = FeatureFlag::KittyImages.override_enabled(true);
    let delete = |deletion_type: DeletionType| {
        let mut block_grid = blockgrid_with_two_kitty_images();
        block_grid
            .grid_handler_mut()
            .delete_kitty_placements(&deletion_type)
    };

    assert_eq!(delete(DeletionType::DeleteAll), HashSet::from([1, 2]));
    assert_eq!(delete(DeletionType::WithZIndex(-1)), HashSet::from([2]));
    assert_eq!(
        delete(DeletionType::InIdRange { first: 2, last: 9 }),
        HashSet::from([2])
    );
    assert_eq!(
        delete(DeletionType::DeleteById(DeleteById {
            image_id: 1,
            placement_id: Some(7)
        })),
        HashSet::new()
    );
    assert_eq!(
        delete(DeletionType::DeleteById(DeleteById {
            image_id: 1,
            placement_id: None
        })),
        HashSet::from([1])
    );
}

#[test]
pub fn test_cursor_display_point_hidden_when_cursor_below_trimmed_content() {
    let size = SizeInfo::new_without_font_metrics(10, 10);
    let mut block_grid = BlockGrid::new(
        size,
        1000,
        ChannelEventListener::new_for_test(),
        ObfuscateSecrets::No,
        PerformResetGridChecks::default(),
    );

    block_grid.start();
    block_grid.input('a');
    block_grid.input('b');
    block_grid.input('c');
    block_grid.on_finish_byte_processing(&ansi::ProcessorInput::new(&[]));
    block_grid.set_trim_trailing_blank_rows(true);

    assert_eq!(block_grid.len_displayed(), 1);
    assert_eq!(
        block_grid.cursor_display_point(),
        Some(CursorDisplayPoint::Visible(Point::new(0, 3)))
    );

    block_grid.goto(VisibleRow(4), 2);
    block_grid.on_finish_byte_processing(&ansi::ProcessorInput::new(&[]));

    assert_eq!(block_grid.len(), 5);
    assert_eq!(block_grid.len_displayed(), 1);
    assert_eq!(
        block_grid.cursor_display_point(),
        Some(CursorDisplayPoint::HiddenCache(Point::new(0, 2)))
    );
}

#[test]
pub fn test_cursor_display_point_not_clipped_when_trimming_disabled() {
    let size = SizeInfo::new_without_font_metrics(10, 10);
    let mut block_grid = BlockGrid::new(
        size,
        1000,
        ChannelEventListener::new_for_test(),
        ObfuscateSecrets::No,
        PerformResetGridChecks::default(),
    );

    block_grid.start();
    block_grid.input('a');
    block_grid.input('b');
    block_grid.input('c');
    block_grid
        .grid_handler_mut()
        .set_marked_text("12345678", &(0..0));

    assert_eq!(block_grid.len_displayed(), 1);
    assert_eq!(
        block_grid.cursor_display_point(),
        Some(CursorDisplayPoint::Visible(Point::new(1, 1)))
    );

    block_grid.set_trim_trailing_blank_rows(true);

    assert_eq!(block_grid.len_displayed(), 1);
    assert_eq!(
        block_grid.cursor_display_point(),
        Some(CursorDisplayPoint::HiddenCache(Point::new(0, 1)))
    );
}
