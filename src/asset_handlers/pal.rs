// SPDX-License-Identifier: GPL-3.0-or-later

use egui::ColorImage;

#[derive(Clone, Copy)]
struct PaletteEntry {
    offset: usize,
    red: u8,
    green: u8,
    blue: u8,
}

pub struct PALHandler {
    file: std::rc::Rc<std::cell::RefCell<crate::PsyFile>>,
    offset: usize,
    length: usize,
    scale: f32,
    texture_handle: std::cell::RefCell<Option<egui::TextureHandle>>,
    decoded_image: std::cell::RefCell<Option<ColorImage>>,
    preview_texture_handle: std::cell::RefCell<Option<egui::TextureHandle>>,
    preview_image: std::cell::RefCell<Option<ColorImage>>,
    palette: std::cell::RefCell<Vec<PaletteEntry>>,
    data_view_dirty: std::cell::Cell<bool>,
}

impl PALHandler {
    pub fn new(
        file: std::rc::Rc<std::cell::RefCell<crate::PsyFile>>,
        offset: usize,
        length: usize,
    ) -> Self {
        let handler = Self {
            file,
            offset,
            length,
            scale: 1.0,
            texture_handle: std::cell::RefCell::new(None),
            decoded_image: std::cell::RefCell::new(None),
            preview_texture_handle: std::cell::RefCell::new(None),
            preview_image: std::cell::RefCell::new(None),
            palette: std::cell::RefCell::new(Vec::new()),
            data_view_dirty: std::cell::Cell::new(false),
        };
        handler.reload_palette();
        handler
    }

    fn reload_palette(&self) {
        let buffer = self.file.borrow().buffer.clone();
        let start = self.offset.min(buffer.len());
        let end = (self.offset + self.length).min(buffer.len());
        let slice = &buffer[start..end];

        let mut palette = Vec::new();
        if slice.len() >= 5 {
            // Some files have non-palette 01 00 patterns before the real palette header.
            // Validate each candidate header before accepting it.
            let mut palette_base = None;
            for rel_pos in 0..=slice.len().saturating_sub(2) {
                if slice[rel_pos] != 1 || slice[rel_pos + 1] != 0 {
                    continue;
                }

                let data = &slice[rel_pos..];
                if data.len() < 5 {
                    continue;
                }

                // The palette count is stored as an 8-bit value. A zero value means the
                // real count is encoded as a 16-bit little-endian word immediately after it.
                let size_check = data[2];
                let (num_colors, colors_offset) = if size_check == 0 {
                    let num = data
                        .get(3..5)
                        .map(|b| u16::from_le_bytes([b[0], b[1]]) as usize)
                        .unwrap_or(0);
                    (num, 5usize)
                } else {
                    let num = size_check as usize;
                    (num, 4usize)
                };

                if num_colors == 0 {
                    continue;
                }

                let required_len = colors_offset + num_colors * 3;
                if required_len > data.len() {
                    continue;
                }

                palette_base = Some(start + rel_pos);
                break;
            }

            if let Some(palette_base) = palette_base {
                let data = &buffer[palette_base..end];
                let size_check = data.get(2).copied().unwrap_or(0);
                let (num_colors, colors_offset) = if size_check == 0 {
                    let num = data
                        .get(3..5)
                        .map(|b| u16::from_le_bytes([b[0], b[1]]) as usize)
                        .unwrap_or(0);
                    (num, 5usize)
                } else {
                    let num = size_check as usize;
                    (num, 4usize)
                };

                let max_colors = (data.len().saturating_sub(colors_offset)) / 3;
                let count = num_colors.min(max_colors);
                for index in 0..count {
                    let byte_offset = palette_base + colors_offset + index * 3;
                    let r = buffer.get(byte_offset).copied().unwrap_or(0);
                    let g = buffer.get(byte_offset + 1).copied().unwrap_or(0);
                    let b = buffer.get(byte_offset + 2).copied().unwrap_or(0);
                    palette.push(PaletteEntry {
                        offset: byte_offset,
                        red: r,
                        green: g,
                        blue: b,
                    });
                }
            }
        }

        *self.palette.borrow_mut() = palette;
        *self.decoded_image.borrow_mut() = None;
        *self.texture_handle.borrow_mut() = None;
        *self.preview_image.borrow_mut() = None;
        *self.preview_texture_handle.borrow_mut() = None;
    }

    fn palette_to_color_image(&self) -> ColorImage {
        let colors = self.palette.borrow();
        if colors.is_empty() {
            return crate::asset_handlers::assets_common::create_checkerboard_texture(32, 32, 6);
        }

        let swatch_size = 20usize;
        let width = colors.len() * swatch_size;
        let height = swatch_size;
        let mut pixels = vec![0u8; width * height * 4];

        for (slot_index, entry) in colors.iter().enumerate() {
            let x_start = slot_index * swatch_size;
            let color = egui::Color32::from_rgb(entry.red, entry.green, entry.blue);
            for y in 0..swatch_size {
                for x in 0..swatch_size {
                    let idx = ((y * width) + (x_start + x)) * 4;
                    pixels[idx] = color.r();
                    pixels[idx + 1] = color.g();
                    pixels[idx + 2] = color.b();
                    pixels[idx + 3] = 255;
                }
            }
        }

        ColorImage::from_rgba_unmultiplied([width, height], &pixels)
    }

    fn ensure_texture(&self, ctx: &egui::Context) -> egui::TextureHandle {
        if let Some(handle) = &*self.preview_texture_handle.borrow() {
            return handle.clone();
        }

        let image = {
            let cached = self.preview_image.borrow().clone();
            if let Some(image) = cached {
                image
            } else {
                let image = self.palette_to_color_image();
                *self.preview_image.borrow_mut() = Some(image.clone());
                image
            }
        };

        let key = format!("pal_preview_{:X}_{:X}", self.offset, self.length);
        let handle = ctx.load_texture(key, image, egui::TextureOptions::NEAREST);
        *self.preview_texture_handle.borrow_mut() = Some(handle.clone());
        handle
    }
}

impl super::assets_common::AssetHandler for PALHandler {
    fn update(&mut self, _ctx: &egui::Context) {}
    fn display_right_panel(&mut self, ui: &mut egui::Ui) {
        ui.separator();
        ui.heading("PAL Asset");
        ui.label("Palette assets define an ordered list of RGB colours. Owl displays the palette as a horizontal strip of swatches and lets each entry be edited in-place.");
    }
    fn data_view_dirty(&self) -> bool {
        self.data_view_dirty.get()
    }

    fn clear_data_view_dirty(&mut self) {
        self.data_view_dirty.set(false);
    }

    fn display_centre_panel(&mut self, ui: &mut egui::Ui) {
        if self.palette.borrow().is_empty() {
            self.reload_palette();
        }

        let preview_tex = self.ensure_texture(ui.ctx());

        ui.add_space(5.0);
        let max_width = ui
            .available_width()
            .min(preview_tex.size_vec2().x * self.scale);
        let size = egui::vec2(max_width.max(32.0), preview_tex.size_vec2().y * self.scale);
        ui.horizontal_wrapped(|ui| {
            ui.set_max_width(ui.available_width());
            ui.add(
                egui::Image::new(&preview_tex)
                    .maintain_aspect_ratio(false)
                    .fit_to_exact_size(size),
            );
        });
        ui.add_space(10.0);

        egui::ScrollArea::both().show(ui, |ui| {
            ui.set_max_width(ui.available_width());
            let table_id = format!("pal_table_{:X}", self.offset);
            egui::Grid::new(table_id)
                .striped(true)
                .min_col_width(70.0)
                .show(ui, |ui| {
                    ui.strong("Offset");
                    ui.strong("Red");
                    ui.strong("Green");
                    ui.strong("Blue");
                    ui.strong("Colour");
                    ui.end_row();

                    let mut palette = self.palette.borrow_mut();
                    for entry in palette.iter_mut() {
                        let mut display_red = entry.red as i32;
                        let mut display_green = entry.green as i32;
                        let mut display_blue = entry.blue as i32;

                        ui.label(format!("{:#X}", entry.offset));

                        let response = ui.add(egui::DragValue::new(&mut display_red).speed(1.0));
                        display_red = display_red.clamp(0, 255);
                        if response.changed() {
                            entry.red = display_red as u8;
                            self.file.borrow_mut().buffer[entry.offset] = entry.red;
                            *self.decoded_image.borrow_mut() = None;
                            *self.texture_handle.borrow_mut() = None;
                            *self.preview_image.borrow_mut() = None;
                            *self.preview_texture_handle.borrow_mut() = None;
                            self.data_view_dirty.set(true);
                        }

                        let response = ui.add(egui::DragValue::new(&mut display_green).speed(1.0));
                        display_green = display_green.clamp(0, 255);
                        if response.changed() {
                            entry.green = display_green as u8;
                            self.file.borrow_mut().buffer[entry.offset + 1] = entry.green;
                            *self.decoded_image.borrow_mut() = None;
                            *self.texture_handle.borrow_mut() = None;
                            *self.preview_image.borrow_mut() = None;
                            *self.preview_texture_handle.borrow_mut() = None;
                            self.data_view_dirty.set(true);
                        }

                        let response = ui.add(egui::DragValue::new(&mut display_blue).speed(1.0));
                        display_blue = display_blue.clamp(0, 255);
                        if response.changed() {
                            entry.blue = display_blue as u8;
                            self.file.borrow_mut().buffer[entry.offset + 2] = entry.blue;
                            *self.decoded_image.borrow_mut() = None;
                            *self.texture_handle.borrow_mut() = None;
                            *self.preview_image.borrow_mut() = None;
                            *self.preview_texture_handle.borrow_mut() = None;
                            self.data_view_dirty.set(true);
                        }

                        let color = egui::Color32::from_rgb(entry.red, entry.green, entry.blue);
                        let (rect, _) =
                            ui.allocate_exact_size(egui::vec2(32.0, 18.0), egui::Sense::hover());
                        ui.painter().rect_filled(rect, 2.0, color);
                        ui.painter().rect_stroke(
                            rect,
                            2.0,
                            egui::Stroke::new(
                                1.0,
                                ui.visuals().widgets.noninteractive.fg_stroke.color,
                            ),
                            egui::StrokeKind::Inside,
                        );
                        ui.end_row();
                    }
                });
        });
    }
}

/*
typedef struct {
    byte Red;
    byte Green;
    byte Blue;
} RGBTriplet;

typedef struct {
    int16 magic;                // Always 01 00
    local uint8 sizeCheck;
    sizeCheck = ReadByte();
    if (sizeCheck <= 0) {
        int16 numColours;
        RGBTriplet RGB[numColours];
    }
    else {
        uint8 numColours;
        uint8 unknown; // Usually either 128 or 256, not sure what significance this has
        RGBTriplet RGB[numColours];
        }
} PALData;
*/
