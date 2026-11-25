// SPDX-License-Identifier: GPL-3.0-or-later
pub struct DefaultHandler {}

impl crate::PsyFileHandler for DefaultHandler {
    fn update(&mut self, _gui: &mut crate::gui_common::CoreGUI, _ctx: &egui::Context) {}
    fn display_left_panel(&mut self, _ui: &mut egui::Ui) {}
    fn display_right_panel(&mut self, _ui: &mut egui::Ui) {}
    fn display_centre_panel(&mut self, ui: &mut egui::Ui) {
        ui.add_space(5.0);
        ui.colored_label(
            egui::Color32::RED,
            "This file type is either unsupported or has no extension (therefore could not be matched with a file handler).",
        );
    }
}

impl DefaultHandler {
    pub fn new(_file: std::rc::Rc<std::cell::RefCell<crate::PsyFile>>) -> Self {
        Self {}
    }
}
