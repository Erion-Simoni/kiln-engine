//! Profiler panel — GPU pass timings, frame budget overlay.

use egui::{Color32, RichText, ScrollArea, Ui, Vec2};
use kiln_render::{GpuProfiler, PassTiming};

/// State of the profiler panel.
#[derive(Debug, Default)]
pub struct ProfilerPanel {
    /// Whether the panel is expanded.
    pub expanded: bool,
    /// Show GPU timings.
    pub show_gpu: bool,
    /// Show frame budget indicator.
    pub show_budget: bool,
    /// Target frame time in ms (e.g., 16.67 for 60 FPS).
    pub target_frame_ms: f64,
    /// History of frame times for graph.
    pub frame_time_history: Vec<f64>,
    /// Max history length.
    pub max_history: usize,
}

impl ProfilerPanel {
    pub fn new() -> Self {
        Self {
            expanded: true,
            show_gpu: true,
            show_budget: true,
            target_frame_ms: 16.67,
            frame_time_history: Vec::new(),
            max_history: 120,
        }
    }

    /// Update with latest profiler data.
    pub fn update(&mut self, profiler: &GpuProfiler) {
        if self.show_budget {
            // Add current frame time to history
            let frame_time_ms = profiler.timings().iter()
                .map(|t| t.duration_ms())
                .sum();
            self.frame_time_history.push(frame_time_ms);
            if self.frame_time_history.len() > self.max_history {
                self.frame_time_history.remove(0);
            }
        }
    }


    /// Render the profiler panel.
    pub fn render(&mut self, ui: &mut Ui, profiler: &GpuProfiler) {
        ui.heading("Profiler");
        ui.add_space(8.0);

        // Controls
        ui.horizontal(|ui| {
            ui.checkbox(&mut self.show_gpu, "GPU Pass Timings");
            ui.checkbox(&mut self.show_budget, "Frame Budget");
            ui.add_space(16.0);
            ui.label("Target:");
            ui.add(egui::DragValue::new(&mut self.target_frame_ms).speed(0.1).suffix(" ms"));
        });

        ui.add_space(8.0);

        if self.show_gpu && profiler.supported() {
            ui.separator();
            ui.label(RichText::new("GPU Pass Timings").strong());
            ui.add_space(4.0);

            let timings = profiler.timings();
            if timings.is_empty() {
                ui.label("No timing data available yet. Run a frame first.");
            } else {
                ScrollArea::vertical().max_height(200.0).show(ui, |ui| {
                    egui::Grid::new("gpu_timings_grid")
                        .num_columns(3)
                        .striped(true)
                        .show(ui, |ui| {
                            ui.label(RichText::new("Pass").strong());
                            ui.label(RichText::new("Time (ms)").strong());
                            ui.label(RichText::new("% of Frame").strong());
                            ui.end_row();

                            let total_ms: f64 = timings.iter().map(|t| t.duration_ms()).sum();
                            
                            for timing in timings {
                                let ms = timing.duration_ms();
                                let pct = if total_ms > 0.0 { (ms / total_ms) * 100.0 } else { 0.0 };
                                
                                ui.label(&timing.name);
                                ui.label(format!("{:.3}", ms));
                                ui.label(format!("{:.1}%", pct));
                                ui.end_row();
                            }
                            
                            ui.label(RichText::new("Total").strong());
                            ui.label(RichText::new(format!("{:.3}", total_ms)).strong());
                            ui.label(RichText::new("100.0%").strong());
                            ui.end_row();
                        });
                });
            }
        }


        if self.show_budget {
            ui.separator();
            ui.label(RichText::new("Frame Budget").strong());
            ui.add_space(4.0);

            let total_ms: f64 = profiler.timings().iter().map(|t| t.duration_ms()).sum();
            let budget_pct = if self.target_frame_ms > 0.0 { (total_ms / self.target_frame_ms) * 100.0 } else { 0.0 };

            // Budget bar
            let budget_color = if budget_pct > 100.0 {
                Color32::RED
            } else if budget_pct > 80.0 {
                Color32::YELLOW
            } else {
                Color32::GREEN
            };

            ui.horizontal(|ui| {
                ui.label(format!("Frame: {:.2} ms", total_ms));
                ui.label(format!("Target: {:.2} ms", self.target_frame_ms));
                ui.label(RichText::new(format!("{:.1}%", budget_pct)).color(budget_color).strong());
            });

            // Visual budget bar
            let bar_width = ui.available_width().min(300.0);
            let bar_height = 20.0;
            let (rect, _) = ui.allocate_exact_size(Vec2::new(bar_width, bar_height), egui::Sense::hover());
            
            // Background
            ui.painter().rect_filled(rect, 4.0, Color32::from_rgb(40, 40, 50));
            
            // Budget used
            let used_width = (bar_width * (budget_pct / 100.0).min(1.0)) as f32;
            let used_rect = egui::Rect::from_min_size(rect.min, Vec2::new(used_width, bar_height));
            ui.painter().rect_filled(used_rect, 4.0, budget_color);
            
            // Target line
            let target_x = rect.left() + (bar_width * (100.0 / (budget_pct.max(100.0)))) as f32;
            if target_x < rect.right() {
                ui.painter().line_segment(
                    [egui::pos2(target_x, rect.top()), egui::pos2(target_x, rect.bottom())],
                    egui::Stroke::new(2.0, Color32::WHITE),
                );
            }
            
            // Border
            ui.painter().rect_stroke(rect, 4.0, egui::Stroke::new(1.0, Color32::from_gray(100)));

            ui.add_space(4.0);

            // Frame time history graph
            if !self.frame_time_history.is_empty() {
                ui.label("Frame Time History (last 120 frames):");
                let graph_height = 100.0;
                let graph_width = ui.available_width().min(400.0);
                let graph_rect = egui::Rect::from_min_size(
                    ui.cursor().left_top(),
                    Vec2::new(graph_width, graph_height),
                );
                ui.allocate_rect(graph_rect, egui::Sense::hover());

                // Graph background
                ui.painter().rect_filled(graph_rect, 2.0, Color32::from_rgb(30, 30, 35));
                ui.painter().rect_stroke(graph_rect, 2.0, egui::Stroke::new(1.0, Color32::from_gray(80)));

                // Target line
                let target_y = graph_rect.bottom() - (graph_height * (self.target_frame_ms / self.max_frame_time())) as f32;
                ui.painter().line_segment(
                    [egui::pos2(graph_rect.left(), target_y), egui::pos2(graph_rect.right(), target_y)],
                    egui::Stroke::new(1.0, Color32::from_rgb(100, 200, 100)),
                );

                // Frame time bars
                let bar_width = graph_width / self.frame_time_history.len() as f32;
                for (i, &frame_ms) in self.frame_time_history.iter().enumerate() {
                    let x = graph_rect.left() + i as f32 * bar_width;
                    let bar_h = (graph_height * (frame_ms / self.max_frame_time())) as f32;
                    let color = if frame_ms > self.target_frame_ms {
                        Color32::RED
                    } else if frame_ms > self.target_frame_ms * 0.8 {
                        Color32::YELLOW
                    } else {
                        Color32::GREEN
                    };
                    let bar_rect = egui::Rect::from_min_size(
                        egui::pos2(x, graph_rect.bottom() - bar_h),
                        Vec2::new(bar_width.max(1.0), bar_h),
                    );
                    ui.painter().rect_filled(bar_rect, 1.0, color);
                }
            }
        }

        if !profiler.supported() {
            ui.separator();
            ui.label(RichText::new("GPU timestamp queries not supported on this hardware/driver.").color(Color32::YELLOW));
        }
    }

    fn max_frame_time(&self) -> f64 {
        self.frame_time_history.iter().copied().fold(self.target_frame_ms * 2.0, f64::max)
    }
}


