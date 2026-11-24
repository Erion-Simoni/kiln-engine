//! KilnScript Debugger Panel — breakpoints, stepping, variable inspection.

use egui::{Color32, RichText, ScrollArea, Ui};
use kiln_ir::Debugger;

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
    /// Filter the live local view to current-frame lexical bindings.
    pub active_locals_only: bool,
    /// Show watch expressions.
    pub show_watches: bool,
    /// New breakpoint instruction input.
    pub new_breakpoint_instr: String,
    /// New watch expression input.
    pub new_watch_expr: String,
    /// Live VM target: scene or entity:ID.
    pub live_watch_owner: String,
    /// Bounded live data selector or read-only expression being authored.
    pub live_watch_selector: String,
    /// Session-only live data watches (owner, selector).
    pub live_watches: Vec<(String, String)>,
    /// Exact project-relative executable source breakpoint being authored.
    pub source_path: String,
    pub source_line: String,
    pub source_message: String,
}

impl DebuggerPanel {
    pub fn new() -> Self {
        Self {
            expanded: true,
            show_breakpoints: true,
            show_call_stack: true,
            show_locals: true,
            active_locals_only: false,
            show_watches: true,
            new_breakpoint_instr: String::new(),
            new_watch_expr: String::new(),
            live_watch_owner: "scene".into(),
            live_watch_selector: String::new(),
            live_watches: Vec::new(),
            source_path: String::new(),
            source_line: "1".into(),
            source_message: String::new(),
        }
    }

    /// Render bounded selectors and isolated expressions against actual gameplay VMs.
    pub fn render_live_watches(
        &mut self,
        ui: &mut Ui,
        session: &kiln_runtime::SliceSession,
        inspect: bool,
    ) {
        ui.collapsing("Live data watches", |ui| {
            ui.label("Owner: scene or entity:ID. Selector: slot:N, local:NAME or state:KEY, followed by /key or /index.");
            ui.add(egui::TextEdit::singleline(&mut self.live_watch_owner).char_limit(256).hint_text("scene"));
            ui.add(egui::TextEdit::singleline(&mut self.live_watch_selector).char_limit(512).hint_text("local:health"));
            if ui.add_enabled(self.live_watches.len() < 32 && !self.live_watch_selector.is_empty(), egui::Button::new("Add watch")).clicked() {
                self.live_watches.push((self.live_watch_owner.clone(), self.live_watch_selector.clone()));
                self.live_watch_selector.clear();
            }
            let mut remove = None;
            for (index, (owner, selector)) in self.live_watches.iter().enumerate() {
                ui.horizontal_wrapped(|ui| {
                    if ui.small_button("Remove").clicked() { remove = Some(index); }
                    ui.monospace(format!("{owner} / {selector}"));
                    if inspect {
                        match session.live_watch(owner, selector) {
                            Ok(value) => { ui.monospace(value); }
                            Err(error) => { ui.colored_label(Color32::LIGHT_RED, error); }
                        }
                    } else { ui.label("Pause to inspect"); }
                });
            }
            if let Some(index) = remove { self.live_watches.remove(index); }
            ui.label("Expression: expr: health * 2. Reads: slot(0), state(\"key\"), game(\"key\"). Pure calls: array_get, array_length, dict_get, dict_has. Names use allocated locals; mutations are rejected.");
        });
    }

    /// Configure executable source lines on the actual gameplay scheduler.
    pub fn render_source_breakpoints(
        &mut self,
        ui: &mut Ui,
        session: &mut kiln_runtime::SliceSession,
    ) -> bool {
        let mut added = false;
        ui.collapsing("Source breakpoints",|ui| {
            ui.label("Exact project-relative path and executable line. Saved-screen UI callbacks remain atomic.");
            ui.add(egui::TextEdit::singleline(&mut self.source_path).char_limit(256).hint_text("scripts/main.ks"));
            ui.add(egui::TextEdit::singleline(&mut self.source_line).char_limit(10).hint_text("line"));
            if ui.button("Add source breakpoint").clicked() {
                self.source_message=match self.source_line.parse::<usize>().map_err(|_|"Line must be a positive integer".to_owned()).and_then(|line|session.add_source_breakpoint(&self.source_path,line)) {
                    Ok(())=>{added=true;"Source breakpoint added".into()},Err(error)=>error,
                };
            }
            ui.label(&self.source_message);
            for point in session.source_breakpoints() {
                ui.horizontal(|ui| {
                    ui.monospace(format!("{}:{}",point.path,point.line));
                    if ui.small_button("Remove source breakpoint").clicked() {session.remove_source_breakpoint(&point.path,point.line);}
                });
            }
        });
        added
    }

    /// Render the debugger panel.
    pub fn render<H: kiln_ir::HostContext>(&mut self, ui: &mut Ui, debugger: &mut Debugger<H>) {
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
                debugger.restart();
            }
        });

        ui.add_space(8.0);

        // Current instruction display
        if let Some(instr) = debugger.current_instruction() {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(format!("IP: {}", debugger.state().current_instruction))
                        .monospace()
                        .strong(),
                );
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
        });

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

    fn render_call_stack<H: kiln_ir::HostContext>(&self, ui: &mut Ui, debugger: &mut Debugger<H>) {
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

    fn render_locals<H: kiln_ir::HostContext>(&self, ui: &mut Ui, debugger: &mut Debugger<H>) {
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

    fn render_watches<H: kiln_ir::HostContext>(&mut self, ui: &mut Ui, debugger: &mut Debugger<H>) {
        ui.heading("Watch Expressions");
        ui.add_space(4.0);

        // Add watch
        ui.horizontal(|ui| {
            ui.label("Expression:");
            ui.text_edit_singleline(&mut self.new_watch_expr);
            if ui.button("Add").clicked() && !self.new_watch_expr.is_empty() {
                debugger.add_watch(self.new_watch_expr.clone());
                self.new_watch_expr.clear();
            }
        });

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
