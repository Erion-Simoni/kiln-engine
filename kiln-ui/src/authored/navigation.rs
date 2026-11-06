use super::*;
impl UiRuntime {
    pub(super) fn repair_focus(&mut self) {
        let Some(screen) = self.document.screens.get(&self.active) else {
            self.focus = None;
            return;
        };
        let rows = self.resolved();
        let available: Vec<_> = rows
            .iter()
            .filter(|r| r.visible && r.enabled && screen.widgets[r.index].kind == UiKind::Button)
            .map(|r| screen.widgets[r.index].id.clone())
            .collect();
        if !self.focus.as_ref().is_some_and(|id| available.contains(id)) {
            self.focus = available
                .iter()
                .find(|id| **id == screen.default_focus)
                .or_else(|| available.first())
                .cloned();
        }
        if self
            .pressed
            .as_ref()
            .is_some_and(|id| !available.contains(id))
        {
            self.pressed = None;
        }
    }
    pub(super) fn navigate(&mut self, direction: &str) {
        let Some(screen) = self.document.screens.get(&self.active) else {
            return;
        };
        let rows = self.resolved();
        let buttons: Vec<_> = rows
            .iter()
            .filter(|r| r.visible && r.enabled && screen.widgets[r.index].kind == UiKind::Button)
            .collect();
        let Some(current) = buttons
            .iter()
            .position(|r| Some(screen.widgets[r.index].id.as_str()) == self.focus.as_deref())
        else {
            return;
        };
        if let Some(target) = screen.widgets[buttons[current].index]
            .navigation
            .get(direction)
        {
            if buttons
                .iter()
                .any(|r| screen.widgets[r.index].id == *target)
            {
                self.focus = Some(target.clone());
                return;
            }
        }
        let center = |r: &ResolvedWidget| [r.bounds.center_x(), r.bounds.center_y()];
        let source = center(buttons[current]);
        let target = if ["next", "previous"].contains(&direction) {
            Some(
                (current
                    + if direction == "previous" {
                        buttons.len() - 1
                    } else {
                        1
                    })
                    % buttons.len(),
            )
        } else {
            buttons
                .iter()
                .enumerate()
                .filter_map(|(index, row)| {
                    let point = center(row);
                    let dx = point[0] - source[0];
                    let dy = point[1] - source[1];
                    let (along, across) = match direction {
                        "up" => (-dy, dx),
                        "down" => (dy, dx),
                        "left" => (-dx, dy),
                        _ => (dx, dy),
                    };
                    (along > 0.01).then_some((index, along + across.abs() * 3.0))
                })
                .min_by(|a, b| a.1.total_cmp(&b.1))
                .map(|(index, _)| index)
        };
        if let Some(index) = target {
            self.focus = Some(screen.widgets[buttons[index].index].id.clone());
        }
    }
}
