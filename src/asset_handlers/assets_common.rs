// SPDX-License-Identifier: GPL-3.0-or-later

use rayon::prelude::*;

pub const ASSET_HEADER_SIZE: usize = 0x14;
pub const MAX_WAVEFORM_POINTS: usize = 4096;

pub struct AssetHeader {
    pub asset_name: String,
    pub asset_offset: usize,
    pub asset_size: usize,
    pub asset_type: AssetType,
    pub asset_extra_data_size: usize,
}

#[derive(PartialEq, Eq, Hash, Copy, Clone)]
pub enum AssetType {
    Ani,
    Arn,
    Atx,
    Audio,
    Bbx,
    Bmp,
    Col,
    Dat,
    Feb,
    Fxd,
    Fxh,
    Gob,
    Grd,
    Img,
    Lod,
    Mob,
    Mot,
    Nav,
    Pal,
    Rda,
    Rec,
    Riff,
    Sfxstr,
    Shp,
    Skl,
    Skn,
    Sph,
    Stb,
    Swp,
    Swr,
    Tex,
    Unknown,
    Vag,
    //Wai,
}

impl AssetType {
    pub fn from_i16(n: i16) -> AssetType {
        match n {
            0 => AssetType::Unknown,
            1 => AssetType::Rda,
            2 => AssetType::Stb,
            4 => AssetType::Dat, // Skn in TD, Dat in AR/AR Demo?
            5 => AssetType::Vag,
            7 => AssetType::Audio,  // Wav or Sfxstr in G-Police
            11 => AssetType::Audio, // Wai, Wav or Sfxstr in G-Police
            12 => AssetType::Shp,
            15 => AssetType::Vag,
            18 => AssetType::Tex,
            21 => AssetType::Pal,
            22 => AssetType::Bmp,
            30 => AssetType::Img,
            31 => AssetType::Shp,
            34 => AssetType::Tex,
            35 => AssetType::Vag,
            37 => AssetType::Pal,
            38 => AssetType::Bmp,
            43 => AssetType::Skn,
            44 => AssetType::Skl,
            64 => AssetType::Unknown, // Shipwreckers/Overboard
            77 => AssetType::Unknown, // Shipwreckers/Overboard
            66 => AssetType::Unknown, // Shipwreckers/Overboard
            84 => AssetType::Unknown, // Shipwreckers/Overboard
            108 => AssetType::Skl,
            119 => AssetType::Fxh,
            121 => AssetType::Fxd,
            131 => AssetType::Col,
            139 => AssetType::Bmp,
            141 => AssetType::Pal,
            143 => AssetType::Tex,
            145 => AssetType::Shp,
            200 => AssetType::Bmp,
            202 => AssetType::Pal,
            204 => AssetType::Tex,
            209 => AssetType::Img,
            225 => AssetType::Skl,
            229 => AssetType::Skn,
            426 => AssetType::Sph,
            690 => AssetType::Img,
            824 => AssetType::Shp,
            947 => AssetType::Col,
            961 => AssetType::Feb,
            1010 => AssetType::Mot,
            1119 => AssetType::Vag,
            1202 => AssetType::Fxh,
            1204 => AssetType::Fxd,
            1211 => AssetType::Vag,
            1214 => AssetType::Col,
            1414 => AssetType::Bmp,
            1416 => AssetType::Pal,
            1418 => AssetType::Tex,
            1423 => AssetType::Img, // G-Police
            1579 => AssetType::Skn,
            1580 => AssetType::Skl,
            1657 => AssetType::Atx,
            1758 => AssetType::Mob,
            2048 => AssetType::Shp,
            2154 => AssetType::Atx,
            2389 => AssetType::Arn,
            2503 => AssetType::Gob,
            2718 => AssetType::Grd,
            2808 => AssetType::Nav,
            3407 => AssetType::Arn,
            3692 => AssetType::Gob,
            4049 => AssetType::Nav,
            6386 => AssetType::Skl,
            6388 => AssetType::Skn,
            6937 => AssetType::Col,
            6944 => AssetType::Feb,
            6949 => AssetType::Mot,
            7549 => AssetType::Mob,
            7863 => AssetType::Grd,
            10016 => AssetType::Ani,
            10048 => AssetType::Swr,
            10068 => AssetType::Lod,
            10300 => AssetType::Bbx,
            10302 => AssetType::Swp,
            11442 => AssetType::Rec,
            _ => AssetType::Unknown,
        }
    }

    pub fn get_icon(t: &AssetType) -> char {
        match *t {
            AssetType::Ani => crate::gui_common::ICON_FILM,
            AssetType::Arn => crate::gui_common::ICON_UNKNOWN,
            AssetType::Atx => crate::gui_common::ICON_FILM,
            AssetType::Audio => crate::gui_common::ICON_MUSIC,
            AssetType::Bbx => crate::gui_common::ICON_UNKNOWN,
            AssetType::Bmp => crate::gui_common::ICON_IMAGE_2,
            AssetType::Col => crate::gui_common::ICON_UNKNOWN,
            AssetType::Dat => crate::gui_common::ICON_SCROLL,
            AssetType::Feb => crate::gui_common::ICON_UNKNOWN,
            AssetType::Fxd => crate::gui_common::ICON_UNKNOWN,
            AssetType::Fxh => crate::gui_common::ICON_UNKNOWN,
            AssetType::Gob => crate::gui_common::ICON_UNKNOWN,
            AssetType::Grd => crate::gui_common::ICON_UNKNOWN,
            AssetType::Img => crate::gui_common::ICON_IMAGE,
            AssetType::Lod => crate::gui_common::ICON_UNKNOWN,
            AssetType::Mob => crate::gui_common::ICON_UNKNOWN,
            AssetType::Mot => crate::gui_common::ICON_DOCUMENT,
            AssetType::Nav => crate::gui_common::ICON_MAP,
            AssetType::Pal => crate::gui_common::ICON_PALETTE,
            AssetType::Rda => crate::gui_common::ICON_CONTAINER,
            AssetType::Rec => crate::gui_common::ICON_RECORDING,
            AssetType::Riff => crate::gui_common::ICON_SPEAKER,
            AssetType::Sfxstr => crate::gui_common::ICON_SPEAKER,
            AssetType::Shp => crate::gui_common::ICON_ROCKET,
            AssetType::Skl => crate::gui_common::ICON_MODEL,
            AssetType::Skn => crate::gui_common::ICON_UNKNOWN,
            AssetType::Sph => crate::gui_common::ICON_UNKNOWN,
            AssetType::Stb => crate::gui_common::ICON_UNKNOWN,
            AssetType::Swp => crate::gui_common::ICON_UNKNOWN,
            AssetType::Swr => crate::gui_common::ICON_UNKNOWN,
            AssetType::Tex => crate::gui_common::ICON_TEXTURE,
            AssetType::Unknown => crate::gui_common::ICON_UNKNOWN,
            AssetType::Vag => crate::gui_common::ICON_MUSIC,
            //AssetType::Wai => crate::gui_common::ICON_MUSIC,
        }
    }
    pub fn get_name(t: &AssetType) -> &str {
        match *t {
            AssetType::Ani => "ANI",
            AssetType::Arn => "ARN",
            AssetType::Atx => "ATX",
            AssetType::Audio => "UNSPECIFIED AUDIO",
            AssetType::Bbx => "BBX",
            AssetType::Bmp => "BMP",
            AssetType::Col => "COL",
            AssetType::Dat => "DAT",
            AssetType::Feb => "FEB",
            AssetType::Fxd => "FXD",
            AssetType::Fxh => "FXH",
            AssetType::Gob => "GOB",
            AssetType::Grd => "GRD",
            AssetType::Img => "IMG",
            AssetType::Lod => "LOD",
            AssetType::Mob => "MOB",
            AssetType::Mot => "MOT",
            AssetType::Nav => "NAV",
            AssetType::Pal => "PAL",
            AssetType::Rda => "RDA",
            AssetType::Rec => "REC",
            AssetType::Riff => "RIFF AUDIO",
            AssetType::Sfxstr => "SFXSTR",
            AssetType::Shp => "SHP",
            AssetType::Skl => "SKL",
            AssetType::Skn => "SKN",
            AssetType::Sph => "SPH",
            AssetType::Stb => "STB",
            AssetType::Swp => "SWP",
            AssetType::Swr => "SWR",
            AssetType::Tex => "TEX",
            AssetType::Unknown => "UNKNOWN",
            AssetType::Vag => "VAG",
            //AssetType::Wai => "WAI",
        }
    }
}

pub trait AssetHandler {
    fn update(&mut self, ctx: &egui::Context);
    fn display_right_panel(&mut self, ui: &mut egui::Ui);
    fn display_centre_panel(&mut self, ui: &mut egui::Ui);
    fn data_view_dirty(&self) -> bool {
        false
    }
    fn clear_data_view_dirty(&mut self) {}
}

#[inline]
pub fn riff_to_waveform(data: &[u8], num_channels: i16, bits_per_sample: i16) -> Vec<[f64; 2]> {
    let bytes_per_sample = (bits_per_sample as usize / 8) * num_channels as usize;
    if bytes_per_sample == 0 || data.len() < bytes_per_sample {
        return (0..MAX_WAVEFORM_POINTS).map(|i| [i as f64, 0.0]).collect();
    }

    let total_samples = data.len() / bytes_per_sample;
    let step = if total_samples > MAX_WAVEFORM_POINTS {
        total_samples / MAX_WAVEFORM_POINTS
    } else {
        1
    };

    let out_len = total_samples.div_ceil(step);
    let scale_8 = 2.0 / 255.0;
    let scale_16 = 1.0 / i16::MAX as f64;
    let indices: Vec<usize> = (0..out_len).into_par_iter().map(|i| i * step).collect();

    indices
        .into_par_iter()
        .map(|sample_index| {
            let byte_index = sample_index * bytes_per_sample;

            let y = match (num_channels, bits_per_sample) {
                (1, 8) => {
                    let b = data[byte_index];
                    b as f64 * scale_8 - 1.0
                }
                (2, 8) => {
                    let b = data[byte_index];
                    b as f64 * scale_8 - 1.0
                }
                (1, 16) => {
                    let s = i16::from_le_bytes([data[byte_index], data[byte_index + 1]]);
                    s as f64 * scale_16
                }
                (2, 16) => {
                    let s = i16::from_le_bytes([data[byte_index], data[byte_index + 1]]);
                    s as f64 * scale_16
                }
                _ => 0.0,
            };

            [sample_index as f64 / step as f64, y]
        })
        .collect()
}

pub fn create_checkerboard_texture(width: usize, height: usize, scale: usize) -> egui::ColorImage {
    let mut pixels = vec![egui::Color32::BLACK; width * height];

    pixels.par_iter_mut().enumerate().for_each(|(i, pixel)| {
        let y = i / width;
        let x = i % width;

        let block_x = x / scale;
        let block_y = y / scale;

        if (block_x + block_y).is_multiple_of(2) {
            *pixel = egui::Color32::WHITE;
        }
    });

    egui::ColorImage::new([width, height], pixels)
}
