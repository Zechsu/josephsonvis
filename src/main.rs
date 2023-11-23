#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use eframe::egui;
use eframe::egui::ColorImage;
use egui::{
    menu,
    plot::{self, Legend, Line, Plot, PlotPoint, PlotPoints, Points},
    Color32, FontId, Layout, Pos2, Stroke, TextStyle, Vec2,
};
use std::{fs, path::PathBuf};

static POSSIBLE_COLORS: [Color32; 11] = [
    Color32::RED,
    Color32::BLUE,
    Color32::LIGHT_GREEN,
    Color32::from_rgb(172, 77, 188),
    Color32::LIGHT_BLUE,
    Color32::KHAKI,
    Color32::from_rgb(118, 77, 188),
    Color32::GREEN,
    Color32::LIGHT_RED,
    Color32::DARK_BLUE,
    Color32::YELLOW,
];

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        drag_and_drop_support: true,
        initial_window_size: Some(egui::vec2(720.0, 640.0)),
        renderer: eframe::Renderer::Glow,
        ..Default::default()
    };
    eframe::run_native(
        "Josephson Visualizer",
        options,
        Box::new(|_cc| Box::<MyApp>::default()),
    )
}

#[derive(PartialEq)]
enum LineMode {
    Normal,
    Derivative,
}

impl Default for LineMode {
    fn default() -> Self {
        LineMode::Normal
    }
}

#[derive(Default)]
struct MyApp {
    dropped_files: Vec<egui::DroppedFile>,
    picked_path: Option<PathBuf>,
    // (f64, f64, f64) = (x, y, y')
    points: Vec<Vec<(f64, f64, f64)>>,
    solutions_count: usize,
    is_visible: Vec<bool>,
    line_mode: LineMode,
    should_reset_plot: bool,
    layer_names: Vec<String>,
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        egui::SidePanel::right("layers_panel")
            .resizable(false)
            .min_width(320.0)
            .show(ctx, |ui| {
                let mut should_break = false;
                for sol in 0..self.solutions_count {
                    ui.with_layout(egui::Layout::left_to_right(egui::Align::TOP), |ui| {
                        let fnt = FontId {
                            size: 16.0,
                            family: egui::FontFamily::Proportional,
                        };
                        ui.label(
                            egui::RichText::new(format!("{}", self.layer_names[sol]))
                                .color(POSSIBLE_COLORS[sol % POSSIBLE_COLORS.len()])
                                .font(fnt),
                        );
                        let name = if self.is_visible[sol] { "Hide" } else { "Show" };
                        if ui.button(name).clicked() {
                            self.is_visible[sol] = !self.is_visible[sol];
                        }
                        if ui.button("X").clicked() {
                            self.points.remove(sol);
                            self.is_visible.remove(sol);
                            self.layer_names.remove(sol);
                            self.solutions_count -= 1;
                            should_break = true;
                        }
                    });
                    ui.add_space(5.0);
                    if should_break {
                        break;
                    }
                }
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Open").clicked() {
                        if let Some(path) = rfd::FileDialog::new().pick_file() {
                            ui.close_menu();
                            self.picked_path = Some(path);
                        }
                    }
                    if ui.button("Clear").clicked() {
                        self.points.clear();
                        self.solutions_count = 0;
                        self.is_visible.clear();
                        self.layer_names.clear();
                        ui.close_menu();
                    }

                    if ui.button("Exit").clicked() {
                        frame.close();
                    }
                });
            });

            ui.with_layout(Layout::left_to_right(egui::Align::TOP), |ui| {
                if ui
                    .radio_value(&mut self.line_mode, LineMode::Normal, "Flux")
                    .clicked()
                {
                    self.should_reset_plot = true;
                }
                if ui
                    .radio_value(&mut self.line_mode, LineMode::Derivative, "Field")
                    .clicked()
                {
                    self.should_reset_plot = true;
                }
            });

            if let Some(picked_path) = &self.picked_path {
                let sol = fs::read_to_string(picked_path);
                if sol.is_ok() {
                    let sol = sol.unwrap();
                    let mut contents = sol.split('\n').collect::<Vec<&str>>();
                    contents.remove(0);

                    let he = contents[0].split_whitespace().collect::<Vec<&str>>()[17]
                        .trim_end_matches(',');
                    let gamma = contents[0].split_whitespace().collect::<Vec<&str>>()[20]
                        .trim_end_matches(',');

                    self.points.push(Vec::new());
                    for x in 0..contents.len() - 1 {
                        let line = contents[x].split_ascii_whitespace().collect::<Vec<&str>>();
                        let (x, y, yp) = (line[0], line[1], line[2]);
                        let (x, y, yp) = (
                            x.parse::<f64>().unwrap(),
                            y.parse::<f64>().unwrap(),
                            yp.parse::<f64>().unwrap(),
                        );

                        self.points[self.solutions_count].push((x, y, yp));
                    }

                    let mut flname = picked_path.file_name().unwrap().to_str().unwrap();
                    flname = flname.trim_end_matches(".DAT");

                    self.layer_names
                        .push(format!("{} he={} gamma={}", flname, he, gamma));
                    self.is_visible.push(true);
                    self.picked_path = None;
                    self.solutions_count += 1;
                }
            }

            let plot_space = Plot::new("Plot")
                .show_background(false)
                .auto_bounds_x()
                .auto_bounds_y()
                .clamp_grid(true)
                .allow_boxed_zoom(false)
                .allow_drag(true)
                .allow_scroll(false);

            if self.should_reset_plot {
                plot_space.reset().show(ui, |plot_ui| {
                    for sol in 0..self.solutions_count {
                        if !self.is_visible[sol] {
                            continue;
                        }

                        let pl: PlotPoints;
                        match self.line_mode {
                            LineMode::Normal => {
                                pl = self.points[sol].iter().map(|i| [i.0, i.1]).collect();
                            }
                            LineMode::Derivative => {
                                pl = self.points[sol].iter().map(|i| [i.0, i.2]).collect();
                            }
                        }

                        let line = Line::new(pl)
                            .width(3.0)
                            .name(self.layer_names[sol].clone())
                            .color(POSSIBLE_COLORS[sol % POSSIBLE_COLORS.len()]);
                        plot_ui.line(line);
                    }
                });

                self.should_reset_plot = false;
            } else {
                plot_space.show(ui, |plot_ui| {
                    for sol in 0..self.solutions_count {
                        if !self.is_visible[sol] {
                            continue;
                        }

                        let pl: PlotPoints;
                        match self.line_mode {
                            LineMode::Normal => {
                                pl = self.points[sol].iter().map(|i| [i.0, i.1]).collect();
                            }
                            LineMode::Derivative => {
                                pl = self.points[sol].iter().map(|i| [i.0, i.2]).collect();
                            }
                        }

                        let line = Line::new(pl)
                            .width(3.0)
                            .name(self.layer_names[sol].clone())
                            .color(POSSIBLE_COLORS[sol % POSSIBLE_COLORS.len()]);
                        plot_ui.line(line);
                    }
                });
            }
        });

        if !self.dropped_files.is_empty() {
            for file in &self.dropped_files {
                let mut info = if let Some(path) = &file.path {
                    let sol = fs::read_to_string(path);
                    if sol.is_ok() {
                        let sol = sol.unwrap();
                        let mut contents = sol.split('\n').collect::<Vec<&str>>();
                        contents.remove(0);

                        let he = contents[0].split_whitespace().collect::<Vec<&str>>()[17]
                            .trim_end_matches(',');
                        let gamma = contents[0].split_whitespace().collect::<Vec<&str>>()[20]
                            .trim_end_matches(',');

                        self.points.push(Vec::new());
                        for x in 0..contents.len() - 1 {
                            let line = contents[x].split_ascii_whitespace().collect::<Vec<&str>>();
                            let (x, y, yp) = (line[0], line[1], line[2]);
                            let (x, y, yp) = (
                                x.parse::<f64>().unwrap(),
                                y.parse::<f64>().unwrap(),
                                yp.parse::<f64>().unwrap(),
                            );

                            self.points[self.solutions_count].push((x, y, yp));
                            self.is_visible.push(true);
                        }

                        let mut flname = path.file_name().unwrap().to_str().unwrap();
                        flname = flname.trim_end_matches(".DAT");

                        self.layer_names
                            .push(format!("{} he={} gamma={}", flname, he, gamma));
                        self.is_visible.push(true);
                        self.picked_path = None;
                        self.solutions_count += 1;
                    }
                };
            }
        }

        self.dropped_files.clear();

        // Collect dropped files:
        ctx.input(|i| {
            if !i.raw.dropped_files.is_empty() {
                self.dropped_files = i.raw.dropped_files.clone();
            }
        });
    }
}
