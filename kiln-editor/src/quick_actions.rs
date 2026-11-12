//! Searchable commands and panel navigation for the native workspace.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum QuickAction {
    OpenProject,
    Save,
    Undo,
    Redo,
    Scene,
    Game,
    Hierarchy,
    Inspector,
    Assets,
    Console,
    Animation,
    Graph,
    Export,
    Preferences,
    ResetLayout,
}

impl QuickAction {
    const ALL: [Self; 15] = [
        Self::OpenProject,
        Self::Save,
        Self::Undo,
        Self::Redo,
        Self::Scene,
        Self::Game,
        Self::Hierarchy,
        Self::Inspector,
        Self::Assets,
        Self::Console,
        Self::Animation,
        Self::Graph,
        Self::Export,
        Self::Preferences,
        Self::ResetLayout,
    ];

    fn label(self) -> &'static str {
        match self {
            Self::OpenProject => "Open Project",
            Self::Save => "Save Project",
            Self::Undo => "Undo",
            Self::Redo => "Redo",
            Self::Scene => "Scene View",
            Self::Game => "Game View",
            Self::Hierarchy => "Hierarchy",
            Self::Inspector => "Inspector",
            Self::Assets => "Asset Browser",
            Self::Console => "Console",
            Self::Animation => "Animation Editor",
            Self::Graph => "Graph Editor",
            Self::Export => "Build / Export",
            Self::Preferences => "Preferences",
            Self::ResetLayout => "Reset Workspace Layout",
        }
    }

    fn category(self) -> &'static str {
        match self {
            Self::OpenProject | Self::Save => "Project",
            Self::Undo | Self::Redo => "Edit",
            Self::Export => "Publish",
            Self::Preferences | Self::ResetLayout => "Workspace",
            _ => "Panel",
        }
    }

    fn matches(self, query: &str) -> bool {
        let text = format!("{} {}", self.label(), self.category()).to_lowercase();
        query
            .split_whitespace()
            .all(|word| text.contains(&word.to_lowercase()))
    }
}

#[derive(Default)]
pub(crate) struct QuickActions {
    pub(crate) open: bool,
    query: String,
    selected: usize,
    focus_search: bool,
}

impl QuickActions {
    pub(crate) fn toggle(&mut self) {
        self.open = !self.open;
        self.query.clear();
        self.selected = 0;
        self.focus_search = self.open;
    }

    pub(crate) fn show(
        &mut self,
        ctx: &egui::Context,
        enabled: impl Fn(QuickAction) -> bool,
    ) -> Option<QuickAction> {
        if !self.open {
            return None;
        }
        let mut chosen = None;
        let mut dismiss = false;
        let width = (ctx.screen_rect().width() - 40.0).clamp(220.0, 540.0);
        egui::Window::new("Quick Actions")
            .id(egui::Id::new("quick_actions"))
            .anchor(egui::Align2::CENTER_TOP, [0.0, 90.0])
            .fixed_size([width, 380.0])
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                let (up, down, enter, escape) = ctx.input_mut(|input| {
                    (
                        input.consume_key(egui::Modifiers::NONE, egui::Key::ArrowUp),
                        input.consume_key(egui::Modifiers::NONE, egui::Key::ArrowDown),
                        input.consume_key(egui::Modifiers::NONE, egui::Key::Enter),
                        input.consume_key(egui::Modifiers::NONE, egui::Key::Escape),
                    )
                });
                let search = ui.add_sized(
                    [ui.available_width(), 32.0],
                    egui::TextEdit::singleline(&mut self.query)
                        .hint_text("Find a command or panel…")
                        .char_limit(128),
                );
                if self.focus_search {
                    search.request_focus();
                    self.focus_search = false;
                }
                if search.changed() {
                    self.selected = 0;
                }
                let results: Vec<_> = QuickAction::ALL
                    .into_iter()
                    .filter(|action| action.matches(&self.query))
                    .collect();
                self.selected = self.selected.min(results.len().saturating_sub(1));
                if up {
                    self.selected = self.selected.saturating_sub(1);
                }
                if down {
                    self.selected = (self.selected + 1).min(results.len().saturating_sub(1));
                }
                dismiss = escape;
                ui.add_space(6.0);
                ui.separator();
                egui::ScrollArea::vertical()
                    .max_height(280.0)
                    .show(ui, |ui| {
                        for (index, action) in results.iter().copied().enumerate() {
                            let response = ui.add_enabled(
                                enabled(action),
                                egui::Button::new(format!(
                                    "{}   /   {}",
                                    action.category(),
                                    action.label()
                                ))
                                .selected(index == self.selected)
                                .min_size(egui::vec2(ui.available_width(), 30.0)),
                            );
                            if index == self.selected && (up || down || search.changed()) {
                                response.scroll_to_me(Some(egui::Align::Center));
                            }
                            if response.clicked() {
                                chosen = Some(action);
                            }
                        }
                        if results.is_empty() {
                            ui.add_space(20.0);
                            ui.weak("No matching actions. Try ‘scene’, ‘assets’, or ‘export’.");
                        }
                    });
                if enter {
                    chosen = results
                        .get(self.selected)
                        .copied()
                        .filter(|action| enabled(*action));
                }
                ui.separator();
                ui.small("↑ ↓ Navigate     Enter Open     Esc Close");
            });
        if chosen.is_some() || dismiss {
            self.open = false;
            ctx.memory_mut(|memory| {
                if let Some(focus) = memory.focused() {
                    memory.surrender_focus(focus);
                }
            });
        }
        chosen
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn search_handles_case_multiple_terms_and_categories() {
        assert!(QuickAction::Assets.matches(" PANEL asset "));
        assert!(QuickAction::Export.matches("publish"));
        assert!(QuickAction::ResetLayout.matches("workspace layout"));
        assert!(!QuickAction::Scene.matches("scene missing"));
        assert!(QuickAction::Save.matches(""));
    }
    #[test]
    fn reopening_resets_query_and_navigation() {
        let mut palette = QuickActions::default();
        palette.toggle();
        palette.query = "missing".into();
        palette.selected = 8;
        palette.toggle();
        palette.toggle();
        assert!(palette.open && palette.focus_search);
        assert!(palette.query.is_empty());
        assert_eq!(palette.selected, 0);
    }
    fn frame(
        palette: &mut QuickActions,
        ctx: &egui::Context,
        events: Vec<egui::Event>,
        enabled: bool,
    ) -> Option<QuickAction> {
        let mut result = None;
        let _ = ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1280.0, 800.0),
                )),
                events,
                ..Default::default()
            },
            |ctx| {
                result = palette.show(ctx, |_| enabled);
            },
        );
        result
    }

    fn key(key: egui::Key) -> egui::Event {
        egui::Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        }
    }

    #[test]
    fn keyboard_search_and_enter_dispatch_the_filtered_action() {
        let ctx = egui::Context::default();
        let mut palette = QuickActions::default();
        palette.toggle();
        frame(&mut palette, &ctx, vec![], true);
        frame(
            &mut palette,
            &ctx,
            vec![egui::Event::Text("console".into())],
            true,
        );
        assert_eq!(
            frame(&mut palette, &ctx, vec![key(egui::Key::Enter)], true),
            Some(QuickAction::Console)
        );
        assert!(!palette.open);
    }

    #[test]
    fn disabled_commands_and_empty_results_do_not_dispatch() {
        let ctx = egui::Context::default();
        let mut palette = QuickActions::default();
        palette.toggle();
        frame(&mut palette, &ctx, vec![], false);
        assert_eq!(
            frame(&mut palette, &ctx, vec![key(egui::Key::Enter)], false),
            None
        );
        assert!(palette.open);
        palette.query = "no such command".into();
        assert_eq!(
            frame(
                &mut palette,
                &ctx,
                vec![key(egui::Key::ArrowDown), key(egui::Key::Enter)],
                true
            ),
            None
        );
        frame(&mut palette, &ctx, vec![key(egui::Key::Escape)], true);
        assert!(!palette.open);
    }

    #[test]
    fn arrow_navigation_works_while_search_has_focus() {
        let ctx = egui::Context::default();
        let mut palette = QuickActions::default();
        palette.toggle();
        frame(&mut palette, &ctx, vec![], true);
        frame(&mut palette, &ctx, vec![key(egui::Key::ArrowDown)], true);
        assert_eq!(palette.selected, 1);
        assert_eq!(
            frame(&mut palette, &ctx, vec![key(egui::Key::Enter)], true),
            Some(QuickAction::Save)
        );
    }
}
