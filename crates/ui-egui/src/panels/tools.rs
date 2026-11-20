use super::edit::{actions_menu, edit_panel, stamp_palette};
use super::{
    Availability, Color32, CornerRadius, EasyPdfApp, LeftPanel, Rect, Sense, Stroke, TOOL_GROUPS, Tokens, ToolGroup, catalog, icons, theme, vec2,
    widgets,
};

const COLLAPSED_TOOLS: usize = 14;

#[derive(Clone, Copy)]
struct ToolDrag(&'static str);

#[derive(Clone, Copy)]
struct MenuItemDrag {
    group: &'static str,
    section: usize,
    command: &'static str,
}

pub(crate) fn normalize_menu_tool_order(order: &mut Vec<String>) {
    let defaults: Vec<String> = TOOL_GROUPS
        .iter()
        .flat_map(|group| group.sections.iter())
        .flat_map(|section| section.items.iter())
        .filter(|item| !matches!(item.availability, Availability::Planned(_)))
        .map(|item| item.command.to_owned())
        .collect();
    let mut normalized = Vec::with_capacity(defaults.len());
    for command in order.iter().take(defaults.len()) {
        if defaults.contains(command) && !normalized.contains(command) {
            normalized.push(command.clone());
        }
    }
    for command in defaults {
        if !normalized.contains(&command) {
            normalized.push(command);
        }
    }
    *order = normalized;
}

pub(super) fn move_menu_item(order: &mut Vec<String>, dragged: &str, target: &str, after: bool) {
    normalize_menu_tool_order(order);
    let Some(from) = order.iter().position(|command| command == dragged) else { return };
    let moved = order.remove(from);
    let Some(mut to) = order.iter().position(|command| command == target) else {
        order.insert(from.min(order.len()), moved);
        return;
    };
    if after {
        to = to.saturating_add(1);
    }
    order.insert(to.min(order.len()), moved);
}

fn group_is_visible(group: &ToolGroup) -> bool {
    !matches!(group.availability, Availability::Planned(_))
        || group.sections.iter().flat_map(|section| section.items).any(|item| !matches!(item.availability, Availability::Planned(_)))
}

pub(crate) fn normalize_tool_order(order: &mut Vec<String>) {
    let mut normalized = Vec::with_capacity(TOOL_GROUPS.len());
    for id in order.iter().take(TOOL_GROUPS.len()) {
        if catalog::group(id).is_some() && !normalized.contains(id) {
            normalized.push(id.clone());
        }
    }
    for group in TOOL_GROUPS {
        if !normalized.iter().any(|id| id == group.id) {
            normalized.push(group.id.to_owned());
        }
    }
    *order = normalized;
}

fn move_tool(order: &mut Vec<String>, dragged: &str, target: &str, after: bool) {
    normalize_tool_order(order);
    let Some(from) = order.iter().position(|id| id == dragged) else { return };
    let moved = order.remove(from);
    let Some(mut to) = order.iter().position(|id| id == target) else {
        order.insert(from.min(order.len()), moved);
        return;
    };
    if after {
        to = to.saturating_add(1);
    }
    order.insert(to.min(order.len()), moved);
}

pub(crate) fn normalize_recommended_tools(recommended: &mut Vec<String>) {
    let mut normalized = Vec::with_capacity(recommended.len().min(TOOL_GROUPS.len()));
    for id in recommended.iter().take(TOOL_GROUPS.len().saturating_add(200)) {
        let known_command = TOOL_GROUPS.iter().flat_map(|group| group.sections.iter()).flat_map(|section| section.items.iter()).any(|item| {
            (item.command == id || item.actions.iter().any(|action| *action == id)) && !matches!(item.availability, Availability::Planned(_))
        });
        if (catalog::group(id).is_some_and(group_is_visible) || known_command) && !normalized.contains(id) {
            normalized.push(id.clone());
        }
    }
    *recommended = normalized;
}

fn hue(g: &ToolGroup) -> Color32 {
    Color32::from_rgb(g.hue[0], g.hue[1], g.hue[2])
}

pub fn left_panel(app: &mut EasyPdfApp, ui: &mut egui::Ui) {
    let t = Tokens::get(ui.ctx());
    // Organize closed another way (the grid's Close, or a page opened from it) closes the tool too,
    // as its panel's Close does, back to the workspace's own panel.
    if app.left == LeftPanel::Tool("organize") && app.active.and_then(|i| app.views.get(i)).is_some_and(|v| !v.organize) {
        app.left = crate::organize_panel::workspace_panel(app.mode);
    }
    egui::Frame::NONE.fill(t.panel).inner_margin(egui::Margin { left: 14, right: 14, top: 12, bottom: 10 }).stroke(Stroke::new(1.0, t.divider)).show(
        ui,
        |ui| match app.left {
            // Organize pages has its own panel whenever the grid shows (with a document open).
            _ if app.active.is_some() && crate::organize_panel::showing(app) => crate::organize_panel::panel(app, ui, &t),
            LeftPanel::AllTools => all_tools(app, ui, &t),
            LeftPanel::Tool(id) => match catalog::group(id) {
                Some(g) => tool_detail(app, ui, &t, g),
                None => app.left = LeftPanel::AllTools,
            },
            LeftPanel::CalculationOrder => crate::calc_order_ui::panel(app, ui, &t),
        },
    );
}

#[cfg(test)]
#[allow(clippy::items_after_test_module, reason = "Tool-order regression tests are replayed beside their helpers")]
mod ordering_tests {
    use super::*;

    #[test]
    fn saved_orders_drop_unknown_and_duplicate_ids() {
        let mut groups = vec!["edit".to_owned(), "edit".to_owned(), "unknown".to_owned()];
        normalize_tool_order(&mut groups);
        assert_eq!(groups.first().map(String::as_str), Some("edit"));
        assert_eq!(groups.iter().filter(|id| id.as_str() == "edit").count(), 1);
        assert!(!groups.iter().any(|id| id == "unknown"));

        let mut recommended = vec!["optimize.advanced".to_owned(), "unknown".to_owned(), "optimize.advanced".to_owned()];
        normalize_recommended_tools(&mut recommended);
        assert_eq!(recommended, ["optimize.advanced"]);
    }

    #[test]
    fn tool_and_menu_moves_keep_every_entry() {
        let mut groups = Vec::new();
        normalize_tool_order(&mut groups);
        let count = groups.len();
        move_tool(&mut groups, "edit", "organize", true);
        assert_eq!(groups.len(), count);
        assert_eq!(groups.iter().filter(|id| id.as_str() == "edit").count(), 1);

        let mut commands = Vec::new();
        normalize_menu_tool_order(&mut commands);
        let count = commands.len();
        move_menu_item(&mut commands, "optimize.advanced", "optimize.reduce", false);
        assert_eq!(commands.len(), count);
        assert_eq!(commands.iter().filter(|id| id.as_str() == "optimize.advanced").count(), 1);
    }
}

/// A panel's title row: a leading button (`lead`: its icon and tooltip) when given, the title
/// and Close. Returns whether the leading button and Close were clicked.
pub(crate) fn panel_header(ui: &mut egui::Ui, t: &Tokens, title: &str, lead: Option<(&str, &str)>) -> (bool, bool) {
    let mut go_back = false;
    let mut close = false;
    ui.horizontal(|ui| {
        if let Some((icon, tip)) = lead
            && icons::button(ui, icon, 26.0, false, tip).clicked()
        {
            go_back = true;
        }
        // Reserve space for Close before laying out the translated title. Long tool names
        // must never expand the fixed-width sidebar or push the Close control off-screen.
        let title_width = (ui.available_width() - 26.0 - ui.spacing().item_spacing.x).max(1.0);
        ui.add_sized([title_width, 26.0], egui::Label::new(egui::RichText::new(tl!(title)).font(theme::semibold(15.5)).color(t.text)).truncate())
            .on_hover_text(tl!(title));
        if icons::button(ui, "x", 26.0, false, tl!("Close panel")).clicked() {
            close = true;
        }
    });
    ui.add_space(4.0);
    ui.separator();
    ui.add_space(6.0);
    (go_back, close)
}

/// Width kept free at the right of a scrolling list for the floating scrollbar.
const SCROLLBAR_GUTTER: f32 = 18.0;

/// The scrollbar floats over the right edge: keeps the rows of the scroll area `ui` is in (and
/// their rounded hover wash) clear of it.
pub(super) fn clear_of_scrollbar(ui: &mut egui::Ui) {
    ui.set_max_width((ui.available_width() - SCROLLBAR_GUTTER).max(0.0));
}

/// The All tools sidebar: Expand (the full view) and Close, then the tool list, the first few
/// tools until View more shows them all.
fn all_tools(app: &mut EasyPdfApp, ui: &mut egui::Ui, t: &Tokens) {
    // Labels distinct from the Combine grid's Expand and Collapse, which can show beside them.
    let (expand, close) = panel_header(ui, t, "All tools", Some(("maximize-2", tl!("Expand all tools"))));
    if expand {
        app.expand_all_tools();
    }
    if close {
        app.left_open = false;
    }
    egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
        clear_of_scrollbar(ui);
        ui.add_space(2.0);
        normalize_tool_order(&mut app.tool_order);
        let available: Vec<String> = app.tool_order.iter().filter(|id| catalog::group(id).is_some_and(group_is_visible)).cloned().collect();
        let shown = if app.all_tools_expanded { available.len() } else { COLLAPSED_TOOLS.min(available.len()) };
        ui.label(egui::RichText::new(tl!("Drag to reorder")).font(theme::regular(11.5)).color(t.text_faint));
        let mut reorder = None;
        let mut recommendation = None;
        for id in available.into_iter().take(shown) {
            let Some(g) = catalog::group(&id) else { continue };
            let (resp, pin_clicked) = tool_row(ui, t, g, app.is_tool_pinned(g.id));
            if resp.drag_started() {
                egui::DragAndDrop::set_payload(ui.ctx(), ToolDrag(g.id));
            }
            let after = resp.ctx.pointer_latest_pos().is_some_and(|pointer| pointer.y > resp.rect.center().y);
            if let Some(dragged) = resp.dnd_hover_payload::<ToolDrag>()
                && dragged.0 != g.id
            {
                let y = if after { resp.rect.bottom() } else { resp.rect.top() };
                ui.painter().hline(resp.rect.x_range(), y, Stroke::new(2.5, t.accent));
            }
            if let Some(dragged) = resp.dnd_release_payload::<ToolDrag>()
                && dragged.0 != g.id
            {
                reorder = Some((dragged.0, g.id, after));
            }
            let recommended = app.recommended_tools.iter().any(|item| item == g.id);
            resp.context_menu(|ui| {
                let action = if recommended { tl!("Remove") } else { tl!("Add") };
                if ui.button(format!("{action}: {}", tl!("Recommended tools"))).clicked() {
                    recommendation = Some((g.id, !recommended));
                    ui.close();
                }
            });
            if pin_clicked {
                app.toggle_pin_tool(g.id);
            } else if resp.clicked() {
                app.select_tool(g.id);
            }
        }
        if let Some((dragged, target, after)) = reorder {
            move_tool(&mut app.tool_order, dragged, target, after);
            if let Some(changed) = app.tool_order_changed.as_mut() {
                changed(app.tool_order.clone());
            }
        }
        if let Some((id, add)) = recommendation {
            if add && !app.recommended_tools.iter().any(|saved| saved == id) {
                app.recommended_tools.push(id.to_owned());
            } else if !add {
                app.recommended_tools.retain(|saved| saved != id);
            }
            normalize_recommended_tools(&mut app.recommended_tools);
            if let Some(changed) = app.recommended_tools_changed.as_mut() {
                changed(app.recommended_tools.clone());
            }
        }
        ui.add_space(4.0);
        let more = if app.all_tools_expanded { "View less" } else { "View more" };
        if ui
            .add(
                egui::Label::new(egui::RichText::new(tl!(more)).color(t.accent_text).font(theme::medium(13.0)))
                    .selectable(false)
                    .sense(Sense::click()),
            )
            .on_hover_cursor(egui::CursorIcon::PointingHand)
            .clicked()
        {
            app.all_tools_expanded = !app.all_tools_expanded;
        }
    });
}

/// Draw a one-line row label without painting into the status chip or beyond the sidebar.
/// Accessibility retains the complete name through the row's WidgetInfo.
pub(super) fn row_label(ui: &mut egui::Ui, text: &str, font: egui::FontId, color: Color32, left_center: egui::Pos2, max_width: f32) -> bool {
    let mut job = egui::text::LayoutJob::simple_singleline(text.to_owned(), font, color);
    job.wrap = egui::text::TextWrapping { max_width: max_width.max(1.0), max_rows: 1, break_anywhere: true, ..Default::default() };
    let galley = ui.fonts_mut(|f| f.layout_job(job));
    let elided = galley.elided;
    ui.painter().galley(left_center - vec2(0.0, galley.size().y / 2.0), galley, color);
    elided
}

/// Draw one tool-group row with its icon, label, badge and availability hint.
fn tool_row(ui: &mut egui::Ui, t: &Tokens, g: &ToolGroup, is_pinned: bool) -> (egui::Response, bool) {
    let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 34.0), Sense::click_and_drag());
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, tl!(g.label)));
    let press = theme::Press::track(ui, &resp);
    press.wash(ui, rect, 6, false);
    let body = rect.translate(press.offset());
    icons::paint(ui, Rect::from_min_size(body.min + vec2(6.0, 7.0), vec2(20.0, 20.0)), g.icon, 19.0, hue(g));

    let pin_rect = Rect::from_center_size(body.right_center() - vec2(14.0, 0.0), vec2(22.0, 22.0));
    let pin_resp = ui.interact(pin_rect, ui.scope_id().with(("pin_toggle_row", g.id)), Sense::click());
    let pin_hovered = pin_resp.hovered();
    let mut pin_clicked = false;

    let pin_space = 26.0;
    let right_reserved = pin_space + 8.0;
    let clipped =
        row_label(ui, tl!(g.label), theme::regular(13.5), t.text, body.left_center() + vec2(36.0, 0.0), rect.width() - 36.0 - right_reserved);

    let row_hovered = ui.rect_contains_pointer(rect) || resp.hovered();
    if is_pinned || row_hovered || pin_hovered {
        if pin_hovered {
            ui.painter().rect_filled(pin_rect, CornerRadius::same(4), t.pressed);
        }
        let pin_tint = if is_pinned {
            t.accent_text
        } else if pin_hovered {
            t.text
        } else {
            t.text_faint
        };
        icons::paint(ui, pin_rect.shrink(3.0), "pin", 15.0, pin_tint);
        let tip = if is_pinned { tl!("Unpin from Home") } else { tl!("Pin to Home") };
        pin_resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, tip));
        if theme::hand(pin_resp).on_hover_text(tip).clicked() {
            pin_clicked = true;
        }
    }

    let tip = match g.availability {
        Availability::Ready => tl!("Available").to_string(),
        Availability::Planned(_) => tl!("Some features are not available yet.").to_owned(),
        Availability::Provider => tl!("Optional: needs an AI provider you configure").into(),
    };
    let tip = if clipped { format!("{} — {tip}", tl!(g.label)) } else { tip };
    (theme::hand(resp).on_hover_text(tip), pin_clicked)
}

/// Draw the detail view for one tool group, dispatching its commands and tool-specific panels.
fn tool_detail(app: &mut EasyPdfApp, ui: &mut egui::Ui, t: &Tokens, g: &'static ToolGroup) {
    if g.id == "edit" {
        edit_panel(app, ui, t, g);
        return;
    }
    let (back, close) = panel_header(ui, t, g.label, Some(("chevron-left", tl!("Back to all tools"))));
    if back {
        app.left = LeftPanel::AllTools;
        app.enter_mode(crate::Mode::AllTools);
    }
    if close {
        app.left_open = false;
    }
    if g.id == "measure" {
        crate::measure_ui::panel(app, ui, t);
        return;
    }
    // Add a stamp: the palette.
    if g.id == "stamp" {
        stamp_palette(app, ui, t);
        return;
    }
    // Prepare a form: Preview fills the form as a reader would; Edit goes back.
    if g.id == "form" {
        ui.horizontal(|ui| {
            let label = if app.form_preview { "Edit fields" } else { "Preview" };
            if widgets::pill_button(ui, tl!(label), app.form_preview).on_hover_text(tl!("Try the form as people filling it in will see it")).clicked()
            {
                app.form_preview = !app.form_preview;
                if app.form_preview {
                    app.quick_tool = crate::QuickTool::Select;
                }
            }
        });
        ui.add_space(6.0);
    }
    let mut run = None;
    let mut reorder = None;
    let mut recommendation = None;
    normalize_menu_tool_order(&mut app.menu_tool_order);
    // Redact a PDF has Acrobat's footer: Clear all / Redact all.
    let footer = g.id == "redact";
    let marks = if footer { app.active_ids().and_then(|(_, id)| app.session.get(id)).map_or(0, |d| d.redaction_marks()) } else { 0 };
    let list_h = if footer { (ui.available_height() - 52.0).max(80.0) } else { ui.available_height() };
    egui::ScrollArea::vertical().auto_shrink([false, false]).max_height(list_h).show(ui, |ui| {
        clear_of_scrollbar(ui);
        ui.add_space(2.0);
        for (section_index, s) in g.sections.iter().enumerate() {
            let mut items: Vec<_> = s.items.iter().filter(|item| !matches!(item.availability, Availability::Planned(_))).collect();
            if items.is_empty() {
                continue;
            }
            items.sort_by_key(|item| app.menu_tool_order.iter().position(|command| command == item.command).unwrap_or(usize::MAX));
            widgets::section_title(ui, tl!(s.title));
            for item in items {
                if g.id == "fill_sign" && (item.command.starts_with("sign.fill.signature") || item.command.starts_with("sign.fill.initials")) {
                    continue;
                }
                let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 34.0), Sense::click_and_drag());
                if resp.drag_started() {
                    egui::DragAndDrop::set_payload(ui.ctx(), MenuItemDrag { group: g.id, section: section_index, command: item.command });
                }
                let after = resp.ctx.pointer_latest_pos().is_some_and(|pointer| pointer.y > resp.rect.center().y);
                if let Some(dragged) = resp.dnd_hover_payload::<MenuItemDrag>()
                    && dragged.group == g.id
                    && dragged.section == section_index
                    && dragged.command != item.command
                {
                    let y = if after { resp.rect.bottom() } else { resp.rect.top() };
                    ui.painter().hline(resp.rect.x_range(), y, Stroke::new(2.5, t.accent));
                }
                if let Some(dragged) = resp.dnd_release_payload::<MenuItemDrag>()
                    && dragged.group == g.id
                    && dragged.section == section_index
                    && dragged.command != item.command
                {
                    reorder = Some((dragged.command, item.command, after));
                }
                let recommended = app.recommended_tools.iter().any(|id| id == item.command);
                resp.context_menu(|ui| {
                    let action = if recommended { tl!("Remove") } else { tl!("Add") };
                    if ui.button(format!("{action}: {}", tl!("Recommended tools"))).clicked() {
                        recommendation = Some((item.command, !recommended));
                        ui.close();
                    }
                });
                resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, tl!(item.label)));
                let ready = item.availability == Availability::Ready;
                let press = theme::Press::track(ui, &resp);
                press.wash(ui, rect, 6, false);
                let body = rect.translate(press.offset());
                let fg = if ready { t.text } else { t.text_muted };
                icons::paint(
                    ui,
                    Rect::from_min_size(body.min + vec2(6.0, 8.0), vec2(18.0, 18.0)),
                    item.icon,
                    17.0,
                    if ready { hue(g) } else { t.text_faint },
                );
                row_label(ui, tl!(item.label), theme::regular(13.0), fg, body.left_center() + vec2(34.0, 0.0), body.width() - 58.0);
                let resp = theme::hand(resp).on_hover_text(format!("{} — {}", tl!(item.label), item.command));
                // An entry with further commands opens a menu; its caret goes before the chip.
                if item.actions.is_empty() {
                    if resp.clicked() {
                        run = Some(item.command);
                    }
                } else if let Some(cmd) = actions_menu(app, ui, &resp, item, body.right_center() - vec2(14.0, 0.0), fg) {
                    run = Some(cmd);
                }
            }
        }
        if g.id == "fill_sign" {
            widgets::section_title(ui, tl!("Sign yourself"));
            if let Some(cmd) = crate::fill_sign::signature_entries(ui, app, t) {
                run = Some(cmd);
            }
        }
    });
    if let Some((dragged, target, after)) = reorder {
        move_menu_item(&mut app.menu_tool_order, dragged, target, after);
        if let Some(changed) = app.menu_tool_order_changed.as_mut() {
            changed(app.menu_tool_order.clone());
        }
    }
    if let Some((id, add)) = recommendation {
        if add && !app.recommended_tools.iter().any(|saved| saved == id) {
            app.recommended_tools.push(id.to_owned());
        } else if !add {
            app.recommended_tools.retain(|saved| saved != id);
        }
        normalize_recommended_tools(&mut app.recommended_tools);
        if let Some(changed) = app.recommended_tools_changed.as_mut() {
            changed(app.recommended_tools.clone());
        }
    }
    if footer {
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            if marks > 0 {
                ui.label(
                    egui::RichText::new(if marks == 1 {
                        crate::i18n::fmt(tl!("1 mark"), &[])
                    } else {
                        crate::i18n::fmt(tl!("{n} marks"), &[("n", &marks.to_string())])
                    })
                    .color(t.text_muted),
                );
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.add_enabled_ui(marks > 0, |ui| widgets::pill_button(ui, tl!("Redact all"), true)).inner.clicked() {
                    run = Some("redact.apply");
                }
                if ui.add_enabled_ui(marks > 0, |ui| widgets::pill_button(ui, tl!("Clear all"), false)).inner.clicked() {
                    run = Some("redact.clear");
                }
            });
        });
    }
    if let Some(cmd) = run {
        app.run_command(cmd);
    }
}
