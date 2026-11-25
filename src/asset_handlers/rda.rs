// SPDX-License-Identifier: GPL-3.0-or-later

const RDA_HEADER_SIZE: usize = 0x10;

pub struct RDAHandler {
    magic: String,
    num_indices: i32,
    total_assets_size: i32,
    cached_data: Vec<String>,
}

impl RDAHandler {
    pub fn new(file: std::rc::Rc<std::cell::RefCell<crate::PsyFile>>, offset: usize) -> Self {
        let file_ref = file.borrow();
        let buf = &file_ref.buffer;

        let magic = std::str::from_utf8(&buf[offset..offset + 8])
            .unwrap_or("?")
            .to_string();

        let num_indices = i32::from_le_bytes(buf[offset + 8..offset + 12].try_into().unwrap());

        let total_assets_size =
            i32::from_le_bytes(buf[offset + 12..offset + 16].try_into().unwrap());

        let mut cached_data = Vec::with_capacity(num_indices as usize);

        for i in 0..num_indices {
            let start = offset + RDA_HEADER_SIZE + (i as usize * 4);
            let end = start + 4;

            cached_data.push(format!(
                "{}",
                i32::from_le_bytes(buf[start..end].try_into().unwrap())
            ));
        }

        drop(file_ref);

        Self {
            magic,
            num_indices,
            total_assets_size,
            cached_data,
        }
    }
}

impl super::assets_common::AssetHandler for RDAHandler {
    fn update(&mut self, _ctx: &egui::Context) {}

    fn display_right_panel(&mut self, ui: &mut egui::Ui) {
        ui.heading("RDA Asset");
        ui.label(
            "RDA assets are a list of indices (32bit) into *.RDT files, with a 16 byte header.",
        );
    }

    fn display_centre_panel(&mut self, ui: &mut egui::Ui) {
        ui.add_space(5.0);
        egui::Grid::new("rda_header_data_grid")
            .num_columns(2)
            .striped(true)
            .min_col_width(150.0)
            .show(ui, |ui| {
                ui.strong("RDA Header Data");
                ui.end_row();

                ui.label("Magic");
                ui.label(&self.magic);
                ui.end_row();

                ui.label("Number of Indices");
                ui.label(self.num_indices.to_string());
                ui.end_row();

                ui.label("Total Size of Assets (bytes)");
                ui.label(self.total_assets_size.to_string());
                ui.end_row();
            });
        ui.separator();
        ui.add_space(5.0);
        egui::Grid::new("rda_indices_data_grid")
            .num_columns(1)
            .striped(true)
            .min_col_width(150.0)
            .show(ui, |ui| {
                ui.strong("RDA Data");
                ui.end_row();
                for i in 0..self.num_indices {
                    ui.label(&self.cached_data[i as usize]);
                    ui.end_row();
                }
            });
    }
}
