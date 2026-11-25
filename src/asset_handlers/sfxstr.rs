// SPDX-License-Identifier: GPL-3.0-or-later

const SFXSTR_HEADER_SIZE: usize = 0x14;

pub struct SFXSTRHandler {
    asset_size: usize,
    bytes_per_channel: i16,
    export_data: Vec<u8>,
    export_dialog: egui_file_dialog::FileDialog,
    file: std::rc::Rc<std::cell::RefCell<crate::PsyFile>>,
    magic: String,
    num_channels: i16,
    offset: usize,
    sample_data_size: usize,
    sample_rate: i32,
    sink: Option<rodio::Player>,
    speed: f32,
    stream_handle: Option<rodio::MixerDeviceSink>,
    volume: f32,
    wav_header: Vec<u8>,
    waveform_points: std::sync::Arc<Vec<egui_plot::PlotPoint>>,
}

impl SFXSTRHandler {
    pub fn new(
        file: std::rc::Rc<std::cell::RefCell<crate::PsyFile>>,
        offset: usize,
        asset_size: usize,
    ) -> Self {
        let file_ref = file.borrow();
        let buf = &file_ref.buffer;

        let magic = std::str::from_utf8(&buf[offset..offset + 8])
            .unwrap_or("INVALID!")
            .to_string();

        let bytes_per_channel =
            i16::from_le_bytes(buf[offset + 18..offset + 20].try_into().unwrap());
        let num_channels = i16::from_le_bytes(buf[offset + 16..offset + 18].try_into().unwrap());
        let sample_data_size =
            i32::from_le_bytes(buf[offset + 8..offset + 12].try_into().unwrap()) as usize;
        let sample_rate = i32::from_le_bytes(buf[offset + 12..offset + 16].try_into().unwrap());

        let bits_per_sample = bytes_per_channel * 8;

        let wav_header = {
            let data_len = sample_data_size as u32;
            let file_size = 36u32 + data_len;
            let block_align = (num_channels as u16) * (bytes_per_channel as u16);
            let byte_rate = sample_rate as u32 * num_channels as u32 * bytes_per_channel as u32;

            let mut h = Vec::with_capacity(44);
            h.extend(b"RIFF");
            h.extend(&file_size.to_le_bytes());
            h.extend(b"WAVE");
            h.extend(b"fmt ");
            h.extend(&16u32.to_le_bytes());
            h.extend(&1u16.to_le_bytes());
            h.extend(&(num_channels as u16).to_le_bytes());
            h.extend(&(sample_rate as u32).to_le_bytes());
            h.extend(&byte_rate.to_le_bytes());
            h.extend(&block_align.to_le_bytes());
            h.extend(&bits_per_sample.to_le_bytes());
            h.extend(b"data");
            h.extend(&data_len.to_le_bytes());
            h
        };

        let waveform_raw = crate::asset_handlers::assets_common::riff_to_waveform(
            &buf[offset + SFXSTR_HEADER_SIZE..offset + SFXSTR_HEADER_SIZE + sample_data_size],
            num_channels,
            bits_per_sample,
        );
        let waveform_points: Vec<egui_plot::PlotPoint> = waveform_raw
            .into_iter()
            .map(|p| egui_plot::PlotPoint::new(p[0], p[1]))
            .collect();

        let export_dialog = egui_file_dialog::FileDialog::new()
            .allow_path_edit_to_save_file_without_extension(true)
            .as_modal(true)
            .default_pos(egui::Pos2::new(10.0, 10.0))
            .title("Export as Wave Audio (*.wav)")
            .allow_file_overwrite(true)
            .default_file_name("Untitled.wav");

        drop(file_ref);

        Self {
            asset_size,
            bytes_per_channel,
            export_data: Vec::new(),
            export_dialog,
            file,
            magic,
            num_channels,
            offset,
            sample_data_size,
            sample_rate,
            sink: None,
            speed: 1.0,
            stream_handle: None,
            volume: 0.5,
            wav_header,
            waveform_points: std::sync::Arc::new(waveform_points),
        }
    }

    fn show_waveform(&self, ui: &mut egui::Ui) {
        let line = egui_plot::Line::new("PCM Audio", self.waveform_points.as_slice())
            .allow_hover(false)
            .color(egui::Color32::LIGHT_BLUE);

        egui_plot::Plot::new("Waveform")
            .height(150.0)
            .show_grid(false)
            .show_axes(false)
            .allow_axis_zoom_drag(false)
            .allow_boxed_zoom(false)
            .allow_double_click_reset(true)
            .allow_drag(false)
            .allow_scroll(false)
            .allow_zoom(false)
            .center_y_axis(true)
            .clamp_grid(true)
            .show_x(false)
            .show_y(false)
            .show(ui, |plot_ui| {
                plot_ui.line(line);
            });
    }

    #[inline]
    fn wav_from_sfxstr(&self) -> Vec<u8> {
        let file_ref: std::cell::Ref<'_, crate::PsyFile> = self.file.borrow();
        let mut wav_data = Vec::with_capacity(self.wav_header.len() + self.asset_size);
        wav_data.extend(&self.wav_header);
        wav_data.extend(
            &file_ref.buffer[self.offset + SFXSTR_HEADER_SIZE
                ..self.offset + SFXSTR_HEADER_SIZE + self.asset_size],
        );
        wav_data
    }

    fn play_sfxstr(&mut self) {
        let wav_data = self.wav_from_sfxstr();
        let cursor = std::io::Cursor::new(wav_data);

        self.stream_handle =
            Some(rodio::DeviceSinkBuilder::open_default_sink().expect("Open Default Audio Stream"));
        let player =
            rodio::Player::connect_new(self.stream_handle.as_ref().expect("Building Sink").mixer());

        let source = rodio::Decoder::new(cursor).unwrap();
        player.set_volume(self.volume);
        player.set_speed(self.speed);
        player.append(source);

        self.sink = Some(player);
    }

    fn export_sfxstr_as_wav(&mut self) {
        self.export_data = self.wav_from_sfxstr();
        self.export_dialog.save_file();
    }
}

impl super::assets_common::AssetHandler for SFXSTRHandler {
    fn update(&mut self, ctx: &egui::Context) {
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
    }

    fn display_right_panel(&mut self, ui: &mut egui::Ui) {
        ui.heading("SFXSTR Asset");
        ui.label("SFXSTR assets are unsigned 8-bit mono pulse-code modulation (PCM) audio data streams with a 20 byte header.");
    }
    fn display_centre_panel(&mut self, ui: &mut egui::Ui) {
        ui.add_space(5.0);

        egui::Grid::new("sfxstr_header_data_grid")
            .num_columns(2)
            .striped(true)
            .min_col_width(150.0)
            .show(ui, |ui| {
                ui.strong("SFXSTR Header Data");
                ui.end_row();

                ui.label("Magic");
                ui.label(&self.magic);
                ui.end_row();

                ui.label("Sample Data Size (bytes)");
                ui.label(format!("{}", self.sample_data_size));
                ui.end_row();

                ui.label("Sample Rate (Hz)");
                ui.label(format!("{}", self.sample_rate));
                ui.end_row();

                ui.label("Number of Channels");
                ui.label(format!("{}", self.num_channels))
                    .on_hover_text("1 = Mono, 2 = Stereo");
                ui.end_row();

                ui.label("Bytes Per Sample");
                ui.label(format!("{}", self.bytes_per_channel));
                ui.end_row();
            });
        ui.separator();
        ui.add_space(5.0);
        self.show_waveform(ui);
        ui.horizontal(|ui| {
            ui.add_space(5.0);
            ui.label("Volume Multiplier:");
            ui.add(
                egui::Slider::new(&mut self.volume, 0.0..=1.0)
                    .logarithmic(false)
                    .fixed_decimals(2)
                    .update_while_editing(false),
            );
            ui.label("Speed Multiplier:");
            ui.add(
                egui::Slider::new(&mut self.speed, 0.1..=2.0)
                    .logarithmic(false)
                    .fixed_decimals(2)
                    .update_while_editing(false),
            );
        });
        ui.add_space(5.0);
        ui.horizontal(|ui| {
            if ui
                .add_sized([80.0, 40.0], egui::Button::new("Play"))
                .clicked()
            {
                self.play_sfxstr();
            };
            if ui
                .add_sized([80.0, 40.0], egui::Button::new("Stop"))
                .clicked()
            {
                if let Some(player) = &self.sink {
                    player.stop();
                }
            };
            if ui
                .add_sized([80.0, 40.0], egui::Button::new("Export as .wav"))
                .on_hover_text("Exports the audio samples and adds a basic Wave File Header to allow the file to be opened by most media players/editors.")
                .clicked()
            {
                self.export_sfxstr_as_wav();
            };
        });
    }
}
