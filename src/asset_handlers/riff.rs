// SPDX-License-Identifier: GPL-3.0-or-later

pub enum MetadataValue {
    Smpl { fields: Vec<(String, String)> },
    List { entries: Vec<(String, String)> },
    Unknown { size: usize },
}

pub struct MetadataChunk {
    pub tag: String,
    pub size: i32,
    pub value: MetadataValue,
}

pub struct RIFFHandler {
    asset_size: usize,
    file: std::rc::Rc<std::cell::RefCell<crate::PsyFile>>,
    offset: usize,
    sink: Option<rodio::Player>,
    speed: f32,
    stream_handle: Option<rodio::MixerDeviceSink>,
    volume: f32,
    riff_chunk_id: String,
    riff_chunk_size: i32,
    format_id: String,
    riff_subchunk1_id: String,
    riff_subchunk1_size: i32,
    audio_format: i16,
    num_channels: i16,
    sample_rate: i32,
    byte_rate: i32,
    block_align: i16,
    bits_per_sample: i16,
    riff_subchunk2_id: String,
    riff_subchunk2_size: i32,
    waveform_points: Vec<egui_plot::PlotPoint>,
    metadata_chunks: Vec<MetadataChunk>,
    audio_cache: Option<std::sync::Arc<[u8]>>,
    // Samples
    // LIST Medtadata
}

impl RIFFHandler {
    pub fn new(
        file: std::rc::Rc<std::cell::RefCell<crate::PsyFile>>,
        offset: usize,
        asset_size: usize,
    ) -> Self {
        let file_ref = file.borrow();
        let buf = &file_ref.buffer;

        let riff_chunk_id = std::str::from_utf8(&buf[offset..offset + 4])
            .unwrap_or("????")
            .to_string();
        let riff_chunk_size = i32::from_le_bytes(buf[offset + 4..offset + 8].try_into().unwrap());
        let format_id = std::str::from_utf8(&buf[offset + 8..offset + 12])
            .unwrap_or("????")
            .to_string();
        let riff_subchunk1_id = std::str::from_utf8(&buf[offset + 12..offset + 16])
            .unwrap_or("????")
            .to_string();
        let riff_subchunk1_size =
            i32::from_le_bytes(buf[offset + 16..offset + 20].try_into().unwrap());
        let audio_format = i16::from_le_bytes(buf[offset + 20..offset + 22].try_into().unwrap());
        let num_channels = i16::from_le_bytes(buf[offset + 22..offset + 24].try_into().unwrap());
        let sample_rate = i32::from_le_bytes(buf[offset + 24..offset + 28].try_into().unwrap());
        let byte_rate = i32::from_le_bytes(buf[offset + 28..offset + 32].try_into().unwrap());
        let block_align = i16::from_le_bytes(buf[offset + 32..offset + 34].try_into().unwrap());
        let bits_per_sample = i16::from_le_bytes(buf[offset + 34..offset + 36].try_into().unwrap());
        let riff_subchunk2_id = std::str::from_utf8(&buf[offset + 36..offset + 40])
            .unwrap_or("????")
            .to_string();
        let riff_subchunk2_size =
            i32::from_le_bytes(buf[offset + 40..offset + 44].try_into().unwrap());

        let waveform_raw = crate::asset_handlers::assets_common::riff_to_waveform(
            &buf[offset + 44..offset + 44 + riff_subchunk2_size as usize],
            num_channels,
            bits_per_sample,
        );
        let waveform_points: Vec<egui_plot::PlotPoint> = waveform_raw
            .into_iter()
            .map(|p| egui_plot::PlotPoint::new(p[0], p[1]))
            .collect();

        let mut metadata_offset = offset + 44 + riff_subchunk2_size as usize;

        // Chunks are padded to even sizes
        if !metadata_offset.is_multiple_of(2) {
            metadata_offset += 1;
        }

        let mut metadata_chunks: Vec<MetadataChunk> = Vec::new();
        let mut pos = metadata_offset;
        let asset_end = offset + asset_size;

        while pos + 8 <= asset_end {
            let raw_tag = &buf[pos..pos + 4];
            let tag = std::str::from_utf8(raw_tag).unwrap_or("????").to_string();

            let size = i32::from_le_bytes(buf[pos + 4..pos + 8].try_into().unwrap()) as usize;

            let data_start = pos + 8;
            let data_end = data_start + size;

            if data_end > asset_end {
                break;
            }

            let raw = &buf[data_start..data_end];

            let value = match tag.as_str() {
                "smpl" => Self::parse_smpl(raw),
                "LIST" => Self::parse_list(raw),
                _ => MetadataValue::Unknown { size: raw.len() },
            };

            metadata_chunks.push(MetadataChunk {
                tag,
                size: size as i32,
                value,
            });

            pos = data_end;

            // Chunks are padded to even sizes
            if !pos.is_multiple_of(2) {
                pos += 1;
            }
        }

        drop(file_ref);

        Self {
            asset_size,
            file,
            offset,
            sink: None,
            speed: 1.0,
            stream_handle: None,
            volume: 0.5,
            riff_chunk_id,
            riff_chunk_size,
            format_id,
            riff_subchunk1_id,
            riff_subchunk1_size,
            audio_format,
            num_channels,
            sample_rate,
            byte_rate,
            block_align,
            bits_per_sample,
            riff_subchunk2_id,
            riff_subchunk2_size,
            waveform_points,
            metadata_chunks,
            audio_cache: None,
        }
    }

    fn parse_smpl(raw: &[u8]) -> MetadataValue {
        if raw.len() < 36 {
            return MetadataValue::Smpl {
                fields: vec![("Error".into(), "SMPL chunk too small".into())],
            };
        }

        let manufacturer = u32::from_le_bytes(raw[0..4].try_into().unwrap());
        let product = u32::from_le_bytes(raw[4..8].try_into().unwrap());
        let sample_period = u32::from_le_bytes(raw[8..12].try_into().unwrap());
        let midi_unity_note = u32::from_le_bytes(raw[12..16].try_into().unwrap());
        let midi_pitch_fraction = u32::from_le_bytes(raw[16..20].try_into().unwrap());
        let smpte_format = u32::from_le_bytes(raw[20..24].try_into().unwrap());
        let smpte_offset = u32::from_le_bytes(raw[24..28].try_into().unwrap());
        let num_loops = u32::from_le_bytes(raw[28..32].try_into().unwrap());
        let sampler_data = u32::from_le_bytes(raw[32..36].try_into().unwrap());

        MetadataValue::Smpl {
            fields: vec![
                ("Manufacturer".into(), manufacturer.to_string()),
                ("Product".into(), product.to_string()),
                ("Sample Period".into(), sample_period.to_string()),
                ("MIDI Unity Note".into(), midi_unity_note.to_string()),
                (
                    "MIDI Pitch Fraction".into(),
                    midi_pitch_fraction.to_string(),
                ),
                ("SMPTE Format".into(), smpte_format.to_string()),
                ("SMPTE Offset".into(), smpte_offset.to_string()),
                ("Num Loops".into(), num_loops.to_string()),
                ("Sampler Data".into(), sampler_data.to_string()),
            ],
        }
    }

    fn parse_list(raw: &[u8]) -> MetadataValue {
        if raw.len() < 4 || &raw[0..4] != b"INFO" {
            return MetadataValue::List {
                entries: vec![("Error".into(), "LIST chunk missing INFO header".into())],
            };
        }

        let mut pos = 4;
        let mut entries = Vec::new();

        while pos + 8 <= raw.len() {
            let tag = std::str::from_utf8(&raw[pos..pos + 4])
                .unwrap_or("????")
                .to_string();
            let len = u32::from_le_bytes(raw[pos + 4..pos + 8].try_into().unwrap()) as usize;

            let start = pos + 8;
            let end = start + len;

            if end > raw.len() {
                break;
            }

            let value = std::str::from_utf8(&raw[start..end])
                .unwrap_or("")
                .to_string();
            entries.push((tag, value));

            pos = end;
            if pos % 2 != 0 {
                pos += 1;
            }
        }

        MetadataValue::List { entries }
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

    fn riff_audio_data(&mut self) -> std::sync::Arc<[u8]> {
        if let Some(data) = &self.audio_cache {
            return std::sync::Arc::clone(data);
        }

        let data = {
            let file_ref = self.file.borrow();
            let slice = &file_ref.buffer[self.offset..self.offset + self.asset_size];
            std::sync::Arc::<[u8]>::from(slice)
        };
        self.audio_cache = Some(std::sync::Arc::clone(&data));
        data
    }

    fn play_riff(&mut self) {
        let cursor = std::io::Cursor::new(self.riff_audio_data());

        self.stream_handle =
            Some(rodio::DeviceSinkBuilder::open_default_sink().expect("Open Default Audio Stream"));
        if self.stream_handle.is_some() {
            self.stream_handle.as_mut().unwrap().log_on_drop(false);
        }
        let player =
            rodio::Player::connect_new(self.stream_handle.as_ref().expect("Building Sink").mixer());

        let source = rodio::Decoder::new(cursor).unwrap();
        player.set_volume(self.volume);
        player.set_speed(self.speed);
        player.append(source);

        self.sink = Some(player);
    }
}

impl super::assets_common::AssetHandler for RIFFHandler {
    fn update(&mut self, _ctx: &egui::Context) {
        if self.sink.as_ref().is_some_and(|player| player.empty()) {
            self.audio_cache = None;
        }
    }
    fn display_right_panel(&mut self, ui: &mut egui::Ui) {
        ui.heading("RIFF Audio Asset");
        ui.label("These assets are RIFF Audio/WAVE files. Some contain LIST, INFO or smpl data appended to the end.");
    }

    fn display_centre_panel(&mut self, ui: &mut egui::Ui) {
        ui.add_space(5.0);
        ui.horizontal(|ui| {
            egui::Grid::new("riff_header_data_grid")
                .num_columns(2)
                .striped(true)
                .min_col_width(150.0)
                .show(ui, |ui| {
                    ui.strong("RIFF Header Data");
                    ui.end_row();

                    ui.label("RIFF Chunk ID");
                    ui.label(&self.riff_chunk_id);
                    ui.end_row();

                    ui.label("RIFF Chunk Size (bytes)");
                    ui.label(format!("{}", self.riff_chunk_size));
                    ui.end_row();

                    ui.label("Format ID");
                    ui.label(&self.format_id);
                    ui.end_row();

                    ui.label("Subchunk 1 ID");
                    ui.label(&self.riff_subchunk1_id);
                    ui.end_row();

                    ui.label("Subchunk 1 Size (bytes)");
                    ui.label(format!("{}", self.riff_subchunk1_size));
                    ui.end_row();

                    ui.label("Audio Format");
                    ui.label(format!("{}", self.audio_format));
                    ui.end_row();

                    ui.label("Number of Channels");
                    ui.label(format!("{}", self.num_channels))
                        .on_hover_text("1 = Mono, 2 = Stereo");
                    ui.end_row();

                    ui.label("Sample Rate (Hz)");
                    ui.label(format!("{}", self.sample_rate));
                    ui.end_row();

                    ui.label("Byte Rate");
                    ui.label(format!("{}", self.byte_rate));
                    ui.end_row();

                    ui.label("Block Align");
                    ui.label(format!("{}", self.block_align));
                    ui.end_row();

                    ui.label("Bits Per Sample");
                    ui.label(format!("{}", self.bits_per_sample));
                    ui.end_row();

                    ui.label("Subchunk 2 ID");
                    ui.label(&self.riff_subchunk2_id);
                    ui.end_row();

                    ui.label("Subchunk 2 Size (bytes)");
                    ui.label(format!("{}", self.riff_subchunk2_size));
                    ui.end_row();
                });

            ui.vertical(|ui| {
                if !self.metadata_chunks.is_empty() {
                    ui.strong("Metadata Chunks");

                    for (i, chunk) in self.metadata_chunks.iter().enumerate() {
                        ui.push_id(i, |ui| {
                            egui::Grid::new(i)
                                .num_columns(2)
                                .striped(true)
                                .min_col_width(150.0)
                                .show(ui, |ui| {
                                    ui.label("Metadata Tag");
                                    ui.label(&chunk.tag);
                                    ui.end_row();

                                    ui.label("Size");
                                    ui.label(chunk.size.to_string());
                                    ui.end_row();

                                    match &chunk.value {
                                        MetadataValue::Smpl { fields } => {
                                            for (label, value) in fields {
                                                ui.label(label).on_hover_text("The sample chunk allows a MIDI sampler to use the Wave file as a collection of samples.
                                                \nSome software also use this chunk for extra data eg loop values.");
                                                ui.label(value);
                                                ui.end_row();
                                            }
                                        }

                                        MetadataValue::List { entries } => {
                                            for (tag, value) in entries {
                                                ui.label(tag).on_hover_text("ISFT = Software Info
                                                \nICRD = Creation Date Info
                                                \nIENG = Engineer Info");
                                                ui.label(value);
                                                ui.end_row();
                                            }
                                        }

                                        MetadataValue::Unknown { size } => {
                                            ui.label("Unknown Metadata");
                                            ui.label(format!("{} bytes of data", size));
                                            ui.end_row();
                                        }
                                    }
                                });
                        });
                    }
                }
            });
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
                self.play_riff();
            };
            if ui
                .add_sized([80.0, 40.0], egui::Button::new("Stop"))
                .clicked()
            {
                if let Some(player) = &self.sink {
                    player.stop();
                }
            };
        });
    }
}
