//! UI Layout Editor panel — visual canvas for kiln-ui widgets.

use egui::{Color32, DragValue, RichText, ScrollArea, Ui, Vec2, Stroke, Sense, Pos2, Rect, Align2};
use kiln_ui::{
    Anchor, Layout, LayoutConstraints, Rect, Size, WidgetId,
    Panel, Label, Button, List, NineSlice, NineSliceRenderer,
};

/// UI widget types for the editor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WidgetType {
    Panel,
    Label,
    Button,
    List,
}

/// A widget instance in the editor.
#[derive(Debug, Clone)]
pub struct EditorWidget {
    pub id: WidgetId,
    pub widget_type: WidgetType,
    pub layout: kiln_ui::Layout,
    pub bounds: kiln_ui::Rect,
    pub visible: bool,
    // Type-specific properties
    pub label_text: String,
    pub button_normal: Option<NineSlice>,
    pub button_hovered: Option<NineSlice>,
    pub button_pressed: Option<NineSlice>,
    pub list_items: Vec<String>,
    pub list_selected: Option<usize>,
    pub panel_background: Option<NineSlice>,
}


/// State of the UI Layout Editor panel.
#[derive(Debug, Default)]
pub struct UiLayoutEditorState {
    /// All widgets in the canvas.
    pub widgets: Vec<EditorWidget>,
    /// Selected widget index.
    pub selected_widget: Option<usize>,
    /// Canvas size.
    pub canvas_size: Size,
    /// Preview mode (play mode).
    pub preview_mode: bool,
    /// Grid snapping enabled.
    pub snap_to_grid: bool,
    /// Grid size.
    grid_size: f32,
    /// Show add widget dialog.
    pub show_add_widget_dialog: bool,
    /// New widget type to add.
    new_widget_type: WidgetType,
    /// Show widget properties panel.
    pub show_properties: bool,
    /// Drag state.
    pub dragging_widget: Option<usize>,
    pub drag_offset: Vec2,
}

impl UiLayoutEditorState {
    pub fn new() -> Self {
        Self {
            widgets: Vec::new(),
            selected_widget: None,
            canvas_size: Size::new(800.0, 600.0),
            preview_mode: false,
            snap_to_grid: true,
            grid_size: 10.0,
            show_add_widget_dialog: false,
            new_widget_type: WidgetType::Panel,
            show_properties: true,
            dragging_widget: None,
            drag_offset: Vec2::ZERO,
        }
    }

    /// Add a new widget to the canvas.
    pub fn add_widget(&mut self, widget_type: WidgetType) -> usize {
        let id = WidgetId::new();
        let widget = match widget_type {
            WidgetType::Panel => EditorWidget {
                id: WidgetId::new(),
                widget_type: WidgetType::Panel,
                layout: kiln_ui::Layout::new(),
                bounds: kiln_ui::Rect::new(100.0, 100.0, 200.0, 150.0),
                visible: true,
                label_text: String::new(),
                button_normal: None,
                button_hovered: None,
                button_pressed: None,
                list_items: Vec::new(),
                list_selected: None,
                panel_background: None,
            },
            WidgetType::Label => EditorWidget {
                id: WidgetId::new(),
                widget_type: WidgetType::Label,
                layout: kiln_ui::Layout::new(),
                bounds: kiln_ui::Rect::new(100.0, 100.0, 200.0, 30.0),
                visible: true,
                label_text: "Label".to_string(),
                button_normal: None,
                button_hovered: None,
                button_pressed: None,
                list_items: Vec::new(),
                list_selected: None,
                panel_background: None,
            },
            WidgetType::Button => EditorWidget {
                id: WidgetId::new(),
                widget_type: WidgetType::Button,
                layout: kiln_ui::Layout::new(),
                bounds: kiln_ui::Rect::new(100.0, 100.0, 120.0, 40.0),
                visible: true,
                label_text: "Button".to_string(),
                button_normal: Some(NineSlice::new(1, 8.0)),
                button_hovered: Some(NineSlice::new(2, 8.0)),
                button_pressed: Some(NineSlice::new(3, 8.0)),
                list_items: Vec::new(),
                list_selected: None,
                panel_background: None,
            },
            WidgetType::List => EditorWidget {
                id: WidgetId::new(),
                widget_type: WidgetType::List,
                layout: kiln_ui::Layout::new(),
                bounds: kiln_ui::Rect::new(100.0, 100.0, 200.0, 200.0),
                visible: true,
                label_text: String::new(),
                button_normal: None,
                button_hovered: None,
                button_pressed: None,
                list_items: vec!["Item 1".to_string(), "Item 2".to_string(), "Item 3".to_string()],
                list_selected: Some(0),
                panel_background: None,
            },
        };
        self.widgets.push(widget);
        let idx = self.widgets.len() - 1;
        self.selected_widget = Some(idx);
        idx
    }

    /// Update the preview (for play mode).
    pub fn update_preview(&mut self, _dt: f32) {
        // In play mode, could animate or update widget states
    }


    /// Render the UI Layout Editor panel.
    pub fn render(&mut self, ui: &mut Ui) {
        ui.heading("UI Layout Editor");
        ui.add_space(8.0);

        // Toolbar
        ui.horizontal(|ui| {
            if ui.button(if self.preview_mode { "⏸ Stop Preview" } else { "▶ Preview" }).clicked() {
                self.preview_mode = !self.preview_mode;
            }
            ui.separator();
            ui.checkbox(&mut self.snap_to_grid, "Snap to Grid");
            ui.add(egui::DragValue::new(&mut self.grid_size).speed(1.0).suffix("px").range(1.0..=100.0));
            ui.separator();
            if ui.button("🗑 Clear Canvas").clicked() {
                self.widgets.clear();
                self.selected_widget = None;
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(format!("Widgets: {}", self.widgets.len()));
            });
        });

        ui.add_space(8.0);

        // Widget palette
        ui.horizontal(|ui| {
            ui.heading("Widget Palette");
            for &widget_type in &[WidgetType::Panel, WidgetType::Label, WidgetType::Button, WidgetType::List] {
                if ui.button(format!("{:?}", widget_type)).clicked() {
                    self.add_widget(widget_type);
                }
            }
        );

        ui.add_space(8.0);

        // Main area: canvas + properties
        ui.columns(2, |cols| {
            // Left: Canvas
            cols[0].vertical(|ui| {
                ui.heading("Canvas");
                ui.add_space(4.0);

                let canvas_size = Vec2::new(self.canvas_size.width, self.canvas_size.height);
                let (canvas_rect, canvas_response) = ui.allocate_exact_size(canvas_size, Sense::click_and_drag());

                // Canvas background
                ui.painter().rect_filled(canvas_rect, 4.0, Color32::from_rgb(30, 30, 40));
                
                // Grid
                if self.snap_to_grid {
                    self.draw_grid(ui.painter(), canvas_rect);
                }

                // Draw widgets
                let mut clicked_widget = None;
                let mut drag_started = None;
                
                for (idx, widget) in self.widgets.iter().enumerate() {
                    if !widget.visible { continue; }
                    
                    let widget_rect = self.widget_rect_to_canvas(widget, canvas_rect);
                    
                    // Widget background
                    let bg_color = if self.selected_widget == Some(idx) {
                        Color32::from_rgb(60, 100, 180)
                    } else {
                        Color32::from_rgb(50, 50, 70)
                    };
                    ui.painter().rect_filled(widget_rect, 4.0, bg_color);
                    ui.painter().rect_stroke(widget_rect, 4.0, 
                        if self.selected_widget == Some(idx) {
                            Stroke::new(2.0, Color32::from_rgb(100, 200, 255))
                        } else {
                            Stroke::new(1.0, Color32::from_gray(100))
                        }
                    );

                    // Widget label
                    let label = match widget.widget_type {
                        WidgetType::Panel => "Panel",
                        WidgetType::Label => &widget.label_text,
                        WidgetType::Button => &widget.label_text,
                        WidgetType::List => "List",
                    };
                    ui.painter().text(
                        widget_rect.center(),
                        Align2::CENTER_CENTER,
                        label,
                        egui::FontId::proportional(12.0),
                        Color32::WHITE,
                    );

                    // Interaction
                    let widget_response = ui.interact(widget_rect, ui.id().with(widget.id.as_u64()), Sense::click_and_drag());
                    
                    if widget_response.clicked() {
                        clicked_widget = Some(idx);
                    }
                    if widget_response.drag_started() {
                        drag_started = Some((idx, widget_response.interact_pointer_pos().unwrap_or(Pos2::ZERO)));
                    }
                    if widget_response.dragged() && self.dragging_widget == Some(idx) {
                        if let Some(pos) = widget_response.interact_pointer_pos() {
                            let new_bounds = self.canvas_pos_to_widget_bounds(pos - self.drag_offset);
                            if let Some(widget) = self.widgets.get_mut(idx) {
                                widget.bounds = self.snap_bounds(new_bounds);
                            }
                        }
                    }
                }

                // Handle selection
                if let Some(idx) = clicked_widget {
                    self.selected_widget = Some(idx);
                    self.dragging_widget = Some(idx);
                    if let Some(pos) = canvas_response.interact_pointer_pos() {
                        if let Some(widget) = self.widgets.get(idx) {
                            let widget_rect = self.widget_rect_to_canvas(widget, canvas_rect);
                            self.drag_offset = pos - widget_rect.min;
                        }
                    }
                }

                // Clear drag on mouse release
                if canvas_response.drag_stopped() {
                    self.dragging_widget = None;
                }

                // Click on empty space deselects
                if canvas_response.clicked() && clicked_widget.is_none() {
                    self.selected_widget = None;
                }
            });


            // Right: Properties panel
            cols[1].vertical(|ui| {
                ui.heading("Properties");
                ui.add_space(8.0);

                if let Some(idx) = self.selected_widget {
                    if let Some(widget) = self.widgets.get_mut(idx) {
                        ui.label(format!("Widget: {:?}", widget.widget_type));
                        ui.add_space(4.0);

                        // ID
                        ui.horizontal(|ui| {
                            ui.label("ID:");
                            ui.label(format!("{:?}", widget.id.as_u64()));
                        });

                        // Visibility
                        ui.checkbox(&mut widget.visible, "Visible");

                        ui.add_space(8.0);

                        // Position & Size
                        ui.separator();
                        ui.label("Transform");
                        ui.horizontal(|ui| {
                            ui.label("X:");
                            ui.add(DragValue::new(&mut widget.bounds.x).speed(1.0).suffix("px"));
                            ui.label("Y:");
                            ui.add(DragValue::new(&mut widget.bounds.y).speed(1.0).suffix("px"));
                        });
                        ui.horizontal(|ui| {
                            ui.label("W:");
                            ui.add(DragValue::new(&mut widget.bounds.width).speed(1.0).suffix("px"));
                            ui.label("H:");
                            ui.add(DragValue::new(&mut widget.bounds.height).speed(1.0).suffix("px"));
                        });

                        ui.add_space(8.0);

                        // Layout properties
                        ui.separator();
                        ui.label("Layout");
                        
                        // Anchor
                        ui.horizontal(|ui| {
                            ui.label("Anchor:");
                            egui::ComboBox::from_label("")
                                .selected_text(format!("{:?}", widget.layout.anchor))
                                .show_ui(ui, |ui| {
                                    for &anchor in &[
                                        Anchor::TopLeft, Anchor::TopCenter, Anchor::TopRight,
                                        Anchor::CenterLeft, Anchor::Center, Anchor::CenterRight,
                                        Anchor::BottomLeft, Anchor::BottomCenter, Anchor::BottomRight,
                                    ] {
                                        ui.selectable_value(&mut widget.layout.anchor, anchor, format!("{:?}", anchor));
                                    }
                                });
                        });

                        // Flex
                        ui.horizontal(|ui| {
                            ui.label("Flex Grow:");
                            ui.add(DragValue::new(&mut widget.layout.flex_grow).speed(0.1).range(0.0..=10.0));
                            ui.label("Flex Shrink:");
                            ui.add(DragValue::new(&mut widget.layout.flex_shrink).speed(0.1).range(0.0..=10.0));
                        });

                        // Constraints
                        ui.add_space(4.0);
                        ui.label("Constraints");
                        ui.horizontal(|ui| {
                            ui.label("Min W:");
                            ui.add(DragValue::new(widget.layout.constraints.min_width.get_or_insert_mut(0.0)).speed(1.0).suffix("px"));
                            ui.label("Max W:");
                            ui.add(DragValue::new(widget.layout.constraints.max_width.get_or_insert_mut(1000.0)).speed(1.0).suffix("px"));
                        });
                        ui.horizontal(|ui| {
                            ui.label("Min H:");
                            ui.add(DragValue::new(widget.layout.constraints.min_height.get_or_insert_mut(0.0)).speed(1.0).suffix("px"));
                            ui.label("Max H:");
                            ui.add(DragValue::new(widget.layout.constraints.max_height.get_or_insert_mut(1000.0)).speed(1.0).suffix("px"));
                        });

                        // Type-specific properties
                        ui.add_space(8.0);
                        ui.separator();
                        match widget.widget_type {
                            WidgetType::Label => {
                                ui.label("Label Properties");
                                ui.text_edit_singleline(&mut widget.label_text);
                            }
                            WidgetType::Button => {
                                ui.label("Button Properties");
                                ui.text_edit_singleline(&mut widget.label_text);
                                ui.label("Normal:");
                                ui.label("Hovered:");
                                ui.label("Pressed:");
                            }
                            WidgetType::List => {
                                ui.label("List Items");
                                // Would show list editor
                            }
                            WidgetType::Panel => {
                                ui.label("Panel Properties");
                                // Background nine-slice
                            }
                        }
                    }
                } else {
                    ui.label("Select a widget to edit its properties.");
                }

                ui.add_space(8.0);
                
                // Add widget button
                if ui.button("➕ Add Widget").clicked() {
                    // Show dialog
                }
            });
        });

        // Update preview
        self.update_preview(1.0 / 60.0);
    }


    /// Draw grid on canvas.
    fn draw_grid(&self, painter: &egui::Painter, canvas_rect: Rect) {
        let grid_size = self.grid_size;
        let start_x = (canvas_rect.left() / grid_size).floor() * grid_size;
        let start_y = (canvas_rect.top() / grid_size).floor() * grid_size;

        let mut x = start_x;
        while x <= canvas_rect.right() {
            painter.line_segment(
                [Pos2::new(x, canvas_rect.top()), Pos2::new(x, canvas_rect.bottom())],
                Stroke::new(0.5, Color32::from_rgba_premultiplied(100, 100, 120, 50)),
            );
            x += grid_size;
        }

        let mut y = start_y;
        while y <= canvas_rect.bottom() {
            painter.line_segment(
                [Pos2::new(canvas_rect.left(), y), Pos2::new(canvas_rect.right(), y)],
                Stroke::new(0.5, Color32::from_rgba_premultiplied(100, 100, 120, 50)),
            );
            y += grid_size;
        }
    }

    /// Convert widget bounds to canvas coordinates.
    fn widget_rect_to_canvas(&self, widget: &EditorWidget, canvas_rect: Rect) -> Rect {
        Rect::from_min_max(
            Pos2::new(canvas_rect.left() + widget.bounds.x, canvas_rect.top() + widget.bounds.y),
            Pos2::new(canvas_rect.left() + widget.bounds.x + widget.bounds.width, 
                     canvas_rect.top() + widget.bounds.y + widget.bounds.height),
        )
    }

    /// Convert canvas position to widget bounds.
    fn canvas_pos_to_widget_bounds(&self, pos: Pos2) -> kiln_ui::Rect {
        kiln_ui::Rect::new(pos.x, pos.y, 100.0, 50.0)
    }

    /// Snap bounds to grid.
    fn snap_bounds(&self, bounds: kiln_ui::Rect) -> kiln_ui::Rect {
        if !self.snap_to_grid {
            return bounds;
        }
        let grid = self.grid_size;
        kiln_ui::Rect::new(
            (bounds.x / grid).round() * grid,
            (bounds.y / grid).round() * grid,
            (bounds.width / grid).round() * grid,
            (bounds.height / grid).round() * grid,
        )
    }
}


