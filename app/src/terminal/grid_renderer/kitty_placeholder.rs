// SPDX-FileCopyrightText: 2026 Tianyu (Sky) Lu
// SPDX-License-Identifier: AGPL-3.0-only

//! Draws the images that Kitty unicode placeholder cells stand for.

use std::collections::HashMap;

use zap_ng_ui::assets::asset_cache::{AssetCache, AssetSource, AssetState};
use zap_ng_ui::elements::CornerRadius;
use zap_ng_ui::geometry::rect::RectF;
use zap_ng_ui::geometry::vector::{vec2f, Vector2F};
use zap_ng_ui::image_cache::{AnimatedImageBehavior, CacheOption, FitType, Image, ImageCache};
use zap_ng_ui::{AppContext, ClipBounds, PaintContext, SingletonEntity};

use crate::terminal::model::cell::Cell;
use crate::terminal::model::image_map::StoredImageMetadata;
use crate::terminal::model::kitty_placeholder::{image_rect_in_box, PlaceholderCell};

/// Placeholder cells next to each other in a grid row that show neighbouring columns of one row
/// of an image, and so can be drawn as one piece of it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PlaceholderRun {
    /// What the run's first cell shows.
    first: PlaceholderCell,
    /// Where the run's first cell is in the rendered grid.
    grid_row: usize,
    grid_column: usize,
    /// The number of cells in the run.
    len: usize,
}

/// Collects the placeholder cells of a grid while it is rendered, to draw their images once the
/// cells are done.
#[derive(Default)]
pub(super) struct PlaceholderRuns {
    runs: Vec<PlaceholderRun>,
    /// The placeholder cell recorded last and where it is in the rendered grid.
    last: Option<(usize, usize, PlaceholderCell)>,
}

impl PlaceholderRuns {
    /// Records the placeholder cell at the given row and column of the rendered grid. Cells are
    /// expected in reading order.
    pub(super) fn push(&mut self, cell: &Cell, grid_row: usize, grid_column: usize) {
        let left = self.last.and_then(|(row, column, placeholder)| {
            (row == grid_row && column + 1 == grid_column).then_some(placeholder)
        });

        let Some(placeholder) = PlaceholderCell::from_cell(cell, left) else {
            self.last = None;
            return;
        };
        self.last = Some((grid_row, grid_column, placeholder));

        // A cell that continues the cell to its left belongs to the run that cell is in, if it
        // shows the next column of the same row of the same image.
        let continued_run = self.runs.last_mut().filter(|run| {
            left.is_some()
                && run.first.image_id == placeholder.image_id
                && run.first.row == placeholder.row
                && run.first.column + run.len as u32 == placeholder.column
        });

        match continued_run {
            Some(run) => run.len += 1,
            None => self.runs.push(PlaceholderRun {
                first: placeholder,
                grid_row,
                grid_column,
                len: 1,
            }),
        }
    }

    /// Draws, for each run of placeholder cells, the part of the image those cells show.
    pub(super) fn render(
        self,
        image_metadata: &HashMap<u32, StoredImageMetadata>,
        cell_size: Vector2F,
        grid_origin: Vector2F,
        ctx: &mut PaintContext,
        app: &AppContext,
    ) {
        for run in self.runs {
            let image_id = run.first.image_id;
            let Some(StoredImageMetadata::Kitty(metadata)) = image_metadata.get(&image_id) else {
                continue;
            };

            // A cell says which of an image's virtual placements it belongs to by its underline
            // colour, which the grid doesn't keep, so an image is shown through its first one.
            let Some(placement) = metadata.virtual_placements.values().next() else {
                continue;
            };

            let image_size = metadata.image_size;
            if image_size.x() == 0. || image_size.y() == 0. {
                continue;
            }

            let (columns, rows) = placement.size_in_cells(image_size, cell_size);
            if run.first.row >= rows || run.first.column >= columns {
                continue;
            }

            // The placement spans a box of cells, which the image is fitted into. The run shows
            // the part of that box that lies under its cells.
            let box_size = cell_size * vec2f(columns as f32, rows as f32);
            let image_rect = image_rect_in_box(image_size, box_size);
            let run_origin =
                grid_origin + cell_size * vec2f(run.grid_column as f32, run.grid_row as f32);
            let box_origin =
                run_origin - cell_size * vec2f(run.first.column as f32, run.first.row as f32);
            let run_columns = (run.len as u32).min(columns - run.first.column);
            let run_rect = RectF::new(run_origin, cell_size * vec2f(run_columns as f32, 1.));

            let image = ImageCache::as_ref(app).image(
                AssetSource::Raw {
                    id: image_id.to_string(),
                },
                (image_rect.size() * ctx.scene.scale_factor()).to_i32(),
                FitType::Stretch,
                AnimatedImageBehavior::FullAnimation,
                CacheOption::BySize,
                ctx.max_texture_dimension_2d,
                AssetCache::as_ref(app),
            );

            let AssetState::Loaded { data: image } = image else {
                continue;
            };

            match image.as_ref() {
                Image::Static(image) => {
                    ctx.scene
                        .start_layer(ClipBounds::BoundedByActiveLayerAnd(run_rect));
                    ctx.scene.draw_image(
                        RectF::new(box_origin + image_rect.origin(), image_rect.size()),
                        image.clone(),
                        1.,
                        CornerRadius::default(),
                    );
                    ctx.scene.stop_layer();
                }
                Image::Animated(_) => {
                    log::warn!("Image should be static");
                }
            }
        }
    }
}

#[cfg(test)]
#[path = "kitty_placeholder_test.rs"]
mod tests;
