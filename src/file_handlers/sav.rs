// SPDX-License-Identifier: GPL-3.0-or-later

const GP_EXPECTED_NUM_SLOTS: [u8; 4] = [0x0F, 0x00, 0x00, 0x00]; // Standard G-Police save game files have 15 slots.
const GP_SAVE_SLOT_SIZE: usize = 0x3D;

pub struct SAVHandler {
    data_changed: bool,
    file: std::rc::Rc<std::cell::RefCell<crate::PsyFile>>,
    name_changed: bool,
    num_save_slots: [u8; 4],
    root_label: String,
    slot_names: Vec<String>,
    selected_slot: Option<usize>,
}

impl crate::PsyFileHandler for SAVHandler {
    fn update(&mut self, gui: &mut crate::gui_common::CoreGUI, _ctx: &egui::Context) {
        let invalid_selection = match self.selected_slot {
            None => true,
            Some(id) => id == usize::MAX || id >= i32::from_le_bytes(self.num_save_slots) as usize,
        };

        if invalid_selection {
            if self.data_changed {
                gui.update_data_view(&mut []);
                self.data_changed = false;
            }
        } else if self.data_changed {
            if let Some(mut slot) = self.get_slot_data() {
                gui.update_data_view(&mut slot);
            } else {
                gui.update_data_view(&mut []);
            }
            self.data_changed = false;
        }
        if self.name_changed {
            self.slot_names =
                Self::get_slot_names(&self.file, i32::from_le_bytes(self.num_save_slots) as usize);
            self.name_changed = false;
        }
    }

    fn display_left_panel(&mut self, ui: &mut egui::Ui) {
        let mut selected_slot = self.selected_slot.unwrap_or(usize::MAX);

        ui.vertical(|ui| {
            ui.label(self.root_label.as_str());
            ui.add_space(2.0);

            let row_height = ui
                .text_style_height(&egui::TextStyle::Body)
                .max(ui.spacing().interact_size.y);
            let clicked_slot = egui::ScrollArea::vertical()
                .id_salt("sav_left_panel")
                .show_rows(ui, row_height, self.slot_names.len(), |ui, row_range| {
                    let mut clicked_slot = None;
                    for row_index in row_range {
                        let label = &self.slot_names[row_index];
                        ui.set_min_width(ui.available_width());
                        let response = ui.selectable_value(
                            &mut selected_slot,
                            row_index,
                            format!("{} {}", crate::gui_common::ICON_FLOPPY, label),
                        );
                        if response.clicked() {
                            clicked_slot = Some(row_index);
                        }
                    }
                    clicked_slot
                })
                .inner;

            if let Some(index) = clicked_slot {
                if self.selected_slot != Some(index) {
                    self.selected_slot = Some(index);
                    self.data_changed = true;
                }
            }
        });
    }

    fn display_right_panel(&mut self, ui: &mut egui::Ui) {
        ui.heading("Meta Information");
        ui.add_space(5.0);
        ui.strong("Number of Save Slots:");
        ui.label(format!("{}", u32::from_le_bytes(self.num_save_slots)));
        ui.separator();
        ui.label(format!("Although the G-Police save file appears to specify the number of possible save slots in its first few bytes ({:02X?}), modifying this value does not appear to actually increase or decrease the number of available slots in-game.", self.num_save_slots));
        ui.add_space(10.0);
        ui.label("Each slot consists of 61 bytes. The first two bytes in each slot are always 0x23 0x23 (##).");
        ui.add_space(10.0);
        ui.label("The exact purpose of all of the bytes within each slot is not known at this time. Those which are known can be edited in the main view.");
    }

    fn display_centre_panel(&mut self, ui: &mut egui::Ui) {
        ui.vertical_centered(|ui| {
            if let Some(offset) = self.get_slot_offset() {
                ui.heading(format!("Saved Game Data at Offset {:#X}", offset));
            } else {
                ui.heading(&*self.root_label);
            }
            ui.add_space(5.0);
            ui.separator();
            ui.add_space(5.0);
        });
        ui.vertical(|ui| {
        if self.num_save_slots != GP_EXPECTED_NUM_SLOTS {
            ui.add_space(5.0);
            ui.colored_label(
                egui::Color32::ORANGE,
                "Warning: Unexpected number of save slots detected! This may be a modified, corrupted, or unsupported .sav file.",
            );
            ui.separator();
            ui.add_space(10.0);
        }

        if self.selected_slot.is_none_or(|id| id > i32::from_le_bytes(self.num_save_slots) as usize) {
            let max_columns = 3;

            // Consider:
            // let rows = i32::from_le_bytes(self.num_save_slots).div_ceil(max_columns);
            // Instead if int_roundings is stabilised. See #88581 <https://github.com/rust-lang/rust/issues/88581>
            let rows = (i32::from_le_bytes(self.num_save_slots) + max_columns - 1) / max_columns;

            let size = 64.0;
            ui.spacing_mut().item_spacing = egui::vec2(size * 0.25, size * 0.25);

            let mut label = 1;
            for row in 0..rows {
                ui.horizontal(|ui| {
                    for col in 0..max_columns {
                        let index = row * max_columns + col;
                        if index >= i32::from_le_bytes(self.num_save_slots) {
                            break;
                        }
                        if ui
                            .add_sized(egui::Vec2::splat(size), egui::Button::new(label.to_string()))
                            .clicked()
                        {
                            self.selected_slot = Some(index as usize);
                            self.data_changed = true;
                        }
                        label += 1;
                    }
                });
            }
            return;
        }

        let mut modified_data = false;
        let mut modified_name = false;
        if let Some(mut save_data) = self.get_slot_data() {
            ui.add_space(5.0);
            egui::ScrollArea::vertical().show(ui, |ui| {
                egui::Grid::new("save_editor_grid")
                    .num_columns(2)
                    .striped(true)
                    .min_col_width(150.0)
                    .show(ui, |ui| {
                        ui.heading("Entry");
                        ui.heading("Value");
                        ui.end_row();

                        ui.label("Name");
                        let mut slot_name = String::from_utf8_lossy(&save_data[19..31]).to_string();
                        let response =
                            ui.add(egui::TextEdit::singleline(&mut slot_name).desired_width(120.0));
                        if response.changed() {
                            slot_name.make_ascii_uppercase();
                            slot_name.retain(|c| c.is_alphanumeric() || c.is_whitespace());
                            let bytes = slot_name.as_bytes();
                            let len = bytes.len().min(12);
                            save_data[19..19 + len].copy_from_slice(&bytes[..len]);
                            save_data[19 + len..31].fill(0);
                            modified_data = true;
                            modified_name = true;
                        }
                        ui.end_row();

                    ui.label("Date");
                    let year = i16::from_le_bytes([save_data[3], save_data[4]]);
                    let month = i8::try_from(save_data[5]).unwrap_or(1);
                    let day = i8::try_from(save_data[9]).unwrap_or(1);

                    if let Ok(mut date_value) = jiff::civil::Date::new(year, month, day) {
                        if ui
                            .add(egui_extras::DatePickerButton::new(&mut date_value))
                            .changed()
                        {
                            let year_bytes = date_value.year().to_le_bytes();
                            save_data[3..5].copy_from_slice(&year_bytes);
                            save_data[5] = date_value.month() as u8;
                            save_data[9] = date_value.day() as u8;
                            modified_data = true;
                        }
                    }
                   else {
                        ui.label("Invalid Date");
                    }
                    ui.end_row();

                    ui.label("Hour (24h)");
                    let mut response = ui.add(egui::DragValue::new(&mut save_data[11]).range(0..=23));
                    ui.end_row();

                    ui.label("Minutes");
                    response |= ui.add(egui::DragValue::new(&mut save_data[13]).range(0..=59));
                    ui.end_row();

                    ui.label("Level Completed");
                    response |= ui
                    .add(egui::DragValue::new(&mut save_data[60]).range(0..=35))
                    .on_hover_text(
                        "Note that a value of zero is interpreted by G-Police as an empty save slot.",
                    );
                    ui.end_row();

                    ui.label("Objectives Achieved");
                    response |= ui.add(egui::DragValue::new(&mut save_data[52]).range(0..=255));
                    ui.end_row();

                    ui.label("Total Objectives");
                    response |= ui.add(egui::DragValue::new(&mut save_data[53]).range(0..=255));
                    ui.end_row();

                    if response.changed() {
                        modified_data = true;
                    }

                    ui.label("Total Enemy Kills");
                    let mut enemy_kills = i16::from_le_bytes([save_data[56], save_data[57]]);
                    if ui
                        .add(egui::DragValue::new(&mut enemy_kills).range(0..=i16::MAX))
                        .changed()
                    {
                        save_data[56..58].copy_from_slice(&enemy_kills.to_le_bytes());
                        modified_data = true;
                    }
                    ui.end_row();

                    ui.label("Total Civilian Kills");
                    let mut civilian_kills = i16::from_le_bytes([save_data[58], save_data[59]]);
                    if ui
                        .add(egui::DragValue::new(&mut civilian_kills).range(0..=i16::MAX))
                        .changed()
                    {
                        save_data[58..60].copy_from_slice(&civilian_kills.to_le_bytes());
                        modified_data = true;
                    }
                    ui.end_row();

                    ui.label("Average Hit Ratio");
                    if ui
                        .add(
                            egui::DragValue::new(&mut save_data[50])
                                .range(0..=100)
                                .suffix("%"),
                        )
                        .changed()
                    {
                        modified_data = true;
                    }
                    ui.end_row();

                    let mut bits = save_data[54];
                    for i in 0..6 {
                        let bit_index = i as u8;
                        let mut checked = (bits & (1 << bit_index)) != 0;
                        ui.label(format!("Secret Mission {} Available", i + 1));
                        if ui.checkbox(&mut checked, "").clicked() {
                            bits ^= 1 << bit_index;
                            modified_data = true;
                        }
                        ui.end_row();
                    }
                    save_data[54] = bits;
                });
            ui.add_space(12.0);

            if ui.button("Reset/Clear Save Slot").clicked() {
                save_data[0..2].fill(0x23);
                save_data[2..GP_SAVE_SLOT_SIZE].fill(0x00);
                modified_data = true;
            }
            });
        }
        if modified_data {
            self.data_changed = true;
        }
        if modified_name {
            self.name_changed = true;
        }
    });
    }
}

impl SAVHandler {
    pub fn new(file: std::rc::Rc<std::cell::RefCell<crate::PsyFile>>) -> Self {
        let num_save_slots: [u8; 4] = file
            .borrow()
            .buffer
            .get(0..4)
            .and_then(|slice| slice.try_into().ok())
            .unwrap_or([0; 4]);

        let root_label = format!(
            "{} {}",
            crate::gui_common::ICON_CONTAINER,
            file.borrow().metadata.name
        );

        let slot_names = Self::get_slot_names(&file, i32::from_le_bytes(num_save_slots) as usize);

        Self {
            data_changed: true,
            file,
            name_changed: false,
            num_save_slots,
            root_label,
            slot_names,
            selected_slot: None,
        }
    }

    fn get_slot_offset(&self) -> Option<usize> {
        if let Some(selected_id) = self.selected_slot.as_ref() {
            if *selected_id == usize::MAX
                || *selected_id >= i32::from_le_bytes(self.num_save_slots) as usize
            {
                return None;
            }
            return Some(4 + (*selected_id) * GP_SAVE_SLOT_SIZE);
        }
        None
    }

    #[inline(always)]
    fn get_slot_data(&self) -> Option<std::cell::RefMut<'_, [u8]>> {
        let slot_offset = self.get_slot_offset()?;
        let end = slot_offset.checked_add(GP_SAVE_SLOT_SIZE)?;

        let file_ref = self.file.borrow_mut();
        let buf_len = file_ref.buffer.len();

        if slot_offset > buf_len || end > buf_len {
            return None;
        }

        Some(std::cell::RefMut::map(file_ref, |f| {
            &mut f.buffer[slot_offset..end]
        }))
    }

    fn get_slot_names(
        file: &std::rc::Rc<std::cell::RefCell<crate::PsyFile>>,
        number_of_slots: usize,
    ) -> Vec<String> {
        let file_ref = file.borrow();
        let buffer = &file_ref.buffer;
        let mut slot_names = Vec::with_capacity(number_of_slots);

        for i in 0..number_of_slots {
            let slot_start = 4 + i * GP_SAVE_SLOT_SIZE;
            let name_start = slot_start + 19;
            let name_end = name_start + 12;

            if name_end > buffer.len() {
                slot_names.push(format!("[{:02}] [INVALID SLOT]", i + 1));
                continue;
            }

            let name_bytes = &buffer[name_start..name_end];
            let name: String = name_bytes
                .iter()
                .take_while(|&&b| b != 0)
                .map(|&b| b as char)
                .collect();

            slot_names.push(format!(
                "[{:02}] {}",
                i + 1,
                if name.is_empty() {
                    "[EMPTY SLOT]"
                } else {
                    &name
                }
            ));
        }
        slot_names
    }
}
