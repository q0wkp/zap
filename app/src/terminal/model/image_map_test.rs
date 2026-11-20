// SPDX-FileCopyrightText: 2026 Tianyu (Sky) Lu
// SPDX-License-Identifier: AGPL-3.0-only

use super::*;

fn place(
    map: &mut ImageMap,
    image_id: u32,
    image_type: ImageType,
    top_left: AbsolutePoint,
    z_index: i32,
) {
    map.add_image_placement_data(
        image_id,
        1,
        ImagePlacementData {
            z_index,
            width_cells: 2,
            height_cells: 3,
            image_size: Vector2F::new(2.0, 3.0),
        },
    );
    map.place(image_id, 1, top_left, image_type, 0);
}

#[test]
fn placed_image_covers_the_cells_under_it() {
    let placed = PlacedImage {
        image_id: 1,
        placement_id: 1,
        top_left: AbsolutePoint { row: 4, col: 5 },
        z_index: 0,
        width_cells: 2,
        height_cells: 3,
    };

    assert!(placed.covers(AbsolutePoint { row: 4, col: 5 }));
    assert!(placed.covers(AbsolutePoint { row: 6, col: 6 }));
    assert!(!placed.covers(AbsolutePoint { row: 7, col: 5 }));
    assert!(!placed.covers(AbsolutePoint { row: 4, col: 7 }));
    assert!(!placed.covers(AbsolutePoint { row: 3, col: 5 }));
    assert!(!placed.covers(AbsolutePoint { row: 4, col: 4 }));
}

#[test]
fn evicting_placements_of_one_type_leaves_the_other_type() {
    let mut map = ImageMap::default();
    place(
        &mut map,
        1,
        ImageType::Kitty,
        AbsolutePoint { row: 0, col: 0 },
        0,
    );
    place(
        &mut map,
        2,
        ImageType::Kitty,
        AbsolutePoint { row: 0, col: 4 },
        -1,
    );
    place(
        &mut map,
        3,
        ImageType::ITerm,
        AbsolutePoint { row: 0, col: 8 },
        0,
    );

    let evicted =
        map.evict_placements_with_type_where(&ImageType::Kitty, |placed| placed.z_index == 0);

    assert_eq!(evicted, HashSet::from([1]));
    assert!(!map.has_placement_of_image(1));
    assert!(map.has_placement_of_image(2));
    assert!(map.has_placement_of_image(3));

    let evicted = map.evict_placements_with_type_where(&ImageType::Kitty, |_| true);

    assert_eq!(evicted, HashSet::from([2]));
    assert!(map.has_placement_of_image(3));
}
