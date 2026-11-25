// SPDX-License-Identifier: GPL-3.0-or-later

pub struct DefaultHandler {}

impl DefaultHandler {
    pub fn new() -> Self {
        Self {}
    }
}

impl super::assets_common::AssetHandler for DefaultHandler {
    fn update(&mut self, _ctx: &egui::Context) {}
    fn display_right_panel(&mut self, _ui: &mut egui::Ui) {}
    fn display_centre_panel(&mut self, _ui: &mut egui::Ui) {}
}
