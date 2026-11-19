use std::collections::HashSet;

use crate::terminal::model::grid::Dimensions as _;
use crate::terminal::model::image_map::{ImagePlacementData, ImageType, PlacedImage};
use crate::terminal::model::kitty::DeletionType;

use super::{AbsolutePoint, AbsoluteRectangle, GridHandler};
use warp_terminal::model::Point;

impl GridHandler {
    pub fn get_image_ids_in_range(
        &self,
        displayed_start_row: usize,
        displayed_end_row: usize,
    ) -> Vec<ImagePlacement> {
        if self.has_displayed_output() {
            return vec![];
        }

        if displayed_start_row > displayed_end_row {
            return vec![];
        }

        self.images
            .get_image_ids_by_rectangle(AbsoluteRectangle::from_range(
                displayed_start_row,
                displayed_end_row,
                self,
            ))
            .into_iter()
            .filter_map(|absolute_image_placement| {
                absolute_image_placement
                    .top_left
                    .to_point(self)
                    .map(|top_left| ImagePlacement {
                        image_id: absolute_image_placement.image_id,
                        placement_id: absolute_image_placement.placement_id,
                        z_index: absolute_image_placement.z_index,
                        top_left,
                    })
            })
            .collect()
    }

    pub(in crate::terminal::model) fn has_image_in_row(&self, displayed_row: usize) -> bool {
        if self.has_displayed_output() {
            return false;
        }
        let absolute_row = AbsolutePoint::from_point(Point::new(displayed_row, 0), self).row;
        self.images.has_image_in_row(absolute_row)
    }

    pub fn get_image_placement_data(
        &self,
        image_id: u32,
        placement_id: u32,
    ) -> Option<&ImagePlacementData> {
        self.images.get_image_placement_data(image_id, placement_id)
    }

    /// Whether the image with the given id has a placement in this grid.
    pub fn has_placement_of_image(&self, image_id: u32) -> bool {
        self.images.has_placement_of_image(image_id)
    }

    /// Deletes the Kitty placements in this grid that a delete command selects, and returns the
    /// ids of the images that lost a placement. Images of other protocols are left alone.
    ///
    /// The cells a command names are those of the screen, the rows the grid shows below its
    /// scrollback.
    pub fn delete_kitty_placements(&mut self, deletion_type: &DeletionType) -> HashSet<u32> {
        let screen_cell = |column: u32, row: u32| {
            let column = (column as usize).checked_sub(1)?;
            let row = (row as usize).checked_sub(1)?;
            Some(AbsolutePoint::from_point(
                Point::new(self.history_size() + row, column),
                self,
            ))
        };

        let should_delete: Box<dyn Fn(&PlacedImage) -> bool> = match deletion_type {
            DeletionType::DeleteAll => Box::new(|_: &PlacedImage| true),
            DeletionType::DeleteById(delete_by_id) => {
                let image_id = delete_by_id.image_id;
                let placement_id = delete_by_id.placement_id;
                Box::new(move |placed: &PlacedImage| {
                    placed.image_id == image_id
                        && placement_id
                            .is_none_or(|placement_id| placed.placement_id == placement_id)
                })
            }
            DeletionType::AtCursor => {
                let cursor = AbsolutePoint::from_point(self.cursor_point(), self);
                Box::new(move |placed: &PlacedImage| placed.covers(cursor))
            }
            DeletionType::AtCell {
                column,
                row,
                z_index,
            } => {
                let Some(cell) = screen_cell(*column, *row) else {
                    return HashSet::new();
                };
                let z_index = *z_index;
                Box::new(move |placed: &PlacedImage| {
                    placed.covers(cell) && z_index.is_none_or(|z_index| placed.z_index == z_index)
                })
            }
            DeletionType::InColumn(column) => {
                let Some(cell) = screen_cell(*column, 1) else {
                    return HashSet::new();
                };
                Box::new(move |placed: &PlacedImage| placed.covers_column(cell.col))
            }
            DeletionType::InRow(row) => {
                let Some(cell) = screen_cell(1, *row) else {
                    return HashSet::new();
                };
                Box::new(move |placed: &PlacedImage| placed.covers_row(cell.row))
            }
            DeletionType::WithZIndex(z_index) => {
                let z_index = *z_index;
                Box::new(move |placed: &PlacedImage| placed.z_index == z_index)
            }
            DeletionType::InIdRange { first, last } => {
                let ids = *first..=*last;
                Box::new(move |placed: &PlacedImage| ids.contains(&placed.image_id))
            }
        };

        self.images
            .evict_placements_with_type_where(&ImageType::Kitty, should_delete)
    }
}

pub struct ImagePlacement {
    pub image_id: u32,
    pub placement_id: u32,
    pub z_index: i32,
    pub top_left: Point,
}
