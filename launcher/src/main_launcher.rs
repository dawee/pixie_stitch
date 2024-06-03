use cottontail::core::*;
use cottontail::image::Grid;
use cottontail::image::{bitmap::*, color::hsl, font::*};
use cottontail::math::*;
use cottontail::{core::PathHelper, image::ColorBlendMode};

use gif::SetParameter;
use indexmap::IndexMap;
use rayon::prelude::*;

use std::fs::File;
use std::collections::{HashMap};
use color_art::{distance, Color as ArtColor};


////////////////////////////////////////////////////////////////////////////////////////////////////
// Constants

const TILE_SIZE: i32 = 16;
const LEGEND_BLOCK_ENTRY_COUNT: usize = 5;
const SPLIT_SEGMENT_WIDTH: i32 = 60;
const SPLIT_SEGMENT_HEIGHT: i32 = 80;
const COLOR_GRID_THIN: PixelRGBA = PixelRGBA::new(128, 128, 128, 255);
const COLOR_GRID_THICK: PixelRGBA = PixelRGBA::new(64, 64, 64, 255);

enum PatternType {
    BlackAndWhite,
    Colorized,
    ColorizedNoSymbols,
    PaintByNumbers,
}

struct Resources {
    font: BitmapFont,
    font_big: BitmapFont,
    stitch_background_image_8x8_premultiplied_alpha: Bitmap,
}

#[derive(Clone)]
struct ColorInfo {
    pub color: PixelRGBA,
    pub count: usize,
    pub symbol: Bitmap,
    pub symbol_alphanum: Bitmap,
    pub stitches_premultiplied: Vec<Bitmap>,
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Paths

fn get_executable_dir() -> String {
    if let Some(executable_path) = std::env::current_exe().ok() {
        path_without_filename(executable_path.to_string_borrowed_or_panic())
    } else {
        ".".to_owned()
    }
}

/// Example:
/// exe path: "C:\bin\pixie_stitch.exe"
/// imagepath: "D:\images\example_image.png"
/// output_dir_suffix: "centered"
///
/// This returns:
/// "C:\bin\example_image_centered"
fn get_image_output_dir(image_filepath: &str, output_dir_suffix: &str) -> String {
    let image_filename = path_to_filename_without_extension(image_filepath);
    let output_dir_root = get_executable_dir();
    if output_dir_suffix.is_empty() {
        path_join(&output_dir_root, &image_filename)
    } else {
        path_join(
            &output_dir_root,
            &(image_filename + "_" + output_dir_suffix),
        )
    }
}

// NOTE: This is for quicker testing to keep images open in imageviewer
#[cfg(debug_assertions)]
fn create_image_output_dir(image_filepath: &str, output_dir_suffix: &str) {
    let output_dir = get_image_output_dir(image_filepath, output_dir_suffix);
    if !path_exists(&output_dir) {
        std::fs::create_dir_all(&output_dir)
            .expect(&format!("Cannot create directory '{}'", &output_dir));
    }
}

#[cfg(not(debug_assertions))]
fn create_image_output_dir(image_filepath: &str, output_dir_suffix: &str) {
    let output_dir = get_image_output_dir(image_filepath, output_dir_suffix);
    if path_exists(&output_dir) {
        std::fs::remove_dir_all(&output_dir).expect(&format!(
            "Cannot overwrite directory '{}': is a file from it still open?",
            &output_dir
        ));
    }
    std::fs::create_dir_all(&output_dir)
        .expect(&format!("Cannot create directory '{}'", &output_dir));
}

fn get_image_output_filepath(image_filepath: &str, output_dir_suffix: &str) -> String {
    let output_dir = get_image_output_dir(image_filepath, output_dir_suffix);
    let image_filename = path_to_filename_without_extension(image_filepath);
    path_join(&output_dir, &image_filename)
}


fn get_image_filepaths_from_commandline() -> Vec<String> {
    let mut args: Vec<String> = std::env::args().collect();

    // NOTE: The first argument is the executable path
    args.remove(0);

    assert!(
        !args.is_empty(),
        "Please drag and drop one (or more) image(s) onto the executable"
    );

    args
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Loading resources

fn get_resource_dir_path() -> String {
    let executable_dir_path = get_executable_dir();
    let resource_dir_path = {
        let candidate = path_join(&executable_dir_path, "resources");

        if path_exists(&candidate) {
            candidate
        } else {
            // There was no symbols dir in the executable dir. Lets try our current workingdir
            "resources".to_owned()
        }
    };

    assert!(
        path_exists(&resource_dir_path),
        "Missing `resources` path in '{}'",
        executable_dir_path
    );

    resource_dir_path
}

fn load_stitch_preview_images_premultiplied_alpha() -> (Vec<Bitmap>, Vec<Bitmap>, Bitmap) {
    let resource_dir_path = get_resource_dir_path();
    let background_tile_image_8x8 =
        Bitmap::from_png_file_or_panic(&path_join(&resource_dir_path, "aida_8x8.png"))
            .to_premultiplied_alpha();
    let stitch_tile_images = ["stitch1.png", "stitch2.png", "stitch3.png"]
        .iter()
        .map(|filename| {
            Bitmap::from_png_file_or_panic(&path_join(&resource_dir_path, filename))
                .to_premultiplied_alpha()
        })
        .collect();
    let stitch_tile_images_luminance = ["stitch1_lum.png", "stitch2_lum.png", "stitch3_lum.png"]
        .iter()
        .map(|filename| {
            Bitmap::from_png_file_or_panic(&path_join(&resource_dir_path, filename))
                .to_premultiplied_alpha()
        })
        .collect();
    (
        stitch_tile_images,
        stitch_tile_images_luminance,
        background_tile_image_8x8,
    )
}

pub fn load_fonts() -> (BitmapFont, BitmapFont) {
    let mut font_regular = BitmapFont::new(
        FONT_DEFAULT_TINY_NAME,
        FONT_DEFAULT_TINY_TTF,
        FONT_DEFAULT_TINY_PIXEL_HEIGHT,
        FONT_DEFAULT_TINY_RASTER_OFFSET,
        0,
        0,
        PixelRGBA::black(),
        PixelRGBA::transparent(),
    );
    let mut font_big = BitmapFont::new(
        FONT_DEFAULT_REGULAR_NAME,
        FONT_DEFAULT_REGULAR_TTF,
        2 * FONT_DEFAULT_REGULAR_PIXEL_HEIGHT,
        FONT_DEFAULT_REGULAR_RASTER_OFFSET,
        0,
        0,
        PixelRGBA::black(),
        PixelRGBA::transparent(),
    );

    // NOTE: Because 0 looks like an 8 in this font on crappy printers we replace it with an O (big o)
    let regular_o = font_regular
        .glyphs
        .get(&('O' as Codepoint))
        .unwrap()
        .clone();
    let big_o = font_big.glyphs.get(&('O' as Codepoint)).unwrap().clone();
    font_regular.glyphs.insert('0' as Codepoint, regular_o);
    font_big.glyphs.insert('0' as Codepoint, big_o);

    (font_regular, font_big)
}

fn collect_symbols() -> Vec<Bitmap> {
    let resource_dir_path = get_resource_dir_path();
    let symbols_filepaths = collect_files_by_extension_recursive(&resource_dir_path, ".png");

    let sym_paths: Vec<Bitmap> = symbols_filepaths
        .into_iter()
        .filter(|filepath| {
            path_to_filename_without_extension(filepath)
                .parse::<u32>()
                .is_ok()
        })
        .map(|symbol_filepath| Bitmap::from_png_file_or_panic(&symbol_filepath))
        .collect();

    sym_paths
}

fn create_alphanumeric_symbols(font: &BitmapFont) -> Vec<Bitmap> {
    let mut symbols = Vec::new();
    for c in "123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ".chars() {
        let mut bitmap =
            Bitmap::new_filled(TILE_SIZE as u32, TILE_SIZE as u32, PixelRGBA::transparent());
        // NOTE: We can unwrap here because we own the font and know that all glyphs exist
        let glyph_bitmap = font
            .glyphs
            .get(&(c as Codepoint))
            .as_ref()
            .unwrap()
            .bitmap
            .as_ref()
            .unwrap();
        let pos = Vec2i::new(
            block_centered_in_block(glyph_bitmap.width, TILE_SIZE),
            block_centered_in_block(glyph_bitmap.height, TILE_SIZE),
        );
        blit_symbol(glyph_bitmap, &mut bitmap, pos, PixelRGBA::transparent());
        symbols.push(bitmap);
    }

    symbols
}

fn open_image(image_filepath: &str) -> Bitmap {
    if path_to_extension(&image_filepath).ends_with("gif") {
        bitmap_create_from_gif_file(&image_filepath)
    } else if path_to_extension(&image_filepath).ends_with("png") {
        Bitmap::from_png_file_or_panic(&image_filepath)
    } else {
        panic!("We only support GIF or PNG images");
    }
}

fn convert_image(image: &Bitmap, stitch_colors_mapping: &HashMap<PixelRGBA, &str>) -> Bitmap {
    Bitmap {
        width: image.width,
        height: image.height,
        data: image.data
            .iter()
            .map(|pixel| find_closest_color(pixel, stitch_colors_mapping))
            .collect()
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Low level bitmap helper function

fn blit_symbol(symbol_bitmap: &Bitmap, image: &mut Bitmap, pos: Vec2i, mask_color: PixelRGBA) {
    let symbol_rect = symbol_bitmap.rect();

    assert!(pos.x >= 0);
    assert!(pos.y >= 0);
    assert!(pos.x + symbol_rect.width() <= image.width);
    assert!(pos.y + symbol_rect.height() <= image.height);

    let dest_color = image.get(pos.x, pos.y);
    let relative_luminance = Color::from_pixelrgba(dest_color).to_relative_luminance();
    let blit_color = if relative_luminance > 0.2 {
        PixelRGBA::black()
    } else {
        PixelRGBA::white()
    };

    for y in 0..symbol_rect.height() {
        for x in 0..symbol_rect.width() {
            let symbol_pixel_color = symbol_bitmap.get(x, y);
            // NOTE: We assume the symbols-images are black on white backround. We don't want to
            //       draw the white background so we treat it as transparent
            if symbol_pixel_color != mask_color {
                image.set(pos.x + x, pos.y + y, blit_color);
            }
        }
    }
}

fn bitmap_create_from_gif_file(image_filepath: &str) -> Bitmap {
    let mut decoder = gif::Decoder::new(
        File::open(image_filepath).expect(&format!("Cannot open file '{}'", image_filepath)),
    );

    decoder.set(gif::ColorOutput::RGBA);
    let mut decoder = decoder
        .read_info()
        .expect(&format!("Cannot decode file '{}'", image_filepath));
    let frame = decoder
        .read_next_frame()
        .expect(&format!(
            "Cannot decode first frame in '{}'",
            image_filepath
        ))
        .expect(&format!("No frame found in '{}'", image_filepath));
    let buffer: Vec<PixelRGBA> = frame
        .buffer
        .chunks_exact(4)
        .into_iter()
        .map(|color| PixelRGBA::new(color[0], color[1], color[2], color[3]))
        .collect();
    Bitmap::new_from_buffer(frame.width as u32, frame.height as u32, buffer)
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Pattern creation

fn draw_origin_line_vertical(bitmap: &mut Bitmap, pos_x: i32) {
    bitmap.draw_rect_filled_safely(pos_x - 2, 0, 4, bitmap.height, PixelRGBA::black());
    bitmap.draw_rect_filled_safely(pos_x - 1, 0, 2, bitmap.height, PixelRGBA::white());
}

fn draw_origin_line_horizontal(bitmap: &mut Bitmap, pos_y: i32) {
    bitmap.draw_rect_filled_safely(0, pos_y - 2, bitmap.width, 4, PixelRGBA::black());
    bitmap.draw_rect_filled_safely(0, pos_y - 1, bitmap.width, 2, PixelRGBA::white());
}

/// NOTE: This assumes that the scaled bitmap width and height are a roughly a multiple of
///       grid_cell_size
fn place_grid_labels_in_pattern(
    scaled_bitmap: &Bitmap,
    grid_cell_size: i32,
    font: &BitmapFont,
    logical_first_coordinate_x: i32,
    logical_first_coordinate_y: i32,
) -> Bitmap {
    let grid_width = scaled_bitmap.width / grid_cell_size;
    let grid_height = scaled_bitmap.height / grid_cell_size;

    let logical_last_coordinate_x = logical_first_coordinate_x + grid_width;
    let logical_last_coordinate_y = logical_first_coordinate_y + grid_height;

    // Determine how much image-padding we need by calculating the maximum label text dimension
    let label_padding = {
        let max_logical_coordinates = [
            logical_first_coordinate_x,
            logical_first_coordinate_y,
            logical_last_coordinate_x,
            logical_last_coordinate_y,
        ];
        let max_text_charcount = max_logical_coordinates
            .iter()
            .map(|max_coordinate| max_coordinate.to_string().len())
            .max()
            .unwrap();

        font.horizontal_advance_max * (max_text_charcount + 4) as i32
    };

    let mut result_bitmap = scaled_bitmap.extended(
        label_padding,
        label_padding,
        label_padding,
        label_padding,
        PixelRGBA::white(),
    );

    // Determine all x label positions
    let label_coords_x = {
        let mut result = Vec::new();
        for bitmap_coord_x in 0..(grid_width + 1) {
            let logical_coord_x = logical_first_coordinate_x + bitmap_coord_x;
            if logical_coord_x % 10 == 0 {
                result.push((bitmap_coord_x, logical_coord_x));
            }
        }

        // Add label for first and last horizontal grid pixel so that we don't mix up a remaining
        // 7, 8 or 9 pixel block with a 10 block
        let pixel_count_in_first_block_horizontal = i32::abs(
            ceil_to_multiple_of_target_i32(logical_first_coordinate_x, 10)
                - logical_first_coordinate_x,
        );
        if pixel_count_in_first_block_horizontal > 3 {
            result.push((0, logical_first_coordinate_x));
        }
        let pixel_count_in_last_block_horizontal = i32::abs(
            floor_to_multiple_of_target_i32(logical_last_coordinate_x, 10)
                - logical_last_coordinate_x,
        );
        if pixel_count_in_last_block_horizontal > 3 {
            result.push((grid_width, logical_last_coordinate_x));
        }

        result
    };

    // Draw x labels
    for (bitmap_coord_x, logical_coord_x) in label_coords_x {
        let text = logical_coord_x.to_string();
        let draw_x = label_padding + grid_cell_size * bitmap_coord_x;
        let draw_pos_top = Vec2i::new(draw_x, label_padding / 2);
        let draw_pos_bottom = Vec2i::new(draw_x, result_bitmap.height - label_padding / 2);

        result_bitmap.draw_text_aligned_in_point(
            font,
            &text,
            1,
            draw_pos_top,
            Vec2i::zero(),
            Some(TextAlignment {
                horizontal: AlignmentHorizontal::Center,
                vertical: AlignmentVertical::Center,
                origin_is_baseline: false,
                ignore_whitespace: false,
            }),
        );
        result_bitmap.draw_text_aligned_in_point(
            font,
            &text,
            1,
            draw_pos_bottom,
            Vec2i::zero(),
            Some(TextAlignment {
                horizontal: AlignmentHorizontal::Center,
                vertical: AlignmentVertical::Center,
                origin_is_baseline: false,
                ignore_whitespace: false,
            }),
        );
    }

    // Determine all y label positions
    let label_coords_y = {
        let mut result = Vec::new();
        for bitmap_coord_y in 0..(grid_height + 1) {
            let logical_coord_y = logical_first_coordinate_y + bitmap_coord_y;
            if logical_coord_y % 10 == 0 {
                result.push((bitmap_coord_y, logical_coord_y));
            }
        }

        // Add label for first and last vertical grid pixel so that we don't mix up a remaining
        // 7, 8 or 9 pixel block with a 10 block
        let pixel_count_in_first_block_vertical = i32::abs(
            ceil_to_multiple_of_target_i32(logical_first_coordinate_y, 10)
                - logical_first_coordinate_y,
        );
        if pixel_count_in_first_block_vertical > 3 {
            result.push((0, logical_first_coordinate_y));
        }
        let pixel_count_in_last_block_vertical = i32::abs(
            floor_to_multiple_of_target_i32(logical_last_coordinate_y, 10)
                - logical_last_coordinate_y,
        );
        if pixel_count_in_last_block_vertical > 3 {
            result.push((grid_height, logical_last_coordinate_y));
        }

        result
    };

    // Draw y labels
    for (bitmap_coord_y, logical_coord_y) in label_coords_y {
        // NOTE: In pixel space our y-coordinates are y-down. We want cartesian y-up so we negate y
        let text = (-logical_coord_y).to_string();
        let draw_y = label_padding + grid_cell_size * bitmap_coord_y;
        let draw_pos_left = Vec2i::new(label_padding / 2, draw_y);
        let draw_pos_right = Vec2i::new(result_bitmap.width - label_padding / 2, draw_y);

        result_bitmap.draw_text_aligned_in_point(
            font,
            &text,
            1,
            draw_pos_left,
            Vec2i::zero(),
            Some(TextAlignment {
                horizontal: AlignmentHorizontal::Center,
                vertical: AlignmentVertical::Center,
                origin_is_baseline: false,
                ignore_whitespace: false,
            }),
        );
        result_bitmap.draw_text_aligned_in_point(
            font,
            &text,
            1,
            draw_pos_right,
            Vec2i::zero(),
            Some(TextAlignment {
                horizontal: AlignmentHorizontal::Center,
                vertical: AlignmentVertical::Center,
                origin_is_baseline: false,
                ignore_whitespace: false,
            }),
        );
    }

    result_bitmap
}

fn create_cross_stitch_pattern(
    bitmap: &Bitmap,
    font_grid_label: &BitmapFont,
    font_segment_index_indicator: &BitmapFont,
    image_filepath: &str,
    output_filename_suffix: &str,
    output_dir_suffix: &str,
    color_mappings: &IndexMap<PixelRGBA, ColorInfo>,
    segment_index: Option<usize>,
    logical_first_coordinate_x: i32,
    logical_first_coordinate_y: i32,
    pattern_type: PatternType,
    add_thick_ten_grid: bool,
    add_origin_grid_bars: bool,
    symbol_mask_color: PixelRGBA,
) {
    let (colorize, add_symbol, use_alphanum) = match pattern_type {
        PatternType::BlackAndWhite => (false, true, false),
        PatternType::Colorized => (true, true, false),
        PatternType::ColorizedNoSymbols => (true, false, false),
        PatternType::PaintByNumbers => (false, true, true),
    };

    let mut scaled_bitmap = Bitmap::new(
        (TILE_SIZE * bitmap.width) as u32,
        (TILE_SIZE * bitmap.height) as u32,
    );
    let scaled_bitmap_width = scaled_bitmap.width;
    let scaled_bitmap_height = scaled_bitmap.height;

    for y in 0..bitmap.height {
        for x in 0..bitmap.width {
            let color = bitmap.get(x, y);

            // Colorize pixels
            if colorize {
                scaled_bitmap.draw_rect_filled(
                    TILE_SIZE * x,
                    TILE_SIZE * y,
                    TILE_SIZE,
                    TILE_SIZE,
                    if color.a == 0 {
                        PixelRGBA::white()
                    } else {
                        color
                    },
                );
            } else {
                scaled_bitmap.draw_rect_filled(
                    TILE_SIZE * x,
                    TILE_SIZE * y,
                    TILE_SIZE,
                    TILE_SIZE,
                    PixelRGBA::white(),
                );
            }

            // Add symbol
            if add_symbol && color.a != 0 {
                let symbol = if use_alphanum {
                    &color_mappings.get(&color).unwrap().symbol_alphanum
                } else {
                    &color_mappings.get(&color).unwrap().symbol
                };

                blit_symbol(
                    symbol,
                    &mut scaled_bitmap,
                    Vec2i::new(TILE_SIZE * x, TILE_SIZE * y),
                    symbol_mask_color,
                );
            }
        }
    }

    // Add 1x1 grid
    for x in 0..bitmap.width {
        scaled_bitmap.draw_rect_filled(TILE_SIZE * x, 0, 1, scaled_bitmap_height, COLOR_GRID_THIN);
    }
    for y in 0..bitmap.height {
        scaled_bitmap.draw_rect_filled(0, TILE_SIZE * y, scaled_bitmap_width, 1, COLOR_GRID_THIN);
    }
    // Close 1x1 grid line on bottom-right bitmap border
    scaled_bitmap.draw_rect_filled(
        scaled_bitmap_width - 1,
        0,
        1,
        scaled_bitmap_height,
        COLOR_GRID_THIN,
    );
    scaled_bitmap.draw_rect_filled(
        0,
        scaled_bitmap_height - 1,
        scaled_bitmap_width,
        1,
        COLOR_GRID_THIN,
    );

    // Add 10x10 grid
    if add_thick_ten_grid {
        for bitmap_x in 0..bitmap.width {
            let logical_x = logical_first_coordinate_x + bitmap_x;
            if logical_x % 10 == 0 {
                scaled_bitmap.draw_rect_filled(
                    TILE_SIZE * bitmap_x,
                    0,
                    2,
                    scaled_bitmap_height,
                    COLOR_GRID_THICK,
                );
            }
        }
        for bitmap_y in 0..bitmap.height {
            let logical_y = logical_first_coordinate_y + bitmap_y;
            if logical_y % 10 == 0 {
                scaled_bitmap.draw_rect_filled(
                    0,
                    TILE_SIZE * bitmap_y,
                    scaled_bitmap_width,
                    2,
                    COLOR_GRID_THICK,
                );
            }
        }
        // Close 10x10 grid line on bottom-right bitmap border if necessary
        if (logical_first_coordinate_x + bitmap.width) % 10 == 0 {
            scaled_bitmap.draw_rect_filled(
                scaled_bitmap_width - 2,
                0,
                2,
                scaled_bitmap_height,
                COLOR_GRID_THICK,
            );
        }
        if (logical_first_coordinate_y + bitmap.height) % 10 == 0 {
            scaled_bitmap.draw_rect_filled(
                0,
                scaled_bitmap_height - 2,
                scaled_bitmap_width,
                2,
                COLOR_GRID_THICK,
            );
        }
    }

    // Add origin grid
    if add_origin_grid_bars {
        let origin_bitmap_coord_x = -logical_first_coordinate_x;
        if 0 < origin_bitmap_coord_x && origin_bitmap_coord_x < bitmap.width {
            draw_origin_line_vertical(&mut scaled_bitmap, TILE_SIZE * origin_bitmap_coord_x);
        }

        let origin_bitmap_coord_y = -logical_first_coordinate_y;
        if 0 < origin_bitmap_coord_y && origin_bitmap_coord_y < bitmap.height {
            draw_origin_line_horizontal(&mut scaled_bitmap, TILE_SIZE * origin_bitmap_coord_y);
        }

        // NOTE: If our origin grid is located on the edge of our image we want to extend our image
        //       so that the origin grid is drawn more clearly visible
        let needs_grid_left = logical_first_coordinate_x == 0;
        let needs_grid_top = logical_first_coordinate_y == 0;
        let needs_grid_right = logical_first_coordinate_x + bitmap.width == 0;
        let needs_grid_bottom = logical_first_coordinate_y + bitmap.height == 0;

        let padding_left = if needs_grid_left { 2 } else { 0 };
        let padding_top = if needs_grid_top { 2 } else { 0 };
        let padding_right = if needs_grid_right { 2 } else { 0 };
        let padding_bottom = if needs_grid_bottom { 2 } else { 0 };

        scaled_bitmap.extend(
            padding_left,
            padding_top,
            padding_right,
            padding_bottom,
            PixelRGBA::white(),
        );

        if needs_grid_left {
            draw_origin_line_vertical(&mut scaled_bitmap, 2);
        }
        if needs_grid_right {
            draw_origin_line_vertical(&mut scaled_bitmap, scaled_bitmap_width);
        }
        if needs_grid_top {
            draw_origin_line_horizontal(&mut scaled_bitmap, 2);
        }
        if needs_grid_bottom {
            draw_origin_line_horizontal(&mut scaled_bitmap, scaled_bitmap_height);
        }
    }

    // Add 10-grid labels
    let final_bitmap = if add_thick_ten_grid {
        // NOTE: At this point the scaled bitmap might not be an exact multiple of the original
        //       bitmap because we may have padded it while drawing the origin grid bars. Therefore
        //       the placement of the labels might be incorrectly shifted by two pixels. This is
        //       okay because it is not really visible and the code complexity to fix this is not
        //       worth it.
        place_grid_labels_in_pattern(
            &scaled_bitmap,
            TILE_SIZE,
            font_grid_label,
            logical_first_coordinate_x,
            logical_first_coordinate_y,
        )
    } else {
        scaled_bitmap
    };

    // Add segment index indicator if necessary
    let final_bitmap = if let Some(segment_index) = segment_index {
        let text_bitmap = Bitmap::create_from_text(
            font_segment_index_indicator,
            &format!("\n Pattern Part {} \n", segment_index),
            1,
            PixelRGBA::white(),
        );
        text_bitmap.glued_to(
            &final_bitmap,
            GluePosition::TopCenter,
            0,
            PixelRGBA::white(),
        )
    } else {
        final_bitmap
    };

    // Write out png image
    let output_filepath = get_image_output_filepath(&image_filepath, output_dir_suffix)
        + "_"
        + output_filename_suffix
        + ".png";
    Bitmap::write_to_png_file(&final_bitmap, &output_filepath);
}

fn create_cross_stitch_pattern_set(
    image: &Bitmap,
    font_grid_label: &BitmapFont,
    font_segment_index_indicator: &BitmapFont,
    image_filepath: &str,
    output_filename_suffix: &str,
    output_dir_suffix: &str,
    color_mappings: &IndexMap<PixelRGBA, ColorInfo>,
    segment_index: Option<usize>,
    logical_first_coordinate_x: i32,
    logical_first_coordinate_y: i32,
    create_paint_by_number_set: bool,
    add_origin_grid_bars: bool,
) {
    rayon::scope(|scope| {
        scope.spawn(|_| {
            create_cross_stitch_pattern(
                &image,
                font_grid_label,
                font_segment_index_indicator,
                &image_filepath,
                &("cross_stitch_colorized_".to_owned() + output_filename_suffix),
                output_dir_suffix,
                &color_mappings,
                segment_index,
                logical_first_coordinate_x,
                logical_first_coordinate_y,
                PatternType::Colorized,
                true,
                add_origin_grid_bars,
                PixelRGBA::white(),
            );
        });
        scope.spawn(|_| {
            create_cross_stitch_pattern(
                &image,
                font_grid_label,
                font_segment_index_indicator,
                &image_filepath,
                &("cross_stitch_".to_owned() + output_filename_suffix),
                output_dir_suffix,
                &color_mappings,
                segment_index,
                logical_first_coordinate_x,
                logical_first_coordinate_y,
                PatternType::BlackAndWhite,
                true,
                add_origin_grid_bars,
                PixelRGBA::white(),
            );
        });
        scope.spawn(|_| {
            create_cross_stitch_pattern(
                &image,
                font_grid_label,
                font_segment_index_indicator,
                &image_filepath,
                &("cross_stitch_colorized_no_symbols_".to_owned() + output_filename_suffix),
                output_dir_suffix,
                &color_mappings,
                segment_index,
                logical_first_coordinate_x,
                logical_first_coordinate_y,
                PatternType::ColorizedNoSymbols,
                true,
                add_origin_grid_bars,
                PixelRGBA::white(),
            );
        });
        if create_paint_by_number_set {
            scope.spawn(|_| {
                create_cross_stitch_pattern(
                    &image,
                    font_grid_label,
                    font_segment_index_indicator,
                    &image_filepath,
                    &("paint_by_numbers_".to_owned() + output_filename_suffix),
                    output_dir_suffix,
                    &color_mappings,
                    segment_index,
                    logical_first_coordinate_x,
                    logical_first_coordinate_y,
                    PatternType::PaintByNumbers,
                    false,
                    false,
                    PixelRGBA::transparent(),
                );
            });
        }
    });
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Image analysis

fn create_color_mappings_from_image(
    image: &Bitmap,
    image_filepath: &str,
    symbols: &[Bitmap],
    symbols_alphanum: &[Bitmap],
    stitch_images_premultiplied_alpha: &[Bitmap],
    stitch_images_luminance_premultiplied_alpha: &[Bitmap]
) -> IndexMap<PixelRGBA, ColorInfo> {
    let mut color_mappings = image_extract_colors_and_counts(&image);

    // Stitch symbols
    assert!(
        symbols.len() >= color_mappings.len(),
        "Not enough symbols to map {} colors found in given image '{}' for cross stitch",
        color_mappings.len(),
        &image_filepath,
    );
    for (entry, symbol) in color_mappings.values_mut().zip(symbols.iter()) {
        entry.symbol = symbol.clone();
    }

    // Alphanum symbols
    assert!(
        symbols_alphanum.len() >= color_mappings.len(),
        "Not enough symbols to map {} colors found in given image '{}' for paint by numbers",
        color_mappings.len(),
        &image_filepath,
    );
    for (entry, symbol_alphanum) in color_mappings.values_mut().zip(symbols_alphanum.iter()) {
        entry.symbol_alphanum = symbol_alphanum.clone();
    }

    // Colorized stitch tiles
    for entry in color_mappings.values_mut() {
        let color = entry.color;
        if color.a != 0 {
            for (stitch_image_premultipllied, stitch_image_luminance_premultiplied) in
                stitch_images_premultiplied_alpha
                    .iter()
                    .zip(stitch_images_luminance_premultiplied_alpha.iter())
            {
                let mut stitch = stitch_image_premultipllied.clone();

                let screen_layer = Bitmap::new_filled(
                    stitch_image_premultipllied.width as u32,
                    stitch_image_premultipllied.height as u32,
                    PixelRGBA::new(105, 109, 128, 255),
                )
                .to_premultiplied_alpha();
                screen_layer.blit_to_alpha_blended_premultiplied(
                    &mut stitch,
                    Vec2i::zero(),
                    false,
                    ColorBlendMode::Screen,
                );

                let color_layer = Bitmap::new_filled(
                    stitch_image_premultipllied.width as u32,
                    stitch_image_premultipllied.height as u32,
                    color,
                )
                .to_premultiplied_alpha();
                color_layer.blit_to_alpha_blended_premultiplied(
                    &mut stitch,
                    Vec2i::zero(),
                    false,
                    ColorBlendMode::Multiply,
                );

                let mut luminosity_layer = stitch_image_luminance_premultiplied.clone();
                let percent = (color.r as f32 + color.g as f32 + color.b as f32) / (3.0 * 255.0);
                for pixel in luminosity_layer.data.iter_mut() {
                    pixel.r /= 6 + (8.0 * percent * percent) as u8;
                    pixel.g /= 6 + (8.0 * percent * percent) as u8;
                    pixel.b /= 6 + (8.0 * percent * percent) as u8;
                    pixel.a /= 6 + (8.0 * percent * percent) as u8;
                }
                luminosity_layer.blit_to_alpha_blended_premultiplied(
                    &mut stitch,
                    Vec2i::zero(),
                    false,
                    ColorBlendMode::Luminosity,
                );

                entry
                    .stitches_premultiplied
                    .push(stitch.masked_by_premultiplied_alpha(&stitch_image_premultipllied));
            }
        }
    }

    color_mappings
}

fn colour_distance(p1: &PixelRGBA, p2: &PixelRGBA) -> i64 {
    let c1 = ArtColor::from_rgb(p1.r, p1.g, p1.b).unwrap_or_default();
    let c2 = ArtColor::from_rgb(p2.r, p2.g, p2.b).unwrap_or_default();

    (distance(&c1, &c2) * 1000f64) as i64
}

fn find_closest_color(pixel: &PixelRGBA, stitch_colors_mapping: &HashMap<PixelRGBA, &str>) -> PixelRGBA {
    let (closest, _) = stitch_colors_mapping
        .iter()
        .map(|(stitch_pixel, _)| (stitch_pixel, colour_distance(pixel, stitch_pixel)))
        .min_by(|(_, distance1), (_, distance2)| distance1.cmp(distance2))
        .unwrap_or_else(|| {
            panic!("Failed to find stitch color");
        });

    *closest
}

fn image_extract_colors_and_counts(image: &Bitmap) -> IndexMap<PixelRGBA, ColorInfo> {
    let mut color_mappings = IndexMap::new();
    for pixel in &image.data {
        if pixel.a == 0 {
            // Ignore transparent regions
            continue;
        }

        let entry = color_mappings.entry(*pixel).or_insert_with(|| ColorInfo {
            color: *pixel,
            count: 0,
            symbol: Bitmap::new_empty(),
            symbol_alphanum: Bitmap::new_empty(),
            stitches_premultiplied: Vec::new(),
        });
        entry.count += 1;
    }

    // This makes color ramps on the legend more pretty
    color_mappings.sort_by(|color_a, _info_a, color_b, _info_b| {
        PixelRGBA::compare_by_hue_luminosity_saturation(color_a, color_b)
    });

    color_mappings
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Pattern dir creation

fn create_patterns_dir(
    image: &Bitmap,
    image_filepath: &str,
    resources: &Resources,
    color_mappings: &IndexMap<PixelRGBA, ColorInfo>,
    stitch_colors_mapping: &HashMap<PixelRGBA, &str>,
) {
    let output_dir_suffix = "";

    let (segment_images, segment_coordinates) =
        image.to_segments(SPLIT_SEGMENT_WIDTH, SPLIT_SEGMENT_HEIGHT);

    rayon::scope(|scope| {
        // Legend
        scope.spawn(|_| {
            create_cross_stitch_legend(
                image.dim(),
                &color_mappings,
                &image_filepath,
                output_dir_suffix,
                &resources.font,
                &segment_coordinates,
                stitch_colors_mapping
            );
        });

        // Create patterns for complete set
        scope.spawn(|_| {
            create_cross_stitch_pattern_set(
                &image,
                &resources.font,
                &resources.font_big,
                &image_filepath,
                "complete",
                output_dir_suffix,
                &color_mappings,
                None,
                0,
                0,
                true,
                false,
            );
        });

        // Create patterns for individual segments if needed
        if segment_images.len() > 1 {
            segment_images
                .par_iter()
                .zip(segment_coordinates.par_iter())
                .enumerate()
                .for_each(|(segment_index, (segment_image, segment_coordinate))| {
                    let label_start_x = SPLIT_SEGMENT_WIDTH * segment_coordinate.x;
                    let label_start_y = SPLIT_SEGMENT_HEIGHT * segment_coordinate.y;

                    create_cross_stitch_pattern_set(
                        segment_image,
                        &resources.font,
                        &resources.font_big,
                        &image_filepath,
                        &format!("segment_{}", segment_index + 1),
                        output_dir_suffix,
                        &color_mappings,
                        Some(segment_index + 1),
                        label_start_x,
                        label_start_y,
                        false,
                        false,
                    );
                });
        }
    });
}

fn create_patterns_dir_centered(
    image: &Bitmap,
    image_filepath: &str,
    resources: &Resources,
    color_mappings: &IndexMap<PixelRGBA, ColorInfo>,
    stitch_colors_mapping: &HashMap<PixelRGBA, &str>,
) {
    let output_dir_suffix = "centered";
    let image_center_x = make_even_upwards(image.width) / 2;
    let image_center_y = make_even_upwards(image.height) / 2;

    let (segment_images, segment_coordinates) =
        image.to_segments(SPLIT_SEGMENT_WIDTH, SPLIT_SEGMENT_HEIGHT);

    rayon::scope(|scope| {
        // Legend
        scope.spawn(|_| {
            create_cross_stitch_legend(
                image.dim(),
                &color_mappings,
                &image_filepath,
                output_dir_suffix,
                &resources.font,
                &segment_coordinates,
                stitch_colors_mapping
            );
        });

        // Create patterns for complete set
        scope.spawn(|_| {
            create_cross_stitch_pattern_set(
                &image,
                &resources.font,
                &resources.font_big,
                &image_filepath,
                "complete",
                output_dir_suffix,
                &color_mappings,
                None,
                -image_center_x,
                -image_center_y,
                true,
                true,
            );
        });

        // Create patterns for individual segments if needed
        if segment_images.len() > 1 {
            segment_images
                .par_iter()
                .zip(segment_coordinates.par_iter())
                .enumerate()
                .for_each(|(segment_index, (segment_image, segment_coordinate))| {
                    let logical_first_coordinate_x =
                        SPLIT_SEGMENT_WIDTH * segment_coordinate.x - image_center_x;
                    let logical_first_coordinate_y =
                        SPLIT_SEGMENT_HEIGHT * segment_coordinate.y - image_center_y;

                    create_cross_stitch_pattern_set(
                        segment_image,
                        &resources.font,
                        &resources.font_big,
                        &image_filepath,
                        &format!("segment_{}", segment_index + 1),
                        output_dir_suffix,
                        &color_mappings,
                        Some(segment_index + 1),
                        logical_first_coordinate_x,
                        logical_first_coordinate_y,
                        false,
                        true,
                    );
                });
        }
    });
}

fn create_cross_stitch_pattern_preview(
    bitmap: &Bitmap,
    image_filepath: &str,
    output_filename_suffix: &str,
    output_dir_suffix: &str,
    resources: &Resources,
    color_mappings: &IndexMap<PixelRGBA, ColorInfo>,
) {
    let bitmap = bitmap.extended(10, 10, 10, 10, PixelRGBA::transparent());
    let tile_width = resources
        .stitch_background_image_8x8_premultiplied_alpha
        .width
        / 8;
    let tile_height = resources
        .stitch_background_image_8x8_premultiplied_alpha
        .height
        / 8;

    // Background only
    let mut background_layer = Bitmap::new(
        (tile_width * bitmap.width) as u32,
        (tile_height * bitmap.height) as u32,
    );
    for y in 0..=bitmap.height / 8 {
        for x in 0..=bitmap.width / 8 {
            let pos = Vec2i::new(
                resources
                    .stitch_background_image_8x8_premultiplied_alpha
                    .width
                    * x,
                resources
                    .stitch_background_image_8x8_premultiplied_alpha
                    .height
                    * y,
            );
            resources
                .stitch_background_image_8x8_premultiplied_alpha
                .blit_to(&mut background_layer, pos, true);
        }
    }
    // Write out png image
    let output_filepath = get_image_output_filepath(&image_filepath, output_dir_suffix)
        + "_"
        + output_filename_suffix
        + "_background.png";
    Bitmap::write_to_png_file(&background_layer, &output_filepath);

    // Stitches only
    let mut colored_stitches_layer = Bitmap::new(
        (tile_width * bitmap.width) as u32,
        (tile_height * bitmap.height) as u32,
    );

    let mut random = Random::new_from_seed(1234);
    for y in 0..bitmap.height {
        for x in 0..bitmap.width {
            let color = bitmap.get(x, y);

            // Add stitch
            if color.a != 0 {
                let tile_pos_center =
                    Vec2i::new(tile_width * x, tile_height * y) + (tile_width / 2);
                let stitches = &color_mappings.get(&color).unwrap().stitches_premultiplied;
                let stitches_count = stitches.len();
                let stitch =
                    &stitches[random.u32_bounded_exclusive(stitches_count as u32) as usize];
                let stitch_center = Vec2i::new(stitch.width / 2, stitch.height / 2);
                stitch.blit_to_alpha_blended_premultiplied(
                    &mut colored_stitches_layer,
                    tile_pos_center - stitch_center,
                    true,
                    ColorBlendMode::Normal,
                );
            }
        }
    }
    // Write out png image
    let output_filepath = get_image_output_filepath(&image_filepath, output_dir_suffix)
        + "_"
        + output_filename_suffix
        + "_stitches.png";
    Bitmap::write_to_png_file(
        &colored_stitches_layer.to_unpremultiplied_alpha(),
        &output_filepath,
    );

    // Combined
    let mut combined = background_layer;
    colored_stitches_layer.blit_to_alpha_blended_premultiplied(
        &mut combined,
        Vec2i::zero(),
        false,
        ColorBlendMode::Normal,
    );
    // Write out png image
    let output_filepath = get_image_output_filepath(&image_filepath, output_dir_suffix)
        + "_"
        + output_filename_suffix
        + ".png";
    Bitmap::write_to_png_file(&combined, &output_filepath);
}

fn create_preview_dir(
    image: &Bitmap,
    image_filepath: &str,
    resources: &Resources,
    color_mappings: &IndexMap<PixelRGBA, ColorInfo>,
) {
    let output_dir_suffix = "preview";

    rayon::scope(|scope| {
        // Create stitched preview
        scope.spawn(|_| {
            create_cross_stitch_pattern_preview(
                &image,
                &image_filepath,
                "complete",
                output_dir_suffix,
                resources,
                &color_mappings,
            );
        });
    });
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Legend creation

fn create_pattern_page_layout(font: &BitmapFont, layout_indices: &[Vec2i]) -> Bitmap {
    let caption_image =
        Bitmap::create_from_text(font, "\n\nPattern parts overview:\n", 1, PixelRGBA::white());

    let page_count = layout_indices.len();
    // NOTE: Indexes begin at 0 therefore we add 1
    let num_rows = 1 + layout_indices.iter().map(|v| v.y).max().unwrap();
    let num_columns = 1 + layout_indices.iter().map(|v| v.x).max().unwrap();

    let page_tile_dim = {
        // NOTE: We want to have a 1px visual gap between page tiles therefore we add 1
        let page_tile_width = 1 + font
            .get_text_bounding_rect(&format!(" {} ", page_count), 1, false)
            .dim
            .x;
        let page_tile_height = 1 + (page_tile_width as f32 * (9.0 / 6.0)) as i32;
        Vec2i::new(page_tile_width, page_tile_height)
    };

    let image_width = num_columns * page_tile_dim.x;
    let image_height = num_rows * page_tile_dim.y;
    let mut image = Bitmap::new_filled(image_width as u32, image_height as u32, PixelRGBA::white());
    for (page_index, pos_index) in layout_indices.iter().enumerate() {
        let pos = *pos_index * page_tile_dim;
        image.draw_rect(
            pos.x,
            pos.y,
            page_tile_dim.x - 1,
            page_tile_dim.y - 1,
            PixelRGBA::black(),
        );
        image.draw_text_aligned_in_point(
            font,
            &(page_index + 1).to_string(),
            1,
            pos + page_tile_dim / 2,
            Vec2i::zero(),
            Some(TextAlignment {
                horizontal: AlignmentHorizontal::Center,
                vertical: AlignmentVertical::Center,
                origin_is_baseline: false,
                ignore_whitespace: false,
            }),
        );
    }

    caption_image.glued_to(&image, GluePosition::TopLeft, 0, PixelRGBA::white())
}

fn create_legend_entry(font: &BitmapFont, info: &ColorInfo, stitch_colors_mapping: &HashMap<PixelRGBA, &str>) -> Bitmap {
    // Draw color and symbol mapping
    let mut color_symbol_map =
        Bitmap::new_filled(2 * TILE_SIZE as u32, TILE_SIZE as u32, PixelRGBA::white());
    color_symbol_map.draw_rect_filled(0, 0, TILE_SIZE, TILE_SIZE, info.color);
    color_symbol_map.draw_rect(
        0,
        0,
        TILE_SIZE,
        TILE_SIZE,
        PixelRGBA::from_color(Color::black()),
    );
    blit_symbol(
        &info.symbol,
        &mut color_symbol_map,
        Vec2i::filled_x(TILE_SIZE),
        PixelRGBA::white(),
    );
    color_symbol_map.draw_rect(
        0 + TILE_SIZE,
        0,
        TILE_SIZE,
        TILE_SIZE,
        PixelRGBA::from_color(Color::black()),
    );

    // Add stitches info
    let stitches_info = Bitmap::create_from_text(
        font,
        &format!(" {} stitches DMC {}", info.count, stitch_colors_mapping.get(&info.color).unwrap_or(&"")),
        1,
        PixelRGBA::white(),
    );
    stitches_info.glued_to(
        &mut color_symbol_map,
        GluePosition::RightCenter,
        0,
        PixelRGBA::white(),
    )
}

fn create_legend_block(font: &BitmapFont, infos: &[ColorInfo], stitch_colors_mapping: &HashMap<PixelRGBA, &str>) -> Bitmap {
    let entries: Vec<Bitmap> = infos
        .iter()
        .map(|entry| create_legend_entry(font, entry, stitch_colors_mapping))
        .collect();
    Bitmap::glue_together_multiple(
        &entries,
        GluePosition::BottomLeft,
        TILE_SIZE,
        PixelRGBA::white(),
    )
}

fn create_cross_stitch_legend(
    image_dimensions: Vec2i,
    color_mappings: &IndexMap<PixelRGBA, ColorInfo>,
    image_filepath: &str,
    output_dir_suffix: &str,
    font: &BitmapFont,
    segment_layout_indices: &[Vec2i],
    stitch_colors_mapping: &HashMap<PixelRGBA, &str>
) {
    let mut legend = {
        // Create color and stitch stats
        let stats_bitmap = {
            let color_count = color_mappings.len();
            let stitch_count = color_mappings
                .values()
                .fold(0, |acc, entry| acc + entry.count);

            Bitmap::create_from_text(
                &font,
                &format!(
                    "Size:     {}x{}\n\nColors:   {}\n\nStitches: {}\n\n\n",
                    image_dimensions.x, image_dimensions.y, color_count, stitch_count
                ),
                1,
                PixelRGBA::white(),
            )
        };

        // Create color mapping blocks
        let blocks = {
            let color_infos: Vec<ColorInfo> = color_mappings.values().cloned().collect();
            let block_bitmaps: Vec<Bitmap> = color_infos
                .chunks(LEGEND_BLOCK_ENTRY_COUNT)
                .map(|chunk| create_legend_block(&font, chunk, stitch_colors_mapping))
                .collect();
            let num_columns = block_bitmaps.len().max(4);
            let block_rows: Vec<Bitmap> = block_bitmaps
                .chunks(num_columns)
                .map(|chunk| {
                    Bitmap::glue_together_multiple(
                        chunk,
                        GluePosition::RightTop,
                        TILE_SIZE,
                        PixelRGBA::white(),
                    )
                })
                .collect();
            Bitmap::glue_together_multiple(
                &block_rows,
                GluePosition::BottomLeft,
                TILE_SIZE,
                PixelRGBA::white(),
            )
            .extended(0, 0, 0, (1.5 * TILE_SIZE as f32) as i32, PixelRGBA::white())
        };

        Bitmap::glue_a_to_b(
            &stats_bitmap,
            &blocks,
            GluePosition::TopLeft,
            0,
            PixelRGBA::white(),
        )
    };

    // Add page layout order if necessary
    if segment_layout_indices.len() > 1 {
        let page_layout_image = create_pattern_page_layout(&font, segment_layout_indices);

        legend = legend.glued_to(
            &page_layout_image,
            GluePosition::TopLeft,
            0,
            PixelRGBA::white(),
        );

        // Draw separating line between colors and page order layout
        for x in 0..legend.width {
            legend.set(
                x,
                legend.height - page_layout_image.height,
                PixelRGBA::black(),
            );
        }
    }

    let padding = TILE_SIZE;
    let final_image = legend.extended(padding, padding, padding, padding, PixelRGBA::white());

    // Write out png image
    let output_filepath =
        get_image_output_filepath(&image_filepath, output_dir_suffix) + "_legend.png";
    Bitmap::write_to_png_file(&final_image, &output_filepath);
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Main


fn set_panic_hook() {
    std::panic::set_hook(Box::new(|panic_info| {
        let (message, location) = panic_message_split_to_message_and_location(panic_info);
        let final_message = format!("{}\n\nError occured at: {}", message, location);

        // show_messagebox("Pixie Stitch Error", &final_message, true);

        // NOTE: This forces the other threads to shutdown as well
        std::process::abort();
    }));
}

fn main() {
    // set_panic_hook();

    // NOTE: We can uncomment this if we want to test our color sorting and symbol contrast
    // test_color_sorting();
    // test_symbols_contrast();

    let (font, font_big) = load_fonts();
    let symbols = collect_symbols();
    let symbols_alphanum = create_alphanumeric_symbols(&font);
    let (
        stitch_images_premultiplied_alpha,
        stitch_images_luminance_premultiplied_alpha,
        stitch_background_image_8x8_premultiplied_alpha,
    ) = load_stitch_preview_images_premultiplied_alpha();
    let resources = Resources {
        font,
        font_big,
        stitch_background_image_8x8_premultiplied_alpha,
    };

    let stitch_colors_mapping: HashMap<PixelRGBA, &str> = HashMap::from([
        ((255,226,226), ("3713")),
        ((255,201,201), ("761")),
        ((245,173,173), ("760")),
        ((241,135,135), ("3712")),
        ((227,109,109), ("3328")),
        ((191,45,45), ("347")),
        ((254,215,204), ("353")),
        ((253,156,151), ("352")),
        ((233,106,103), ("351")),
        ((224,72,72), ("350")),
        ((210,16,53), ("349")),
        ((187,5,31), ("817")),
        ((255,203,213), ("3708")),
        ((255,173,188), ("3706")),
        ((255,121,146), ("3705")),
        ((231,73,103), ("3801")),
        ((227,29,66), ("666")),
        ((199,43,59), ("321")),
        ((183,31,51), ("304")),
        ((167,19,43), ("498")),
        ((151,11,35), ("816")),
        ((135,7,31), ("815")),
        ((123,0,27), ("814")),
        ((255,178,187), ("894")),
        ((252,144,162), ("893")),
        ((255,121,140), ("892")),
        ((255,87,115), ("891")),
        ((255,223,217), ("818")),
        ((253,181,181), ("957")),
        ((255,145,145), ("956")),
        ((86,74,74), ("309")),
        ((255,215,215), ("963")),
        ((255,189,189), ("3716")),
        ((230,138,138), ("962")),
        ((207,115,115), ("961")),
        ((234,134,153), ("3833")),
        ((219,85,110), ("3832")),
        ((179,47,72), ("3831")),
        ((145,53,70), ("777")),
        ((255,238,235), ("819")),
        ((251,173,180), ("3326")),
        ((252,176,185), ("776")),
        ((242,118,136), ("899")),
        ((238,84,110), ("335")),
        ((179,59,75), ("326")),
        ((240,206,212), ("151")),
        ((228,166,172), ("3354")),
        ((232,135,155), ("3733")),
        ((218,103,131), ("3731")),
        ((188,67,101), ("3350")),
        ((171,2,73), ("150")),
        ((251,191,194), ("3689")),
        ((231,169,172), ("3688")),
        ((201,107,112), ("3687")),
        ((171,51,87), ("3803")),
        ((136,21,49), ("3685")),
        ((255,192,205), ("605")),
        ((255,176,190), ("604")),
        ((255,164,190), ("603")),
        ((226,72,116), ("602")),
        ((209,40,106), ("601")),
        ((205,47,99), ("600")),
        ((255,140,174), ("3806")),
        ((243,71,139), ("3805")),
        ((224,40,118), ("3804")),
        ((244,174,213), ("3609")),
        ((234,156,196), ("3608")),
        ((197,73,137), ("3607")),
        ((156,36,98), ("718")),
        ((155,19,89), ("917")),
        ((130,0,67), ("915")),
        ((255,223,213), ("225")),
        ((235,183,175), ("224")),
        ((226,160,153), ("152")),
        ((204,132,124), ("223")),
        ((188,108,100), ("3722")),
        ((161,75,81), ("3721")),
        ((136,62,67), ("221")),
        ((223,179,187), ("778")),
        ((219,169,178), ("3727")),
        ((183,115,127), ("316")),
        ((155,91,102), ("3726")),
        ((129,73,82), ("315")),
        ((113,65,73), ("3802")),
        ((130,38,55), ("902")),
        ((215,203,211), ("3743")),
        ((183,157,167), ("3042")),
        ((149,111,124), ("3041")),
        ((120,87,98), ("3740")),
        ((186,145,170), ("3836")),
        ((148,96,131), ("3835")),
        ((114,55,93), ("3834")),
        ((87,36,51), ("154")),
        ((227,203,227), ("211")),
        ((195,159,195), ("210")),
        ((163,123,167), ("209")),
        ((131,91,139), ("208")),
        ((108,58,110), ("3837")),
        ((99,54,102), ("327")),
        ((230,204,217), ("153")),
        ((219,179,203), ("554")),
        ((163,99,139), ("553")),
        ((128,58,107), ("552")),
        ((92,24,78), ("550")),
        ((211,215,237), ("3747")),
        ((183,191,221), ("341")),
        ((163,174,209), ("156")),
        ((173,167,199), ("340")),
        ((152,145,182), ("155")),
        ((119,107,152), ("3746")),
        ((92,84,120), ("333")),
        ((187,195,217), ("157")),
        ((143,156,193), ("794")),
        ((112,125,162), ("793")),
        ((96,103,140), ("3807")),
        ((85,91,123), ("792")),
        ((76,82,110), ("158")),
        ((70,69,99), ("791")),
        ((176,192,218), ("3840")),
        ((123,142,171), ("3839")),
        ((92,114,148), ("3838")),
        ((192,204,222), ("800")),
        ((148,168,198), ("809")),
        ((116,142,182), ("799")),
        ((70,106,142), ("798")),
        ((19,71,125), ("797")),
        ((17,65,109), ("796")),
        ((14,54,92), ("820")),
        ((219,236,245), ("162")),
        ((189,221,237), ("827")),
        ((161,194,215), ("813")),
        ((107,158,191), ("826")),
        ((71,129,165), ("825")),
        ((57,105,135), ("824")),
        ((48,194,236), ("996")),
        ((20,170,208), ("3843")),
        ((38,150,182), ("995")),
        ((6,227,230), ("3846")),
        ((4,196,202), ("3845")),
        ((18,174,186), ("3844")),
        ((199,202,215), ("159")),
        ((153,159,183), ("160")),
        ((120,128,164), ("161")),
        ((238,252,252), ("3756")),
        ((217,235,241), ("775")),
        ((205,223,237), ("3841")),
        ((184,210,230), ("3325")),
        ((147,180,206), ("3755")),
        ((115,159,193), ("334")),
        ((90,143,184), ("322")),
        ((53,102,139), ("312")),
        ((44,89,124), ("803")),
        ((37,59,115), ("336")),
        ((33,48,99), ("823")),
        ((27,40,83), ("939")),
        ((219,226,233), ("3753")),
        ((199,209,219), ("3752")),
        ((162,181,198), ("932")),
        ((106,133,158), ("931")),
        ((69,92,113), ("930")),
        ((56,76,94), ("3750")),
        ((197,232,237), ("828")),
        ((172,216,226), ("3761")),
        ((126,177,200), ("519")),
        ((79,147,167), ("518")),
        ((62,133,162), ("3760")),
        ((59,118,143), ("517")),
        ((50,102,124), ("3842")),
        ((28,80,102), ("311")),
        ((229,252,253), ("747")),
        ((153,207,217), ("3766")),
        ((100,171,186), ("807")),
        ((61,149,165), ("806")),
        ((52,127,140), ("3765")),
        ((188,227,230), ("3811")),
        ((144,195,204), ("598")),
        ((91,163,179), ("597")),
        ((72,142,154), ("3810")),
        ((63,124,133), ("3809")),
        ((54,105,112), ("3808")),
        ((221,227,227), ("928")),
        ((189,203,203), ("927")),
        ((152,174,174), ("926")),
        ((101,127,127), ("3768")),
        ((86,106,106), ("924")),
        ((82,179,164), ("3849")),
        ((85,147,146), ("3848")),
        ((52,125,117), ("3847")),
        ((169,226,216), ("964")),
        ((89,199,180), ("959")),
        ((62,182,161), ("958")),
        ((47,140,132), ("3812")),
        ((73,179,161), ("3851")),
        ((61,147,132), ("943")),
        ((55,132,119), ("3850")),
        ((144,192,180), ("993")),
        ((111,174,159), ("992")),
        ((80,139,125), ("3814")),
        ((71,123,110), ("991")),
        ((185,215,192), ("966")),
        ((167,205,175), ("564")),
        ((143,192,152), ("563")),
        ((83,151,106), ("562")),
        ((51,131,98), ("505")),
        ((153,195,170), ("3817")),
        ((101,165,125), ("3816")),
        ((77,131,97), ("163")),
        ((71,119,89), ("3815")),
        ((44,106,69), ("561")),
        ((196,222,204), ("504")),
        ((178,212,189), ("3813")),
        ((123,172,148), ("503")),
        ((91,144,113), ("502")),
        ((57,111,82), ("501")),
        ((4,77,51), ("500")),
        ((162,214,173), ("955")),
        ((136,186,145), ("954")),
        ((109,171,119), ("913")),
        ((27,157,107), ("912")),
        ((24,144,101), ("911")),
        ((24,126,86), ("910")),
        ((21,111,73), ("909")),
        ((17,90,59), ("3818")),
        ((215,237,204), ("369")),
        ((166,194,152), ("368")),
        ((105,136,90), ("320")),
        ((97,122,82), ("367")),
        ((32,95,46), ("319")),
        ((23,73,35), ("890")),
        ((200,216,184), ("164")),
        ((141,166,117), ("989")),
        ((115,139,91), ("988")),
        ((88,113,65), ("987")),
        ((64,82,48), ("986")),
        ((228,236,212), ("772")),
        ((204,217,177), ("3348")),
        ((113,147,92), ("3347")),
        ((64,106,58), ("3346")),
        ((27,89,21), ("3345")),
        ((27,83,0), ("895")),
        ((158,207,52), ("704")),
        ((123,181,71), ("703")),
        ((71,167,47), ("702")),
        ((63,143,41), ("701")),
        ((7,115,27), ("700")),
        ((5,101,23), ("699")),
        ((199,230,102), ("907")),
        ((127,179,53), ("906")),
        ((98,138,40), ("905")),
        ((85,120,34), ("904")),
        ((216,228,152), ("472")),
        ((174,191,121), ("471")),
        ((148,171,79), ("470")),
        ((114,132,60), ("469")),
        ((98,113,51), ("937")),
        ((76,88,38), ("936")),
        ((66,77,33), ("935")),
        ((49,57,25), ("934")),
        ((171,177,151), ("523")),
        ((156,164,130), ("3053")),
        ((136,146,104), ("3052")),
        ((95,102,72), ("3051")),
        ((196,205,172), ("524")),
        ((150,158,126), ("522")),
        ((102,109,79), ("520")),
        ((131,151,95), ("3364")),
        ((114,130,86), ("3363")),
        ((94,107,71), ("3362")),
        ((239,244,164), ("165")),
        ((224,232,104), ("3819")),
        ((192,200,64), ("166")),
        ((167,174,56), ("581")),
        ((136,141,51), ("580")),
        ((199,192,119), ("734")),
        ((188,179,76), ("733")),
        ((148,140,54), ("732")),
        ((147,139,55), ("731")),
        ((130,123,48), ("730")),
        ((185,185,130), ("3013")),
        ((166,167,93), ("3012")),
        ((137,138,88), ("3011")),
        ((204,183,132), ("372")),
        ((191,166,113), ("371")),
        ((184,157,100), ("370")),
        ((219,190,127), ("834")),
        ((200,171,108), ("833")),
        ((189,155,81), ("832")),
        ((170,143,86), ("831")),
        ((141,120,75), ("830")),
        ((126,107,66), ("829")),
        ((220,196,170), ("613")),
        ((188,154,120), ("612")),
        ((150,118,86), ("611")),
        ((121,96,71), ("610")),
        ((231,214,193), ("3047")),
        ((216,188,154), ("3046")),
        ((188,150,106), ("3045")),
        ((167,124,73), ("167")),
        ((252,252,238), ("746")),
        ((245,236,203), ("677")),
        ((198,159,123), ("422")),
        ((183,139,97), ("3828")),
        ((160,112,66), ("420")),
        ((131,94,57), ("869")),
        ((228,180,104), ("728")),
        ((206,145,36), ("783")),
        ((174,119,32), ("782")),
        ((162,109,32), ("781")),
        ((148,99,26), ("780")),
        ((229,206,151), ("676")),
        ((208,165,62), ("729")),
        ((188,141,14), ("680")),
        ((169,130,4), ("3829")),
        ((246,220,152), ("3822")),
        ((243,206,117), ("3821")),
        ((223,182,95), ("3820")),
        ((205,157,55), ("3852")),
        ((255,251,139), ("445")),
        ((253,237,84), ("307")),
        ((255,227,0), ("973")),
        ((255,214,0), ("444")),
        ((253,249,205), ("3078")),
        ((255,241,175), ("727")),
        ((253,215,85), ("726")),
        ((255,200,64), ("725")),
        ((255,181,21), ("972")),
        ((255,233,173), ("745")),
        ((255,231,147), ("744")),
        ((254,211,118), ("743")),
        ((255,191,87), ("742")),
        ((255,163,43), ("741")),
        ((255,139,0), ("740")),
        ((247,139,19), ("970")),
        ((246,127,0), ("971")),
        ((255,123,77), ("947")),
        ((235,99,7), ("946")),
        ((209,88,7), ("900")),
        ((255,222,213), ("967")),
        ((254,205,194), ("3824")),
        ((252,171,152), ("3341")),
        ((255,131,111), ("3340")),
        ((253,93,53), ("608")),
        ((250,50,3), ("606")),
        ((255,226,207), ("951")),
        ((255,211,181), ("3856")),
        ((247,151,111), ("722")),
        ((242,120,66), ("721")),
        ((229,92,31), ("720")),
        ((253,189,150), ("3825")),
        ((226,115,35), ("922")),
        ((198,98,24), ("921")),
        ((172,84,20), ("920")),
        ((166,69,16), ("919")),
        ((130,52,10), ("918")),
        ((255,238,227), ("3770")),
        ((251,213,187), ("945")),
        ((247,167,119), ("402")),
        ((207,121,57), ("3776")),
        ((179,95,43), ("301")),
        ((143,67,15), ("400")),
        ((111,47,0), ("300")),
        ((255,253,227), ("3823")),
        ((250,211,150), ("3855")),
        ((242,175,104), ("3854")),
        ((242,151,70), ("3853")),
        ((247,187,119), ("3827")),
        ((220,156,86), ("977")),
        ((194,129,66), ("976")),
        ((173,114,57), ("3826")),
        ((145,79,18), ("975")),
        ((254,231,218), ("948")),
        ((247,203,191), ("754")),
        ((244,187,169), ("3771")),
        ((238,170,155), ("758")),
        ((217,137,120), ("3778")),
        ((197,106,91), ("356")),
        ((185,85,68), ("3830")),
        ((152,68,54), ("355")),
        ((134,48,34), ("3777")),
        ((248,202,200), ("3779")),
        ((186,139,124), ("3859")),
        ((150,74,63), ("3858")),
        ((104,37,26), ("3857")),
        ((243,225,215), ("3774")),
        ((238,211,196), ("950")),
        ((196,142,112), ("3064")),
        ((187,129,97), ("407")),
        ((182,117,82), ("3773")),
        ((160,108,80), ("3772")),
        ((135,85,57), ("632")),
        ((215,206,203), ("453")),
        ((192,179,174), ("452")),
        ((145,123,115), ("451")),
        ((166,136,129), ("3861")),
        ((125,93,87), ("3860")),
        ((98,75,69), ("779")),
        ((255,251,239), ("712")),
        ((248,228,200), ("739")),
        ((236,204,158), ("738")),
        ((228,187,142), ("437")),
        ((203,144,81), ("436")),
        ((184,119,72), ("435")),
        ((152,94,51), ("434")),
        ((122,69,31), ("433")),
        ((101,57,25), ("801")),
        ((73,42,19), ("898")),
        ((54,31,14), ("938")),
        ((30,17,8), ("3371")),
        ((242,227,206), ("543")),
        ((203,182,156), ("3864")),
        ((164,131,92), ("3863")),
        ((138,110,78), ("3862")),
        ((75,60,42), ("3031")),
        ((255,255,255), ("B5200")),
        ((252,251,248), ("White")),
        ((249,247,241), ("3865")),
        ((240,234,218), ("Ecru")),
        ((231,226,211), ("822")),
        ((221,216,203), ("644")),
        ((164,152,120), ("642")),
        ((133,123,97), ("640")),
        ((98,93,80), ("3787")),
        ((79,75,65), ("3021")),
        ((235,234,231), ("3024")),
        ((177,170,151), ("3023")),
        ((142,144,120), ("3022")),
        ((99,100,88), ("535")),
        ((227,216,204), ("3033")),
        ((210,188,166), ("3782")),
        ((179,159,139), ("3032")),
        ((127,106,85), ("3790")),
        ((107,87,67), ("3781")),
        ((250,246,240), ("3866")),
        ((209,186,161), ("842")),
        ((182,155,126), ("841")),
        ((154,124,92), ("840")),
        ((103,85,65), ("839")),
        ((89,73,55), ("838")),
        ((230,232,232), ("3072")),
        ((188,180,172), ("648")),
        ((176,166,156), ("647")),
        ((135,125,115), ("646")),
        ((110,101,92), ("645")),
        ((72,72,72), ("844")),
        ((236,236,236), ("762")),
        ((211,211,214), ("415")),
        ((171,171,171), ("318")),
        ((140,140,140), ("414")),
        ((209,209,209), ("168")),
        ((132,132,132), ("169")),
        ((108,108,108), ("317")),
        ((86,86,86), ("413")),
        ((66,66,66), ("3799")),
        ((0,0,0), ("310")),
    ]).into_iter()
    .map(|((r, g, b), stitch_id)| (PixelRGBA {r: r, g: g, b: b, a: 255}, stitch_id))
    .collect();


    for image_filepath in get_image_filepaths_from_commandline() {
        create_image_output_dir(&image_filepath, "");
        create_image_output_dir(&image_filepath, "centered");
        create_image_output_dir(&image_filepath, "preview");

        let mut image = open_image(&image_filepath);
        image = convert_image(&image, &stitch_colors_mapping);
        let color_mappings = create_color_mappings_from_image(
            &image,
            &image_filepath,
            &symbols,
            &symbols_alphanum,
            &stitch_images_premultiplied_alpha,
            &stitch_images_luminance_premultiplied_alpha
        );

        rayon::scope(|scope| {
            scope.spawn(|_| {
                create_patterns_dir(&image, &image_filepath, &resources, &color_mappings, &stitch_colors_mapping);
            });
            scope.spawn(|_| {
                create_patterns_dir_centered(&image, &image_filepath, &resources, &color_mappings, &stitch_colors_mapping);
            });
            scope.spawn(|_| {
                create_preview_dir(&image, &image_filepath, &resources, &color_mappings);
            });
        });
    }

    // #[cfg(not(debug_assertions))]
    // show_messagebox("Pixie Stitch", "Finished creating patterns. Enjoy!", false);
}
