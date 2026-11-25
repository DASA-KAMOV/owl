// SPDX-License-Identifier: GPL-3.0-or-later

use rayon::prelude::*;

pub struct IMGHandler {
    file: std::rc::Rc<std::cell::RefCell<crate::PsyFile>>,
    offset: usize,
    length: usize,
    scale: f32,
    texture_handle: std::cell::RefCell<Option<egui::TextureHandle>>,
    decoded_image: std::cell::RefCell<Option<egui::ColorImage>>,
}

impl super::assets_common::AssetHandler for IMGHandler {
    fn update(&mut self, _ctx: &egui::Context) {}
    fn display_right_panel(&mut self, ui: &mut egui::Ui) {
        ui.separator();
        ui.heading("IMG Asset");
        ui.label("IMG assets are images with no headers other than the standard Psygnosis asset header.
        \nProperties such as image dimensions are therefore not specified anywhere within the asset itself, the game using the asset (and Owl) use manually hardcoded values.
        \nWhere Owl does not know the image dimensions in advance, it assumes the image is square. If the image data does not fit into a square, Owl displays a placeholder.");
    }

    fn display_centre_panel(&mut self, ui: &mut egui::Ui) {
        ui.add_space(5.0);
        let tex = self.ensure_texture(ui.ctx());
        let size = tex.size_vec2() * self.scale;
        egui::Frame::new()
            .fill(egui::Color32::BLACK)
            .stroke(egui::Stroke::new(1.0, ui.visuals().code_bg_color))
            .show(ui, |ui| {
                ui.add(
                    egui::Image::new(&tex)
                        .maintain_aspect_ratio(true)
                        .fit_to_exact_size(size),
                );
            });
        ui.add_space(5.0);
        ui.horizontal(|ui| {
            ui.label("Scale:");
            ui.add(
                egui::Slider::new(&mut self.scale, 0.25..=4.0)
                    .logarithmic(true)
                    .fixed_decimals(1)
                    .update_while_editing(false),
            );
        });
    }
}

impl IMGHandler {
    pub fn new(
        file: std::rc::Rc<std::cell::RefCell<crate::PsyFile>>,
        offset: usize,
        length: usize,
    ) -> Self {
        Self {
            file,
            offset,
            length,
            scale: 1.0,
            texture_handle: std::cell::RefCell::new(None),
            decoded_image: std::cell::RefCell::new(None),
        }
    }

    pub fn ensure_texture(&self, ctx: &egui::Context) -> egui::TextureHandle {
        if let Some(handle) = &*self.texture_handle.borrow() {
            return handle.clone();
        }

        let color_image = {
            let cached = self.decoded_image.borrow().clone();
            if let Some(color_image) = cached {
                color_image
            } else {
                let decoded = {
                    let buffer = &self.file.borrow().buffer;

                    if self.offset >= buffer.len() {
                        crate::asset_handlers::assets_common::create_checkerboard_texture(
                            250, 250, 25,
                        )
                    } else {
                        let end = (self.offset + self.length).min(buffer.len());
                        let data = &buffer[self.offset..end];

                        if !data.len().is_multiple_of(2) {
                            eprintln!("Image data length not divisible by 2");
                            crate::asset_handlers::assets_common::create_checkerboard_texture(
                                250, 250, 25,
                            )
                        } else {
                            let pixels = data.len() / 2;
                            if pixels == 640 * 480 {
                                Self::rgbx5551_to_color_image(data, 640, 480)
                            } else {
                                let side = (pixels as f64).sqrt() as usize;
                                let side = side.max(1);
                                Self::rgbx5551_to_color_image(data, side, side)
                            }
                        }
                    }
                };
                *self.decoded_image.borrow_mut() = Some(decoded.clone());
                decoded
            }
        };

        let key = format!("img_cached_{:X}_{:X}", self.offset, self.length);
        let handle = ctx.load_texture(key, color_image, egui::TextureOptions::NEAREST);
        *self.texture_handle.borrow_mut() = Some(handle.clone());
        handle
    }

    pub fn rgbx5551_to_color_image(
        asset_data: &[u8],
        width: usize,
        height: usize,
    ) -> egui::ColorImage {
        let num_pixels = width * height;
        let mut rgba_buffer = vec![0u8; num_pixels * 4];

        asset_data
            .par_chunks(2)
            .zip(rgba_buffer.par_chunks_mut(4))
            .for_each(|(src, dst)| {
                let word = u16::from_le_bytes(src[0..2].try_into().unwrap());

                let r5 = (word & 0x1F) as u8;
                let g5 = ((word >> 5) & 0x1F) as u8;
                let b5 = ((word >> 10) & 0x1F) as u8;
                let a1 = ((word >> 15) & 0x01) as u8;

                let r8 = (r5 << 3) | (r5 >> 2);
                let g8 = (g5 << 3) | (g5 >> 2);
                let b8 = (b5 << 3) | (b5 >> 2);
                let a8 = ((!a1) & 1) * 255;

                dst[0] = r8;
                dst[1] = g8;
                dst[2] = b8;
                dst[3] = a8;
            });

        egui::ColorImage::from_rgba_unmultiplied([width, height], &rgba_buffer)
    }
}
