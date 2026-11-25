// SPDX-License-Identifier: GPL-3.0-or-later

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod file_handlers {
    pub mod default;
    pub mod rdt;
    pub mod sav;
}

mod asset_handlers {
    pub mod assets_common;
    pub mod default;
    pub mod img;
    pub mod pal;
    pub mod rda;
    pub mod riff;
    pub mod sfxstr;
}

mod gui_common;

pub trait PsyFileHandler {
    fn update(&mut self, gui: &mut gui_common::CoreGUI, ctx: &egui::Context);
    fn display_left_panel(&mut self, ui: &mut egui::Ui);
    fn display_right_panel(&mut self, ui: &mut egui::Ui);
    fn display_centre_panel(&mut self, ui: &mut egui::Ui);
}

fn handler_from_file(
    file_refcell: std::rc::Rc<std::cell::RefCell<PsyFile>>,
) -> Box<dyn PsyFileHandler> {
    let ext = file_refcell.borrow().metadata.extension.clone();
    match ext.as_str() {
        "rdt" => Box::new(file_handlers::rdt::RDTHandler::new(file_refcell)),
        "sav" => Box::new(file_handlers::sav::SAVHandler::new(file_refcell)),
        _ => Box::new(file_handlers::default::DefaultHandler::new(file_refcell)),
    }
}

#[derive(Default)]
pub struct PsyFileMetadata {
    created: Option<std::time::SystemTime>,
    extension: String,
    length: u64,
    modified: Option<std::time::SystemTime>,
    name: String,
    path: std::path::PathBuf,
}

struct PsyFile {
    buffer: Vec<u8>,
    metadata: PsyFileMetadata,
}

impl PsyFile {
    pub fn open(path: &std::path::Path) -> std::io::Result<Self> {
        let md = std::fs::metadata(path)?;
        let metadata = PsyFileMetadata {
            created: md.created().ok(),
            extension: path
                .extension()
                .and_then(|e| e.to_str())
                .map_or_else(String::new, |s| s.to_ascii_lowercase()),
            length: md.len(),
            modified: md.modified().ok(),
            name: path
                .file_name()
                .and_then(|n| n.to_str())
                .map_or_else(String::new, str::to_owned),
            path: path.to_path_buf(),
        };

        let mut buffer = Vec::with_capacity(md.len() as usize);
        std::io::Read::read_to_end(&mut std::fs::File::open(path)?, &mut buffer)?;

        Ok(PsyFile { buffer, metadata })
    }
}
struct Owl {
    current_file_handler: Option<Box<dyn PsyFileHandler>>,
    current_file: Option<std::rc::Rc<std::cell::RefCell<PsyFile>>>,
    gui: gui_common::CoreGUI,
}

impl Owl {
    pub fn new(_cc: &eframe::CreationContext) -> Self {
        let mut gui = gui_common::CoreGUI::new();
        gui.dark_mode = true;
        gui.apply_theme(&_cc.egui_ctx);
        Self {
            current_file_handler: None,
            current_file: None,
            gui,
        }
    }

    fn handle_file_picker(&mut self, ctx: &egui::Context) {
        self.gui.file_picker.update(ctx);
        if let Some(path) = self.gui.file_picker.take_picked() {
            match PsyFile::open(path.as_path()) {
                Ok(file) => {
                    self.gui.data_view_cache.clear();
                    let file_refcell = std::rc::Rc::new(std::cell::RefCell::new(file));
                    let handler = handler_from_file(file_refcell.clone());
                    self.current_file = Some(file_refcell);
                    self.current_file_handler = Some(handler);
                }
                Err(_) => {
                    eprintln!("Error opening file!");
                }
            };
        }
    }

    fn handle_file_saver(&mut self, ctx: &egui::Context) {
        self.gui.file_saver.update(ctx);
        if let Some(path) = self.gui.file_saver.take_picked() {
            if let Some(f) = &mut self.current_file {
                match std::fs::File::create(&path) {
                    Ok(mut output) => {
                        if let Err(error) =
                            std::io::Write::write_all(&mut output, &f.borrow().buffer)
                        {
                            eprintln!("Failed to write {:?}: {}", path, error);
                        }
                    }
                    Err(error) => {
                        eprintln!("Failed to create {:?}: {}", path, error);
                    }
                }
            } else {
                eprintln!("No file currently open to save.");
            }
        }
    }

    fn update(&mut self, ctx: &egui::Context) {
        if let Some(handler) = &mut self.current_file_handler {
            handler.update(&mut self.gui, ctx);
        }
    }

    fn display_panels(&mut self, ui: &mut egui::Ui) {
        egui::Panel::left("left_panel")
            .default_size(180.0)
            .resizable(false)
            .show(ui, |ui| {
                ui.vertical_centered_justified(|ui| {
                    ui.add_space(9.0);
                    ui.heading("Navigation");
                    ui.add_space(9.0);
                    ui.separator();
                });
                if let Some(handler) = &mut self.current_file_handler {
                    handler.display_left_panel(ui);
                    ui.separator();
                }
            });

        egui::Panel::right("right_panel")
            .default_size(180.0)
            .resizable(false)
            .show(ui, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(9.0);
                    ui.heading("Meta Information");
                    ui.add_space(9.0);
                    ui.separator();
                });
                egui::ScrollArea::vertical().show(ui, |ui| {
                    if let Some(file) = &self.current_file {
                        self.gui.display_file_metadata(ui, &file.borrow().metadata);
                        ui.separator();
                        if let Some(handler) = &mut self.current_file_handler {
                            handler.display_right_panel(ui);
                        }
                    }
                    else
                    {
                        ui.add_space(5.0);
                        ui.label(gui_common::platinum("Platinum:"));
                        ui.label("Full Support for all major features such as viewing, editing and extracting all relevant data (as applicable).");
                        ui.add_space(5.0);
                        ui.label(gui_common::gold("Gold:"));
                        ui.label("Functional support for viewing, editing, extracting all relevant data (as applicable), but some features may be limited.");
                        ui.add_space(5.0);
                        ui.label(gui_common::silver("Silver:"));
                        ui.label("Partial support. Some major features may be limited or unavailable.");
                        ui.add_space(5.0);
                        ui.label(gui_common::bronze("Bronze:"));
                        ui.label("Limited support. All major features are either limited or unavailable.");
                        ui.separator();
                        ui.label(format!("Hover over the {} icon in the table for file format specific details.", gui_common::ICON_INFO));
                    }
                });
            });

        egui::CentralPanel::default().show(ui, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                if let Some(handler) = &mut self.current_file_handler {
                    handler.display_centre_panel(ui);
                    self.gui.display_data_view(ui);
                } else {
                    ui.add_space(5.0);
                    ui.label("Currently supported file types:");
                    ui.add_space(10.0);
                    egui_extras::TableBuilder::new(ui)
                        .column(egui_extras::Column::auto_with_initial_suggestion(150.0))
                        .column(egui_extras::Column::auto_with_initial_suggestion(150.0))
                        .column(egui_extras::Column::auto_with_initial_suggestion(150.0))
                        .column(egui_extras::Column::auto_with_initial_suggestion(150.0))
                        .column(egui_extras::Column::auto_with_initial_suggestion(150.0))
                        .header(20.0, |mut header| {
                            header.col(|ui| {
                                ui.set_min_width(200.0);
                                ui.label(egui::RichText::new("Game").strong());
                            });
                            header.col(|ui| {
                                ui.set_min_width(60.0);
                                ui.label(egui::RichText::new("Platform").strong());
                            });
                            header.col(|ui| {
                                ui.set_min_width(200.0);
                                ui.label(egui::RichText::new("Description").strong());
                            });
                            header.col(|ui| {
                                ui.set_min_width(60.0);
                                ui.label(egui::RichText::new("Extension").strong());
                            });
                            header.col(|ui| {
                                ui.set_min_width(200.0);
                                ui.label(egui::RichText::new("Support").strong());
                            });
                        })
                        .body(|mut body| {
                            body.row(18.0, |mut row| {
                                row.col(|ui| { ui.label("G-Police"); });
                                row.col(|ui| { ui.label("Windows");});
                                row.col(|ui| { ui.label("Saved Game Data"); });
                                row.col(|ui| {ui.label("*.SAV");});
                                row.col(|ui| {
                                    let label_text = format!("Gold {}", gui_common::ICON_INFO);
                                    ui.label(gui_common::gold(&label_text))
                                    .on_hover_text(format!("{} View Saved Game Data\n{} Edit/Write Saved Game Data\n{} Some bytes within the *.sav file still have an unknown purpose.", gui_common::ICON_CHECK, gui_common::ICON_CHECK, gui_common::ICON_HALF_CIRCLE));
                                });
                            });
                            body.row(18.0, |mut row| {
                                row.col(|ui| { ui.label("G-Police"); });
                                row.col(|ui| { ui.label("Windows");});
                                row.col(|ui| { ui.label("Resource Data Table"); });
                                row.col(|ui| {ui.label("*.RDT");});
                                row.col(|ui| {
                                    let label_text = format!("Silver {}", gui_common::ICON_INFO);
                                    ui.label(gui_common::silver(&label_text))
                                    .on_hover_text(format!("{} Some resources cannot be fully viewed\n{} Limited Editing/Writing Data\n{} Extract Data", gui_common::ICON_HALF_CIRCLE, gui_common::ICON_HALF_CIRCLE, gui_common::ICON_CHECK));
                                });
                            });
                            body.row(18.0, |mut row| {
                                row.col(|ui| { ui.label("G-Police"); });
                                row.col(|ui| { ui.label("PSX");});
                                row.col(|ui| { ui.label("Resource Data Table"); });
                                row.col(|ui| {ui.label("*.RDT");});
                                row.col(|ui| {
                                    let label_text = format!("Bronze {}", gui_common::ICON_INFO);
                                    ui.label(gui_common::bronze(&label_text))
                                    .on_hover_text(format!("{} Most resources cannot be fully viewed\n{} Edit/Write Data\n{} Extract Data", gui_common::ICON_HALF_CIRCLE, gui_common::ICON_X, gui_common::ICON_CHECK));
                                });
                            });
                            body.row(18.0, |mut row| {
                                row.col(|ui| { ui.label("G-Police: Weapons of Justice"); });
                                row.col(|ui| { ui.label("PSX");});
                                row.col(|ui| { ui.label("Resource Data Table"); });
                                row.col(|ui| {ui.label("*.RDT");});
                                row.col(|ui| {
                                    let label_text = format!("Bronze {}", gui_common::ICON_INFO);
                                    ui.label(gui_common::bronze(&label_text))
                                    .on_hover_text(format!("{} Most resources cannot be fully viewed\n{} Edit/Write Data\n{} Extract Data", gui_common::ICON_HALF_CIRCLE, gui_common::ICON_X, gui_common::ICON_CHECK));
                                });
                            });
                            body.row(18.0, |mut row| {
                                row.col(|ui| { ui.label("Tellurian Defense"); });
                                row.col(|ui| { ui.label("Windows");});
                                row.col(|ui| { ui.label("Resource Data Table"); });
                                row.col(|ui| {ui.label("*.RDT");});
                                row.col(|ui| {
                                    let label_text = format!("Bronze {}", gui_common::ICON_INFO);
                                    ui.label(gui_common::bronze(&label_text))
                                    .on_hover_text(format!("{} Most resources cannot be fully viewed\n{} Edit/Write Data\n{} Extract Data", gui_common::ICON_HALF_CIRCLE, gui_common::ICON_X, gui_common::ICON_CHECK));
                                });
                            });
                            body.row(18.0, |mut row| {
                                row.col(|ui| { ui.label("Overboard! / Shipwreckers"); });
                                row.col(|ui| { ui.label("Windows");});
                                row.col(|ui| { ui.label("Resource Data Table"); });
                                row.col(|ui| {ui.label("*.RDT");});
                                row.col(|ui| {
                                    let label_text = format!("Bronze {}", gui_common::ICON_INFO);
                                    ui.label(gui_common::bronze(&label_text))
                                    .on_hover_text(format!("{} Most resources cannot be fully viewed\n{} Edit/Write Data\n{} Extract Data", gui_common::ICON_HALF_CIRCLE, gui_common::ICON_X, gui_common::ICON_CHECK));
                                });
                            });
                            body.row(18.0, |mut row| {
                                row.col(|ui| { ui.label("Assault Rigs"); });
                                row.col(|ui| { ui.label("Windows/DOS");});
                                row.col(|ui| { ui.label("Resource Data Table"); });
                                row.col(|ui| {ui.label("*.RDT");});
                                row.col(|ui| {
                                    let label_text = format!("Bronze {}", gui_common::ICON_INFO);
                                    ui.label(gui_common::bronze(&label_text))
                                    .on_hover_text(format!("{} Most resources cannot be fully viewed\n{} Edit/Write Data\n{} Extract Data", gui_common::ICON_HALF_CIRCLE, gui_common::ICON_X, gui_common::ICON_CHECK));
                                });
                            });
                        });
                }
            });
        });
    }
}

impl eframe::App for Owl {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.gui.apply_theme(ctx);
        self.handle_file_picker(ctx);
        self.handle_file_saver(ctx);
        self.update(ctx);
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::Panel::top("menu_bar")
            .frame(
                egui::Frame::side_top_panel(ui.style())
                    .corner_radius(0.0)
                    .shadow(egui::Shadow::NONE),
            )
            .show_separator_line(false)
            .show(ui, |ui| {
                egui::MenuBar::new().ui(ui, |ui| {
                    if let Some(file) = &self.current_file {
                        let allow_save = !file.borrow().buffer.is_empty();
                        self.gui.display_file_menu(ui, allow_save);
                    } else {
                        self.gui.display_file_menu(ui, false);
                    }
                    self.gui.display_options_menu(ui);
                    self.gui.display_about_menu(ui);
                });
                ui.separator();
            });
        self.display_panels(ui);
        self.gui.display_popups(ui);
    }
}

fn main() -> eframe::Result<()> {
    let icon = image::load_from_memory(include_bytes!("../owl.ico"))
        .map(|img| {
            let rgba = img.to_rgba8();
            let (width, height) = rgba.dimensions();
            egui::IconData {
                rgba: rgba.into_raw(),
                width,
                height,
            }
        })
        .unwrap_or_else(|err| {
            eprintln!("Warning: failed to load owl.png: {err}");
            egui::IconData::default()
        });

    let options: eframe::NativeOptions = eframe::NativeOptions {
        renderer: eframe::Renderer::Wgpu,
        viewport: egui::ViewportBuilder::default()
            .with_app_id("owlapp")
            .with_clamp_size_to_monitor_size(true)
            .with_close_button(true)
            .with_icon(icon)
            .with_inner_size(egui::Vec2::new(1152.0, 864.0))
            .with_maximize_button(true)
            .with_min_inner_size(egui::Vec2::new(1152.0, 864.0))
            .with_minimize_button(true),
        centered: true,
        ..Default::default()
    };

    eframe::run_native(
        format!("Owl {}", env!("CARGO_PKG_VERSION")).as_str(),
        options,
        Box::new(|ctx| Ok(Box::new(Owl::new(ctx)))),
    )
}
