//! KilnScript Debugger Panel — breakpoints, stepping, variable inspection.

use egui::{Color32, RichText, ScrollArea, Ui, Vec2};
use kiln_ir::{Breakpoint, DebugEvent, Debugger, DebugState, StepMode, WatchExpression};

/// State of the debugger panel.
#[derive(Debug, Default)]
pub struct DebuggerPanel {
    /// Whether the panel is expanded.
    pub expanded: bool,
    /// Show breakpoints list.
    pub show_breakpoints: bool,
    /// Show call stack.
    pub show_call_stack: bool,
    /// Show local variables.
    pub show_locals: bool,
    /// Show watch expressions.
    pub show_watches: bool,
    /// New breakpoint instruction input.
    pub new_breakpoint_instr: String,
    /// New watch expression input.
    pub new_watch_expr: String,
}

impl DebuggerPanel {
    pub fn new() -> Self {
        Self {
            expanded: true,
            show_breakpoints: true,
            show_call_stack: true,
            show_locals: true,
            show_watches: true,
            new_breakpoint_instr: String::new(),
            new_watch_expr: String::new(),
        }
    }


    /// Render the debugger panel.
    pub fn render<H: kiln_ir::HostContext>(
        &mut self,
        ui: &mut Ui,
        debugger: &mut Debugger<H>,
    ) {
        ui.heading("KilnScript Debugger");
        ui.add_space(8.0);

        // Debug controls
        ui.horizontal(|ui| {
            if ui.button("▶ Continue").clicked() {
                debugger.run();
            }
            if ui.button("⏸ Break").clicked() {
                debugger.set_step_mode(kiln_ir::StepMode::StepInto);
            }
            if ui.button("⏭ Step Into").clicked() {
                debugger.step_into();
            }
            if ui.button("⏭ Step Over").clicked() {
                debugger.step_over();
            }
            if ui.button("⏮ Step Out").clicked() {
                debugger.step_out();
            }
            ui.separator();
            if ui.button("Restart").clicked() {
                // Restart would need to reset VM
            }
        });

        ui.add_space(8.0);

        // Current instruction display
        if let Some(instr) = debugger.current_instruction() {
            ui.horizontal(|ui| {
                ui.label(RichText::new(format!("IP: {}", debugger.state().current_instruction)).monospace().strong());
                ui.label(RichText::new(format!("{:?}", instr)).monospace());
            });
        } else {
            ui.label("No instruction (program not running or completed)");
        }

        ui.add_space(8.0);

        // Tabs for different debugger views
        egui::TopBottomPanel::bottom("debugger_tabs").show_inside(ui, |ui| {
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.show_breakpoints, true, "Breakpoints");
                ui.selectable_value(&mut self.show_call_stack, true, "Call Stack");
                ui.selectable_value(&mut self.show_locals, true, "Locals");
                ui.selectable_value(&mut self.show_watches, true, "Watches");
            });
        });

        ui.add_space(4.0);

        if self.show_breakpoints {
            self.render_breakpoints(ui, debugger);
        }

        if self.show_call_stack {
            self.render_call_stack(ui, debugger);
        }

        if self.show_locals {
            self.render_locals(ui, debugger);
        }

        if self.show_watches {
            self.render_watches(ui, debugger);
        }
    }


    fn render_breakpoints<H: kiln_ir::HostContext>(
        &mut self,
        ui: &mut Ui,
        debugger: &mut Debugger<H>,
    ) {
        ui.heading("Breakpoints");
        ui.add_space(4.0);

        // Add breakpoint
        ui.horizontal(|ui| {
            ui.label("Instruction:");
            ui.text_edit_singleline(&mut self.new_breakpoint_instr);
            if ui.button("Add").clicked() {
                if let Ok(instr) = self.new_breakpoint_instr.parse::<usize>() {
                    debugger.set_breakpoint(instr, None);
                    self.new_breakpoint_instr.clear();
                }
            }
        );

        ui.add_space(4.0);

        let breakpoints: Vec<_> = debugger.state().breakpoints.values().cloned().collect();
        if breakpoints.is_empty() {
            ui.label("No breakpoints set.");
        } else {
            ScrollArea::vertical().max_height(200.0).show(ui, |ui| {
                egui::Grid::new("breakpoints_grid")
                    .num_columns(5)
                    .striped(true)
                    .show(ui, |ui| {
                        ui.label(RichText::new("Instruction").strong());
                        ui.label(RichText::new("Enabled").strong());
                        ui.label(RichText::new("Condition").strong());
                        ui.label(RichText::new("Hits").strong());
                        ui.label(RichText::new("Actions").strong());
                        ui.end_row();

                        for bp in breakpoints {
                            let mut enabled = bp.enabled;
                            if ui.checkbox(&mut enabled, "").changed() {
                                // Toggle breakpoint
                                debugger.toggle_breakpoint(bp.instruction);
                            }
                            ui.label(bp.condition.as_deref().unwrap_or("(none)"));
                            ui.label(bp.hit_count.to_string());
                            if ui.button("Remove").clicked() {
                                debugger.remove_breakpoint(bp.instruction);
                            }
                            ui.end_row();
                        }
                    });
            });
        }
    }


    fn render_call_stack<H: kiln_ir::HostContext>(
        &self,
        ui: &mut Ui,
        debugger: &mut Debugger<H>,
    ) {
        ui.heading("Call Stack");
        ui.add_space(4.0);

        let call_stack = debugger.vm().call_stack();
        if call_stack.is_empty() {
            ui.label("Call stack empty");
        } else {
            for (i, &return_addr) in call_stack.iter().enumerate().rev() {
                ui.label(format!("#{}  Return IP: {}", i, return_addr));
            }
        }
    }

    fn render_locals<H: kiln_ir::HostContext>(
        &self,
        ui: &mut Ui,
        debugger: &mut Debugger<H>,
    ) {
        ui.heading("Local Variables");
        ui.add_space(4.0);

        let locals = debugger.vm().locals();
        if locals.is_empty() {
            ui.label("No local variables");
        } else {
            ScrollArea::vertical().max_height(200.0).show(ui, |ui| {
                egui::Grid::new("locals_grid")
                    .num_columns(2)
                    .striped(true)
                    .show(ui, |ui| {
                        ui.label(RichText::new("Slot").strong());
                        ui.label(RichText::new("Value").strong());
                        ui.end_row();

                        for (i, value) in locals.iter().enumerate() {
                            ui.label(format!("{}", i));
                            ui.label(format!("{:?}", value));
                            ui.end_row();
                        }
                    });
            });
        }
    }

    fn render_watches<H: kiln_ir::HostContext>(
        &mut self,
        ui: &mut Ui,
        debugger: &mut Debugger<H>,
    ) {
        ui.heading("Watch Expressions");
        ui.add_space(4.0);

        // Add watch
        ui.horizontal(|ui| {
            ui.label("Expression:");
            ui.text_edit_singleline(&mut self.new_watch_expr);
            if ui.button("Add").clicked() {
                if !self.new_watch_expr.is_empty() {
                    debugger.add_watch(self.new_watch_expr.clone());
                    self.new_watch_expr.clear();
                }
            }
        );

        ui.add_space(4.0);

        let watches = debugger.state().watch_expressions.clone();
        if watches.is_empty() {
            ui.label("No watch expressions");
        } else {
            ScrollArea::vertical().max_height(150.0).show(ui, |ui| {
                for (i, watch) in watches.iter().enumerate() {
                    ui.horizontal(|ui| {
                        ui.label(format!("#{}", i));
                        ui.label(RichText::new(&watch.expression).monospace());
                        if let Some(val) = &watch.value {
                            ui.label(format!("= {:?}", val));
                        } else if let Some(err) = &watch.error {
                            ui.label(RichText::new(format!("Error: {}", err)).color(Color32::RED));
                        } else {
                            ui.label(RichText::new("Evaluating...").color(Color32::GRAY));
                        }
                        if ui.small_button("Remove").clicked() {
                            debugger.remove_watch(i);
                        }
                        if ui.small_button("Refresh").clicked() {
                            // Would re-evaluate
                        }
                    });
                }
            });
        }
    }
}


