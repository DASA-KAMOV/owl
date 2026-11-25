// SPDX-License-Identifier: GPL-3.0-or-later

use rayon::prelude::*;

pub struct RDTHandler {
    assets: Vec<crate::asset_handlers::assets_common::AssetHeader>,
    asset_changed: bool,
    asset_counts: std::collections::HashMap<crate::asset_handlers::assets_common::AssetType, usize>,
    current_asset_handler: Option<Box<dyn crate::asset_handlers::assets_common::AssetHandler>>,
    export_data: Vec<u8>,
    export_dialog: egui_file_dialog::FileDialog,
    file: std::rc::Rc<std::cell::RefCell<crate::PsyFile>>,
    root_label: String,
    selected_asset: Option<usize>,
    found_asset_types: Vec<crate::asset_handlers::assets_common::AssetType>,
    asset_type_filter: Vec<crate::asset_handlers::assets_common::AssetType>,
    filter_text: String,
}

impl crate::PsyFileHandler for RDTHandler {
    fn update(&mut self, gui: &mut crate::gui_common::CoreGUI, ctx: &egui::Context) {
        self.export_dialog.update(ctx);
        if let Some(path) = self.export_dialog.take_picked() {
            if !&self.export_data.is_empty() {
                match std::fs::File::create(&path) {
                    Ok(mut output) => {
                        if let Err(error) =
                            std::io::Write::write_all(&mut output, &self.export_data)
                        {
                            eprintln!("Failed to write {:?}: {}", path, error);
                        }
                    }
                    Err(error) => {
                        eprintln!("Failed to create {:?}: {}", path, error);
                    }
                }
            }
            self.export_data.clear();
        }

        if !self.asset_changed {
            let data_view_dirty = if let Some(asset_handler) = self.current_asset_handler.as_mut() {
                asset_handler.update(ctx);
                asset_handler.data_view_dirty()
            } else {
                false
            };

            if data_view_dirty {
                if let Some(mut asset) = self.get_asset_data(true) {
                    gui.update_data_view(&mut asset);
                }
                if let Some(asset_handler) = self.current_asset_handler.as_mut() {
                    asset_handler.clear_data_view_dirty();
                }
            }
            return;
        }

        self.current_asset_handler = None;
        if self
            .selected_asset
            .is_some_and(|id| id != usize::MAX && id < self.assets.len())
        {
            if let Some(mut asset) = self.get_asset_data(true) {
                gui.update_data_view(&mut asset);
            } else {
                gui.update_data_view(&mut []);
            }
        } else {
            gui.update_data_view(&mut []);
        }
        self.asset_changed = false;
    }

    fn display_left_panel(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("Find:");
            ui.text_edit_singleline(&mut self.filter_text);
        });

        ui.add_space(1.0);

        ui.horizontal(|ui| {
            ui.label("Type:");

            let selected_text = if self.asset_type_filter.is_empty() {
                "ALL".to_owned()
            } else {
                let mut s = String::new();
                for (idx, t) in self.asset_type_filter.iter().enumerate() {
                    if idx > 0 {
                        s.push_str(", ");
                    }
                    s.push(crate::asset_handlers::assets_common::AssetType::get_icon(t));
                    s.push(' ');
                    s.push_str(crate::asset_handlers::assets_common::AssetType::get_name(t));
                }
                s
            };

            egui::ComboBox::from_id_salt("asset_type_filter")
                .wrap_mode(egui::TextWrapMode::Wrap)
                .selected_text(selected_text)
                .show_ui(ui, |ui| {
                    if ui
                        .selectable_label(self.asset_type_filter.is_empty(), "ALL / CLEAR FILTER")
                        .clicked()
                    {
                        self.asset_type_filter.clear();
                    }

                    for &t in &self.found_asset_types {
                        let selected = self.asset_type_filter.contains(&t);

                        let mut label = String::new();
                        label.push(crate::asset_handlers::assets_common::AssetType::get_icon(
                            &t,
                        ));
                        label.push(' ');
                        label.push_str(crate::asset_handlers::assets_common::AssetType::get_name(
                            &t,
                        ));

                        if ui.selectable_label(selected, label).clicked() {
                            if selected {
                                self.asset_type_filter.retain(|x| *x != t);
                            } else {
                                self.asset_type_filter.push(t);
                            }
                        }
                    }
                });
        });

        let filter_text = self.filter_text.to_lowercase();
        let do_text_filter = !filter_text.is_empty();
        let type_filter = &self.asset_type_filter;

        let filtered_indices: Vec<usize> = self
            .assets
            .iter()
            .enumerate()
            .filter_map(|(i, asset)| {
                if !type_filter.is_empty() && !type_filter.contains(&asset.asset_type) {
                    return None;
                }

                if do_text_filter {
                    let name_lower = asset.asset_name.to_lowercase();
                    if !name_lower.contains(&filter_text) {
                        return None;
                    }
                }

                Some(i)
            })
            .collect();

        ui.add_space(2.0);

        let row_height = ui
            .text_style_height(&egui::TextStyle::Body)
            .max(ui.spacing().interact_size.y);
        let clicked_index = egui::ScrollArea::vertical()
            .id_salt("rdt_left_panel")
            .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysVisible)
            .show_rows(ui, row_height, filtered_indices.len(), |ui, row_range| {
                let mut current_selection = self.selected_asset.unwrap_or(usize::MAX);
                let mut clicked_index = None;

                for row in row_range {
                    let asset_index = filtered_indices[row];
                    let asset = &self.assets[asset_index];
                    let display_name = if do_text_filter {
                        Self::highlight_text(&asset.asset_name, &filter_text)
                    } else {
                        asset.asset_name.clone()
                    };

                    ui.set_min_width(ui.available_width());
                    let response =
                        ui.selectable_value(&mut current_selection, asset_index, display_name);
                    if response.clicked() {
                        clicked_index = Some(asset_index);
                    }
                }

                clicked_index
            })
            .inner;

        if let Some(index) = clicked_index {
            if self.selected_asset != Some(index) {
                self.selected_asset = Some(index);
                self.asset_changed = true;
            }
        }
    }

    fn display_right_panel(&mut self, ui: &mut egui::Ui) {
        if let Some(asset_handler) = self.current_asset_handler.as_mut() {
            asset_handler.display_right_panel(ui);
            if self
                .selected_asset
                .is_some_and(|id| id != usize::MAX && id < self.assets.len())
            {
                ui.separator();
                ui.heading("Export Asset");
                ui.add_space(5.0);
                if ui
                    .add_sized(
                        [150.0, 40.0],
                        egui::Button::new("Export Current Asset WITH Asset Header"),
                    )
                    .clicked()
                {
                    let maybe_data = self
                        .get_asset_data(true)
                        .map(|asset_data| asset_data.to_vec());

                    if let Some(data) = maybe_data {
                        self.export_asset(data);
                    }
                };
                if ui
                    .add_sized(
                        [150.0, 40.0],
                        egui::Button::new("Export Current Asset WITHOUT Asset Header"),
                    )
                    .clicked()
                {
                    let maybe_data = self
                        .get_asset_data(false)
                        .map(|asset_data| asset_data.to_vec());

                    if let Some(data) = maybe_data {
                        self.export_asset(data);
                    }
                };
            }
        }
    }

    fn display_centre_panel(&mut self, ui: &mut egui::Ui) {
        if self.selected_asset.is_none_or(|i| i >= self.assets.len()) {
            ui.vertical_centered(|ui| {
                ui.heading(self.root_label.as_str());
                ui.add_space(5.0);
                ui.separator();
            });
            ui.strong("Total Number of Assets:");
            ui.label(format!("{}", self.assets.len()));
            ui.add_space(10.0);
            egui::Grid::new("asset_counts_grid")
                .num_columns(2)
                .striped(true)
                .min_col_width(120.0)
                .show(ui, |ui| {
                    ui.strong("Asset Type");
                    ui.strong("Count");
                    ui.end_row();

                    for t in &self.found_asset_types {
                        let count = self.asset_counts.get(t).copied().unwrap_or(0);
                        ui.label(format!(
                            "{} {}",
                            crate::asset_handlers::assets_common::AssetType::get_icon(t),
                            crate::asset_handlers::assets_common::AssetType::get_name(t)
                        ));
                        ui.label(count.to_string());
                        ui.end_row();
                    }
                });
            return;
        }

        if let Some(asset_data) = self.get_asset_data(true) {
            let index = self.selected_asset.unwrap();
            let header = &self.assets[index];
            let extra_count = header.asset_extra_data_size / 4;

            ui.vertical_centered(|ui| {
                ui.add_space(5.0);
                ui.heading(format!("Asset at Offset {:#X}", header.asset_offset));
                ui.add_space(5.0);
                ui.separator();
            });

            ui.add_space(5.0);
            ui.horizontal_top(|ui| {
                egui::Grid::new("header_data_grid")
                    .num_columns(2)
                    .striped(true)
                    .min_col_width(150.0)
                    .show(ui, |ui| {
                        ui.strong("Asset Header Data");
                        ui.end_row();

                        ui.label("Index");
                        let index_val = i32::from_le_bytes(asset_data[0..4].try_into().unwrap());

                        ui.label(format!("{:#X}", index_val));
                        ui.end_row();

                        ui.label("Size (bytes)");
                        ui.label(header.asset_size.to_string());
                        ui.end_row();

                        ui.label("Type");
                        ui.label(crate::asset_handlers::assets_common::AssetType::get_name(
                            &header.asset_type,
                        ));
                        ui.end_row();

                        ui.label("Extra Data Count");
                        ui.label(extra_count.to_string());
                        ui.end_row();
                    });

                if extra_count > 0 {
                    ui.add_space(10.0);
                    egui::Grid::new("header_extra_data_grid")
                        .num_columns(2)
                        .striped(true)
                        .min_col_width(150.0)
                        .show(ui, |ui| {
                            ui.strong("Extra Data");
                            ui.end_row();

                            for i in 0..extra_count {
                                ui.label((i + 1).to_string());

                                let start = crate::asset_handlers::assets_common::ASSET_HEADER_SIZE
                                    + (i * 4);
                                let end = start + 4;

                                ui.label(format!(
                                    "{:#X}",
                                    i32::from_le_bytes(asset_data[start..end].try_into().unwrap())
                                ));
                                ui.end_row();
                            }
                        });
                }
            });

            ui.add_space(10.0);
            ui.separator();
        }

        if let Some(h) = self.current_asset_handler.as_mut() {
            h.display_centre_panel(ui);
        } else if let Some(index) = self.selected_asset {
            if let Some(asset) = self.assets.get(index) {
                self.current_asset_handler = Some(self.create_asset_handler(asset));
            }
        }
    }
}

impl RDTHandler {
    pub fn new(file: std::rc::Rc<std::cell::RefCell<crate::PsyFile>>) -> Self {
        let assets = Self::parse_assets(&file.borrow().buffer);
        let mut asset_counts = std::collections::HashMap::new();
        for header in &assets {
            *asset_counts.entry(header.asset_type).or_insert(0) += 1;
        }

        let mut sorted_asset_types: Vec<_> = asset_counts.keys().copied().collect();
        sorted_asset_types.sort_by(|a, b| {
            crate::asset_handlers::assets_common::AssetType::get_name(a)
                .cmp(crate::asset_handlers::assets_common::AssetType::get_name(b))
        });

        let root_label = format!(
            "{} {}",
            crate::gui_common::ICON_FILE_CABINET,
            file.borrow().metadata.name
        );
        Self {
            assets,
            asset_changed: true,
            asset_counts,
            current_asset_handler: None,
            export_data: Vec::new(),
            export_dialog: egui_file_dialog::FileDialog::new()
                .allow_path_edit_to_save_file_without_extension(true)
                .as_modal(true)
                .default_pos(egui::Pos2::new(10.0, 10.0))
                .title("Export Current Asset...")
                .allow_file_overwrite(true)
                .default_file_name("Untitled"),
            file,
            root_label,
            selected_asset: None,
            found_asset_types: sorted_asset_types,
            asset_type_filter: Vec::new(),
            filter_text: "".to_string(),
        }
    }

    pub fn parse_assets(data: &[u8]) -> Vec<crate::asset_handlers::assets_common::AssetHeader> {
        let mut minimal = Vec::new();
        let mut asset_offset = 0;

        while asset_offset < data.len() {
            while asset_offset < data.len()
                && (data[asset_offset] == 0x00 || data[asset_offset] == 0x55)
            {
                asset_offset += 1;
            }
            if asset_offset + crate::asset_handlers::assets_common::ASSET_HEADER_SIZE > data.len() {
                break;
            }

            let extra_size = (u16::from_le_bytes([data[asset_offset + 16], data[asset_offset + 17]])
                as usize)
                * 4;

            let asset_size = u32::from_le_bytes([
                data[asset_offset + 4],
                data[asset_offset + 5],
                data[asset_offset + 6],
                data[asset_offset + 7],
            ]) as usize;

            let total_len =
                crate::asset_handlers::assets_common::ASSET_HEADER_SIZE + extra_size + asset_size;
            if asset_offset + total_len > data.len() {
                break;
            }

            minimal.push((asset_offset, asset_size, extra_size, total_len));
            asset_offset += total_len;
        }

        minimal
            .into_par_iter()
            .map(|(asset_offset, asset_size, extra_size, _total_len)| {
                let mut asset_type = crate::asset_handlers::assets_common::AssetType::from_i16(
                    i16::from_le_bytes([data[asset_offset + 14], data[asset_offset + 15]]),
                );

                if asset_type == crate::asset_handlers::assets_common::AssetType::Audio
                    && asset_size > 4
                    && &data[asset_offset + crate::asset_handlers::assets_common::ASSET_HEADER_SIZE
                        ..asset_offset
                            + crate::asset_handlers::assets_common::ASSET_HEADER_SIZE
                            + 4]
                        == b"RIFF"
                {
                    asset_type = crate::asset_handlers::assets_common::AssetType::Riff;
                }
                if asset_type == crate::asset_handlers::assets_common::AssetType::Audio
                    && asset_size > 6
                    && &data[asset_offset + crate::asset_handlers::assets_common::ASSET_HEADER_SIZE
                        ..asset_offset
                            + crate::asset_handlers::assets_common::ASSET_HEADER_SIZE
                            + 6]
                        == b"SFXSTR"
                {
                    asset_type = crate::asset_handlers::assets_common::AssetType::Sfxstr;
                }

                crate::asset_handlers::assets_common::AssetHeader {
                    asset_name: format!(
                        "{} {:#X}",
                        crate::asset_handlers::assets_common::AssetType::get_icon(&asset_type),
                        asset_offset
                    ),
                    asset_offset,
                    asset_size,
                    asset_type,
                    asset_extra_data_size: extra_size,
                }
            })
            .collect()
    }

    #[inline(always)]
    pub fn get_asset_data(&self, include_header: bool) -> Option<std::cell::RefMut<'_, [u8]>> {
        let index = *self.selected_asset.as_ref()?;
        if index == usize::MAX {
            return None;
        }

        let header = self.assets.get(index)?;
        let start = header.asset_offset
            + if include_header {
                0
            } else {
                crate::asset_handlers::assets_common::ASSET_HEADER_SIZE
                    + header.asset_extra_data_size
            };
        let end = start
            + if include_header {
                crate::asset_handlers::assets_common::ASSET_HEADER_SIZE
                    + header.asset_extra_data_size
                    + header.asset_size
            } else {
                header.asset_size
            };

        let file_ref = self.file.borrow_mut();
        Some(std::cell::RefMut::map(file_ref, |f| {
            &mut f.buffer[start..end]
        }))
    }

    fn export_asset(&mut self, data: Vec<u8>) {
        self.export_data = data;
        self.export_dialog.save_file();
    }

    fn create_asset_handler(
        &self,
        asset: &crate::asset_handlers::assets_common::AssetHeader,
    ) -> Box<dyn crate::asset_handlers::assets_common::AssetHandler> {
        use crate::asset_handlers::assets_common::{AssetType, ASSET_HEADER_SIZE};

        let base_offset = asset.asset_offset + ASSET_HEADER_SIZE + asset.asset_extra_data_size;

        match asset.asset_type {
            AssetType::Img => Box::new(crate::asset_handlers::img::IMGHandler::new(
                self.file.clone(),
                base_offset,
                asset.asset_size,
            )),

            AssetType::Pal => Box::new(crate::asset_handlers::pal::PALHandler::new(
                self.file.clone(),
                base_offset,
                asset.asset_size,
            )),

            AssetType::Rda => Box::new(crate::asset_handlers::rda::RDAHandler::new(
                self.file.clone(),
                base_offset,
            )),

            AssetType::Riff => Box::new(crate::asset_handlers::riff::RIFFHandler::new(
                self.file.clone(),
                base_offset,
                asset.asset_size,
            )),

            AssetType::Sfxstr => Box::new(crate::asset_handlers::sfxstr::SFXSTRHandler::new(
                self.file.clone(),
                base_offset,
                asset.asset_size,
            )),

            _ => Box::new(crate::asset_handlers::default::DefaultHandler::new()),
        }
    }

    fn highlight_text(name: &str, needle: &str) -> String {
        if needle.is_empty() {
            return name.to_owned();
        }

        if let Some(pos) = name.to_lowercase().find(&needle.to_lowercase()) {
            let end = pos + needle.len();

            let mut out = String::with_capacity(name.len() + 2);

            out.push_str(&name[..pos]);
            out.push('[');
            out.push_str(&name[pos..end]);
            out.push(']');
            out.push_str(&name[end..]);

            out
        } else {
            name.to_owned()
        }
    }
}
