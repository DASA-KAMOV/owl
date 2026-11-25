// SPDX-License-Identifier: GPL-3.0-or-later

use rayon::prelude::*;

const LICENSE: &str = include_str!("../LICENSE");

const HEX_LOOKUP_TABLE: &[u8; 16] = &[
    0x30, 0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37, 0x38, 0x39, 0x41, 0x42, 0x43, 0x44, 0x45, 0x46,
];

// Use Font Book at https://www.egui.rs/#demo for supported icons
pub const ICON_CHECK: char = '\u{2714}'; // ✔
pub const _ICON_COFFEE: char = '\u{2615}'; // ☕
pub const ICON_CONTAINER: char = '\u{1F4E6}'; // 📦
pub const ICON_CRESCENT_MOON: char = '\u{1F319}'; // 🌙
pub const ICON_DOCUMENT: char = '\u{1F5B9}'; // 🖹
pub const ICON_EXIT: char = '\u{2386}'; // ⎆
pub const ICON_FLOPPY: char = '\u{1F4BE}'; // 💾
pub const ICON_FILE_CABINET: char = '\u{1F5C4}'; // 🗄
pub const ICON_FILM: char = '\u{1F39E}'; // 🎞
pub const ICON_FOLDER_CLOSED: char = '\u{1F5C0}'; // 🗀
pub const ICON_FOLDER_OPEN: char = '\u{1F5C1}'; // 🗁
pub const ICON_GEAR: char = '\u{26ED}'; // ⚙
pub const ICON_HALF_CIRCLE: char = '\u{25D1}'; // ◑
pub const ICON_IMAGE: char = '\u{1F5BC}'; // 🖼
pub const ICON_IMAGE_2: char = '\u{1F5BB}'; // 🖻
pub const ICON_INFO: char = '\u{2139}'; // ℹ
pub const ICON_LIGHT_BULB: char = '\u{1F4A1}'; // 💡
pub const ICON_MAP: char = '\u{1F5FA}'; // 🗺
pub const ICON_MODEL: char = '\u{1F310}'; // 🌐
pub const ICON_MUSIC: char = '\u{266A}'; // ♪
pub const ICON_PALETTE: char = '\u{1F3A8}'; // 🎨
pub const ICON_RECORDING: char = '\u{1F3A5}'; // 🎥
pub const ICON_ROCKET: char = '\u{1F680}'; // 🚀
pub const ICON_SCALE: char = '\u{1F5DB}'; // 🗛
pub const ICON_SCROLL: char = '\u{1F4C3}'; // 📃
pub const ICON_SPEAKER: char = '\u{1F50A}'; // 🔊
pub const ICON_TEXTURE: char = '\u{2593}'; // ▓
pub const ICON_UNKNOWN: char = '\u{2753}'; // ?
pub const ICON_WRENCH: char = '\u{1F527}'; // 🔧
pub const ICON_X: char = '\u{2716}'; // ✖

pub struct DataViewCache {
    hex_display: String,
    txt_display: String,
    int_display: String,
}

impl Default for DataViewCache {
    fn default() -> Self {
        Self {
            hex_display: "NO DATA".to_string(),
            txt_display: "NO DATA".to_string(),
            int_display: "NO DATA".to_string(),
        }
    }
}

impl DataViewCache {
    pub fn clear(&mut self) {
        self.hex_display.clear();
        self.txt_display.clear();
        self.int_display.clear();
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum IntDisplayFormat {
    U8,
    I8,
    U16,
    I16,
    U32,
    I32,
}

impl IntDisplayFormat {
    pub fn as_str(&self) -> &'static str {
        match self {
            IntDisplayFormat::U8 => "Unsigned 8-Bit (Default)",
            IntDisplayFormat::I8 => "Signed 8-Bit",
            IntDisplayFormat::U16 => "Unsigned 16-Bit",
            IntDisplayFormat::I16 => "Signed 16-Bit",
            IntDisplayFormat::U32 => "Unsigned 32-Bit",
            IntDisplayFormat::I32 => "Signed 32-Bit",
        }
    }
}

pub struct CoreGUI {
    pub data_view_cache: DataViewCache,
    pub file_picker: egui_file_dialog::FileDialog,
    pub file_saver: egui_file_dialog::FileDialog,
    pub int_display_type: IntDisplayFormat,
    pub max_line_width: i16,
    pub dark_mode: bool,
    show_about_popup: bool,
    show_license_popup: bool,
    ui_scaling: f32,
}

impl CoreGUI {
    pub fn new() -> Self {
        CoreGUI {
            data_view_cache: DataViewCache::default(),
            file_picker: egui_file_dialog::FileDialog::new()
                .title("Select File")
                .as_modal(true)
                .default_pos(egui::Pos2::new(10.0, 10.0))
                .add_file_filter(
                    "Psygnosis Asset Archive (*.rdt)",
                    egui_file_dialog::Filter::new(|path: &std::path::Path| {
                        path.extension()
                            .unwrap_or_default()
                            .eq_ignore_ascii_case("rdt")
                    }),
                )
                .add_file_filter(
                    "G-Police (PC) Save File (*.sav)",
                    egui_file_dialog::Filter::new(|path: &std::path::Path| {
                        path.extension()
                            .unwrap_or_default()
                            .eq_ignore_ascii_case("sav")
                    }),
                ),
            file_saver: egui_file_dialog::FileDialog::new()
                .title("Save File As...")
                .allow_path_edit_to_save_file_without_extension(true)
                .as_modal(true)
                .default_pos(egui::Pos2::new(10.0, 10.0)),
            int_display_type: IntDisplayFormat::U8,
            max_line_width: 0,
            dark_mode: true,
            show_about_popup: false,
            show_license_popup: false,
            ui_scaling: 1.0,
        }
    }

    pub fn apply_theme(&mut self, ctx: &egui::Context) {
        let theme = if self.dark_mode {
            egui::ThemePreference::Dark
        } else {
            egui::ThemePreference::Light
        };
        ctx.set_theme(theme);
    }

    pub fn menu_style(&self, ui: &egui::Ui) -> egui::Style {
        let mut style = ui.style().as_ref().clone();
        style.visuals = if self.dark_mode {
            egui::Visuals::dark()
        } else {
            egui::Visuals::light()
        };
        style
    }

    pub fn set_dark_mode(&mut self, ctx: &egui::Context, dark_mode: bool) {
        if self.dark_mode == dark_mode {
            return;
        }

        self.dark_mode = dark_mode;
        self.apply_theme(ctx);
        ctx.request_repaint();
    }

    pub fn display_file_menu(&mut self, ui: &mut egui::Ui, allow_save: bool) {
        ui.menu_button(
            format!("{} File", ICON_FOLDER_CLOSED),
            |ui: &mut egui::Ui| {
                if ui
                    .button(format!("{} Open File", ICON_FOLDER_OPEN))
                    .clicked()
                {
                    self.file_picker.pick_file();
                    ui.close();
                }
                ui.separator();
                if allow_save {
                    if ui.button(format!("{} Save File", ICON_FLOPPY)).clicked() {
                        self.file_saver.save_file();
                        ui.close();
                    }
                } else {
                    ui.label(format!("{} Save File", ICON_FLOPPY))
                        .on_hover_text("No file currently open to save.");
                }
                ui.separator();
                if ui.button(format!("{} Exit Owl", ICON_EXIT)).clicked() {
                    std::process::exit(0);
                }
            },
        );
        ui.separator();
    }

    pub fn display_about_menu(&mut self, ui: &mut egui::Ui) {
        ui.menu_button(format!("{} About", ICON_INFO), |ui| {
            if ui.button(format!("{} License", ICON_SCROLL)).clicked() {
                self.show_license_popup = true;
                ui.close();
            }
            if ui.button(format!("{} About Owl", ICON_INFO)).clicked() {
                self.show_about_popup = true;
                ui.close();
            }
        });
        ui.separator();
    }

    pub fn display_options_menu(&mut self, ui: &mut egui::Ui) {
        ui.menu_button(format!("{} Options", ICON_GEAR), |ui: &mut egui::Ui| {
            let dark_mode = self.dark_mode;
            let theme_button = if dark_mode {
                ui.button(format!("{} Switch to Light Mode", ICON_LIGHT_BULB))
            } else {
                ui.button(format!("{} Switch to Dark Mode", ICON_CRESCENT_MOON))
            };
            if theme_button.clicked() {
                self.set_dark_mode(ui.ctx(), !dark_mode);
                ui.close();
            }
            ui.menu_button(format!("{} Data View", ICON_WRENCH), |ui| {
                ui.menu_button("Maximum Hex Bytes Per Line", |ui| {
                    let mut response =
                        ui.selectable_value(&mut self.max_line_width, 0, "Auto (Default)");
                    response |= ui.selectable_value(&mut self.max_line_width, 4, "4 Bytes");
                    response |= ui.selectable_value(&mut self.max_line_width, 8, "8 Bytes");
                    response |= ui.selectable_value(&mut self.max_line_width, 16, "16 Bytes");
                    response |= ui.selectable_value(&mut self.max_line_width, 32, "32 Bytes");
                    response |= ui.selectable_value(&mut self.max_line_width, 64, "64 Bytes");
                    if response.clicked() {
                        self.reformat_data_view();
                        ui.close();
                    }
                });
                ui.menu_button("Integer Display Type", |ui| {
                    let int_types: [IntDisplayFormat; 6] = [
                        IntDisplayFormat::U8,
                        IntDisplayFormat::I8,
                        IntDisplayFormat::U16,
                        IntDisplayFormat::I16,
                        IntDisplayFormat::U32,
                        IntDisplayFormat::I32,
                    ];

                    for &int_type in &int_types {
                        if ui
                            .selectable_value(
                                &mut self.int_display_type,
                                int_type,
                                int_type.as_str(),
                            )
                            .clicked()
                        {
                            self.reformat_data_view();
                        }
                    }
                });
            });
            ui.menu_button(format!("{} UI Scale", ICON_SCALE), |ui| {
                let mut response = ui.selectable_value(&mut self.ui_scaling, 0.5, "0.5x");
                response |= ui.selectable_value(&mut self.ui_scaling, 0.75, "0.75x");
                response |= ui.selectable_value(&mut self.ui_scaling, 1.0, "1x (Default)");
                response |= ui.selectable_value(&mut self.ui_scaling, 1.25, "1.25x");
                response |= ui.selectable_value(&mut self.ui_scaling, 1.5, "1.5x");
                response |= ui.selectable_value(&mut self.ui_scaling, 1.75, "1.75x");
                response |= ui.selectable_value(&mut self.ui_scaling, 2.0, "2x");
                if response.clicked() {
                    ui.ctx().set_pixels_per_point(self.ui_scaling);
                    ui.close();
                }
            });
        });
        ui.separator();
    }

    pub fn display_file_metadata(&self, ui: &mut egui::Ui, file_info: &crate::PsyFileMetadata) {
        ui.strong("File:");
        ui.label(file_info.name.as_str());
        ui.strong("Path:");
        ui.label(file_info.path.display().to_string());
        ui.strong("Size (bytes):");
        ui.label(file_info.length.to_string());
        if let Some(created) = file_info.created {
            ui.strong("Created:");
            let label = jiff::Timestamp::try_from(created)
                .map(|ts| {
                    let dt = ts.to_zoned(jiff::tz::TimeZone::UTC).datetime();
                    format!(
                        "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
                        dt.year(),
                        dt.month(),
                        dt.day(),
                        dt.hour(),
                        dt.minute(),
                        dt.second()
                    )
                })
                .unwrap_or_else(|_| String::from("<invalid timestamp>"));
            ui.label(label);
        }
        if let Some(modified) = file_info.modified {
            ui.strong("Modified:");
            let label = jiff::Timestamp::try_from(modified)
                .map(|ts| {
                    let dt = ts.to_zoned(jiff::tz::TimeZone::UTC).datetime();
                    format!(
                        "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
                        dt.year(),
                        dt.month(),
                        dt.day(),
                        dt.hour(),
                        dt.minute(),
                        dt.second()
                    )
                })
                .unwrap_or_else(|_| String::from("<invalid timestamp>"));
            ui.label(label);
        }
    }
    pub fn display_data_view(&self, ui: &mut egui::Ui) {
        if self.data_view_cache.hex_display.is_empty()
            && self.data_view_cache.int_display.is_empty()
            && self.data_view_cache.txt_display.is_empty()
        {
            return;
        }
        ui.add_space(10.0);
        ui.separator();
        ui.strong("As Hexadecimal:")
            .on_hover_text("The number of bytes per line can be changed from the Options Menu.");
        ui.separator();
        egui::ScrollArea::vertical()
            .id_salt("hex_scroll")
            .min_scrolled_height(150.0)
            .max_height(300.0)
            .show(ui, |ui| {
                ui.set_max_width(ui.available_width());
                ui.add(
                    egui::Label::new(
                        egui::RichText::new(&self.data_view_cache.hex_display)
                            .font(egui::FontId::monospace(12.0)),
                    )
                    .wrap(),
                );
            });
        ui.add_space(12.0);
        ui.separator();
        ui.strong("As Text:")
            .on_hover_text("Non-printable characters are represented as ' . '");
        ui.separator();
        egui::ScrollArea::vertical()
            .id_salt("txt_scroll")
            .min_scrolled_height(150.0)
            .max_height(300.0)
            .show(ui, |ui| {
                ui.set_max_width(ui.available_width());
                ui.add(
                    egui::Label::new(
                        egui::RichText::new(&self.data_view_cache.txt_display)
                            .font(egui::FontId::monospace(12.0)),
                    )
                    .wrap(),
                );
            });
        ui.add_space(12.0);
        ui.separator();
        ui.strong(format!("As {}:", self.int_display_type.as_str()))
            .on_hover_text("The integer format can be changed from the Options Menu.");
        ui.separator();
        egui::ScrollArea::vertical()
            .id_salt("int_scroll")
            .min_scrolled_height(150.0)
            .max_height(300.0)
            .show(ui, |ui| {
                ui.set_max_width(ui.available_width());
                ui.add(
                    egui::Label::new(
                        egui::RichText::new(&self.data_view_cache.int_display)
                            .font(egui::FontId::monospace(12.0)),
                    )
                    .wrap(),
                );
            });
        ui.separator();
    }

    pub fn update_data_view(&mut self, data: &mut [u8]) {
        if data.is_empty() {
            self.data_view_cache.clear();
            return;
        }

        self.data_view_cache.clear();

        for (i, &byte) in data.iter().enumerate() {
            if i > 0 {
                if self.max_line_width > 0 && i % self.max_line_width as usize == 0 {
                    self.data_view_cache.hex_display.push('\n');
                } else {
                    self.data_view_cache.hex_display.push(' ');
                }
            }
            self.data_view_cache
                .hex_display
                .push(HEX_LOOKUP_TABLE[(byte >> 4) as usize] as char);
            self.data_view_cache
                .hex_display
                .push(HEX_LOOKUP_TABLE[(byte & 0x0F) as usize] as char);

            self.data_view_cache
                .txt_display
                .push(Self::printable_only_char(byte));
        }

        match self.int_display_type {
            IntDisplayFormat::U8 => {
                self.data_view_cache.int_display = data
                    .par_iter()
                    .map(|&byte| byte.to_string())
                    .collect::<Vec<String>>()
                    .join(" ");
            }
            IntDisplayFormat::I8 => {
                self.data_view_cache.int_display = data
                    .par_iter()
                    .map(|&byte| i8::from_le_bytes([byte]).to_string())
                    .collect::<Vec<String>>()
                    .join(" ");
            }
            IntDisplayFormat::U16 => {
                self.data_view_cache.int_display = data
                    .par_chunks_exact(2)
                    .map(|chunk| {
                        let value = u16::from_le_bytes([chunk[0], chunk[1]]);
                        value.to_string()
                    })
                    .collect::<Vec<String>>()
                    .join(" ");
            }
            IntDisplayFormat::I16 => {
                self.data_view_cache.int_display = data
                    .par_chunks_exact(2)
                    .map(|chunk| {
                        let value = i16::from_le_bytes([chunk[0], chunk[1]]);
                        value.to_string()
                    })
                    .collect::<Vec<String>>()
                    .join(" ");
            }
            IntDisplayFormat::U32 => {
                self.data_view_cache.int_display = data
                    .par_chunks_exact(4)
                    .map(|chunk| {
                        let value = u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
                        value.to_string()
                    })
                    .collect::<Vec<String>>()
                    .join(" ");
            }
            IntDisplayFormat::I32 => {
                self.data_view_cache.int_display = data
                    .par_chunks_exact(4)
                    .map(|chunk| {
                        let value = i32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
                        value.to_string()
                    })
                    .collect::<Vec<String>>()
                    .join(" ");
            }
        }
    }

    pub fn reformat_data_view(&mut self) {
        let hex_chars: String = self
            .data_view_cache
            .hex_display
            .chars()
            .filter(|c| c.is_ascii_hexdigit())
            .collect();

        self.data_view_cache.hex_display.clear();
        for (i, ch) in hex_chars.chars().enumerate() {
            if i > 0 {
                if self.max_line_width > 0 && i % (self.max_line_width as usize * 2) == 0 {
                    self.data_view_cache.hex_display.push('\n');
                } else if i % 2 == 0 {
                    self.data_view_cache.hex_display.push(' ');
                }
            }
            self.data_view_cache.hex_display.push(ch);
        }

        let bytes = Self::reconstruct_bytes(&self.data_view_cache.hex_display);
        self.data_view_cache.txt_display.clear();
        self.data_view_cache.txt_display = Self::printable_only_string(&bytes);

        self.data_view_cache.int_display.clear();
        match self.int_display_type {
            IntDisplayFormat::U8 => {
                self.data_view_cache.int_display = bytes
                    .par_iter()
                    .map(|&b| b.to_string())
                    .collect::<Vec<_>>()
                    .join(" ");
            }
            IntDisplayFormat::I8 => {
                self.data_view_cache.int_display = bytes
                    .par_iter()
                    .map(|&b| i8::from_le_bytes([b]).to_string())
                    .collect::<Vec<_>>()
                    .join(" ");
            }
            IntDisplayFormat::U16 => {
                self.data_view_cache.int_display = bytes
                    .par_chunks_exact(2)
                    .map(|c| u16::from_le_bytes([c[0], c[1]]).to_string())
                    .collect::<Vec<_>>()
                    .join(" ");
            }
            IntDisplayFormat::I16 => {
                self.data_view_cache.int_display = bytes
                    .par_chunks_exact(2)
                    .map(|c| i16::from_le_bytes([c[0], c[1]]).to_string())
                    .collect::<Vec<_>>()
                    .join(" ");
            }
            IntDisplayFormat::U32 => {
                self.data_view_cache.int_display = bytes
                    .par_chunks_exact(4)
                    .map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]]).to_string())
                    .collect::<Vec<_>>()
                    .join(" ");
            }
            IntDisplayFormat::I32 => {
                self.data_view_cache.int_display = bytes
                    .par_chunks_exact(4)
                    .map(|c| i32::from_le_bytes([c[0], c[1], c[2], c[3]]).to_string())
                    .collect::<Vec<_>>()
                    .join(" ");
            }
        }
    }

    fn hex_pair_to_byte(hex_pair: &str) -> Option<u8> {
        if hex_pair.len() == 2 {
            u8::from_str_radix(hex_pair, 16).ok()
        } else {
            None // Should never occur
        }
    }

    pub fn reconstruct_bytes(hex_display: &str) -> Vec<u8> {
        let hex_chars: String = hex_display
            .chars()
            .filter(|c| c.is_ascii_hexdigit())
            .collect();

        hex_chars
            .as_bytes()
            .par_chunks(2)
            .filter_map(|chunk| {
                Self::hex_pair_to_byte(std::str::from_utf8(chunk).unwrap_or_default())
            })
            .collect::<Vec<u8>>()
    }

    pub fn display_popups(&mut self, ui: &mut egui::Ui) {
        egui::Window::new(format!("{} License", ICON_SCROLL))
    .collapsible(false)
    .constrain(true)
    .default_pos(egui::Pos2::new(10.0, 10.0))
    .max_height(ui.min_size().y - 20.0)
    .max_width(ui.min_size().x - 20.0)
    .min_width(600.0)
    .open(&mut self.show_license_popup)
    .resizable(false)
    .show(ui.ctx(), |ui| {
        ui.label("Owl is free software.
        \nYou may redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, either Version 3 of the License, or (at your option) any later version.
        \nThis program is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
        \nOwl's author(s) will always make a good-faith effort to ensure that all of Owl's dependencies use licenses which are comaptible with the GNU General Public License Version 3. If you have a good-faith belief that one or more of Owl's dependencies are in violation of this principle, the author(s) strongly encourage you to contact them, so that appropriate action may be taken.
        \n\nSee the full GNU General Public License Version 3 text below.");
        ui.add_space(10.0);
        ui.separator();
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.with_layout(
                egui::Layout::centered_and_justified(egui::Direction::TopDown),
                  |ui| {
                     ui.add(
                        egui::TextEdit::multiline(&mut LICENSE.to_string())
                            .font(egui::TextStyle::Monospace)
                            .desired_rows(30)
                            .interactive(false)
                            .lock_focus(false)
                        );
                    },
                );
            });
        });

        egui::Window::new(format!("{} About Owl", ICON_INFO))
            .collapsible(false)
            .constrain(true)
            .default_pos(egui::Pos2::new(10.0, 10.0))
            .max_height(ui.min_size().y - 20.0)
            .max_width(ui.min_size().x - 20.0)
            .min_width(600.0)
            .open(&mut self.show_about_popup)
            .resizable(false)
            .show(ui.ctx(), |ui| {
                ui.strong("About Owl:");
                ui.label(env!("CARGO_PKG_DESCRIPTION"));
                ui.add_space(10.0);
                ui.strong("Owl Version:");
                ui.label(env!("CARGO_PKG_VERSION"));
                ui.add_space(10.0);
                ui.strong("Author(s):");
                ui.label(
                    env!("CARGO_PKG_AUTHORS")
                        .split(':')
                        .collect::<Vec<&str>>()
                        .join("\n"),
                );
                ui.add_space(10.0);
                ui.strong("Source Code");
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 0.0;
                    ui.label("Owl was built with Rust: ");
                    ui.hyperlink_to("https://www.rust-lang.org/", "https://www.rust-lang.org/");
                });
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 0.0;
                    ui.label("Owl's Source Code can be found at: ");
                    ui.hyperlink_to(env!("CARGO_PKG_REPOSITORY"), env!("CARGO_PKG_REPOSITORY"));
                });
                ui.add_space(10.0);
                ui.label("Contributions to Owl which improve, extend or optimise it are welcome.");
            });
    }

    pub fn printable_only_string(bytes: &[u8]) -> String {
        bytes
            .par_iter()
            .map(|&b| Self::printable_only_char(b))
            .collect()
    }

    pub fn printable_only_char(b: u8) -> char {
        let c = b as char;
        if c.is_control() || c.is_whitespace() || c == 0x2D as char || c == 0x00 as char {
            '.'
        } else {
            c
        }
    }
}

impl Default for CoreGUI {
    fn default() -> Self {
        Self::new()
    }
}

pub fn platinum(text: &str) -> egui::RichText {
    egui::RichText::new(text)
        .color(egui::Color32::from_rgb(172, 172, 255))
        .strong()
}

pub fn silver(text: &str) -> egui::RichText {
    egui::RichText::new(text)
        .color(egui::Color32::from_rgb(180, 180, 180))
        .strong()
}

pub fn gold(text: &str) -> egui::RichText {
    egui::RichText::new(text)
        .color(egui::Color32::from_rgb(225, 200, 0))
        .strong()
}

pub fn bronze(text: &str) -> egui::RichText {
    egui::RichText::new(text)
        .color(egui::Color32::from_rgb(205, 127, 50))
        .strong()
}
