use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use gtk4::prelude::*;
use libadwaita::prelude::*;

use gnomeqs_core::Visibility;
use gnomeqs_core::channel::{ChannelDirection, ChannelMessage};

use super::cursor::set_pointer_cursor;
use super::pulse::build_pulse_placeholder_sized;
use super::transfer_row::TransferRow;
use crate::bridge::FromUi;
use crate::settings;
use crate::tr;
use crate::transfer_history::{self, HistoryDirection, HistoryEntry};

pub struct ReceiveView {
    pub root: gtk4::Box,
    transfers: Rc<RefCell<HashMap<String, TransferRow>>>,
    transfer_list: gtk4::ListBox,
    recent_list: gtk4::ListBox,
    transfer_header: gtk4::Box,
    transfers_heading: gtk4::Label,
    history_button: gtk4::Button,
    empty_page: gtk4::Box,
    stack: gtk4::Stack,
    list_scroll: gtk4::ScrolledWindow,
    vis_indicator_icon: gtk4::Image,
    vis_indicator_label: gtk4::Label,
    from_ui_tx: async_channel::Sender<FromUi>,
    toast_overlay: libadwaita::ToastOverlay,
    history_controls: ReceiveHistoryControls,
}

#[derive(Clone)]
struct ReceiveHistoryControls {
    history_button: gtk4::Button,
    transfer_header: gtk4::Box,
    transfers_heading: gtk4::Label,
    clear_all_row: libadwaita::ActionRow,
    clear_all_btn: gtk4::Button,
    stack: gtk4::Stack,
    toast_overlay: libadwaita::ToastOverlay,
}

impl ReceiveView {
    pub fn new(
        from_ui_tx: async_channel::Sender<FromUi>,
        toast_overlay: libadwaita::ToastOverlay,
    ) -> Self {
        let root = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
        root.add_css_class("receive-page");
        root.set_vexpand(true);

        // ── Ready-to-receive card ────────────────────────────────
        let ready_card = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
        ready_card.add_css_class("recv-ready-card");
        ready_card.set_vexpand(true);
        ready_card.set_margin_top(34);
        ready_card.set_margin_start(22);
        ready_card.set_margin_end(22);
        ready_card.set_margin_bottom(20);

        let pulse = build_pulse_placeholder_sized(None, None, false, Some(228));
        pulse.set_margin_top(10);
        pulse.set_margin_bottom(0);
        ready_card.append(&pulse);

        let title_box = gtk4::Box::new(gtk4::Orientation::Vertical, 4);
        title_box.set_halign(gtk4::Align::Center);
        title_box.set_margin_top(-8);
        title_box.set_margin_bottom(12);

        let title_line1 = gtk4::Label::new(Some(&tr!("Ready to")));
        title_line1.add_css_class("recv-ready-title-plain");
        title_line1.set_halign(gtk4::Align::Center);

        let title_line2 = gtk4::Label::new(Some(&tr!("receive")));
        title_line2.add_css_class("recv-ready-title-accent");
        title_line2.set_halign(gtk4::Align::Center);

        title_box.append(&title_line1);
        title_box.append(&title_line2);
        ready_card.append(&title_box);

        // Visibility indicator — clickable pill that toggles Visible ↔ Invisible
        let vis_indicator = gtk4::Box::new(gtk4::Orientation::Horizontal, 6);
        vis_indicator.add_css_class("recv-vis-indicator");
        vis_indicator.set_halign(gtk4::Align::Center);

        let vis_indicator_icon = gtk4::Image::from_icon_name("eye-open-negative-filled-symbolic");
        vis_indicator_icon.set_pixel_size(20);
        let vis_indicator_label = gtk4::Label::new(None);
        vis_indicator_label.add_css_class("recv-vis-label");
        vis_indicator.append(&vis_indicator_icon);
        vis_indicator.append(&vis_indicator_label);

        let vis_btn = gtk4::Button::new();
        vis_btn.set_child(Some(&vis_indicator));
        vis_btn.add_css_class("flat");
        vis_btn.add_css_class("recv-vis-btn");
        vis_btn.set_halign(gtk4::Align::Center);
        vis_btn.set_margin_bottom(20);
        set_pointer_cursor(&vis_btn);

        {
            let icon = vis_indicator_icon.clone();
            let label = vis_indicator_label.clone();
            let tx = from_ui_tx.clone();
            vis_btn.connect_clicked(move |_| {
                let current = Visibility::from_raw_value(settings::get_visibility_raw() as u64);
                let next = match current {
                    Visibility::Visible => Visibility::Invisible,
                    Visibility::Invisible => Visibility::Visible,
                    Visibility::Temporarily => Visibility::Visible,
                };
                settings::set_visibility_raw(next as i32);
                apply_vis_indicator(&icon, &label, next);
                let _ = tx.send_blocking(FromUi::ChangeVisibility(next));
            });
        }
        ready_card.append(&vis_btn);

        // Wrapper so the card sits near the top like the Figma receive view.
        let empty_page = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
        empty_page.set_vexpand(true);
        empty_page.append(&ready_card);

        let scroll = gtk4::ScrolledWindow::new();
        scroll.set_vexpand(true);
        scroll.set_policy(gtk4::PolicyType::Never, gtk4::PolicyType::Automatic);
        scroll.set_propagate_natural_height(true);

        let transfer_header = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
        transfer_header.set_margin_top(2);
        transfer_header.set_margin_bottom(8);
        transfer_header.set_margin_start(16);
        transfer_header.set_margin_end(16);
        transfer_header.set_visible(false);

        let transfers_heading = gtk4::Label::new(Some(&tr!("Active transfers")));
        transfers_heading.add_css_class("caption-heading");
        transfers_heading.set_halign(gtk4::Align::Start);
        transfers_heading.set_hexpand(true);

        let history_button = gtk4::Button::with_label(&tr!("History"));
        history_button.add_css_class("history-button");
        history_button.set_visible(false);
        history_button.set_halign(gtk4::Align::End);
        set_pointer_cursor(&history_button);

        transfer_header.append(&transfers_heading);
        transfer_header.append(&history_button);

        let transfer_list = gtk4::ListBox::new();
        transfer_list.add_css_class("boxed-list");
        transfer_list.add_css_class("glass-card");
        transfer_list.set_selection_mode(gtk4::SelectionMode::None);
        transfer_list.set_valign(gtk4::Align::Start);
        transfer_list.set_vexpand(false);
        transfer_list.set_margin_top(6);
        transfer_list.set_margin_bottom(12);
        transfer_list.set_margin_start(12);
        transfer_list.set_margin_end(12);
        transfer_list.set_visible(false);

        scroll.set_child(Some(&transfer_list));

        let recent_list = gtk4::ListBox::new();
        recent_list.add_css_class("history-list");
        recent_list.set_selection_mode(gtk4::SelectionMode::None);
        recent_list.set_margin_top(0);
        recent_list.set_margin_bottom(0);
        recent_list.set_margin_start(0);
        recent_list.set_margin_end(0);

        let history_controls = ReceiveHistoryControls {
            history_button: history_button.clone(),
            transfer_header: transfer_header.clone(),
            transfers_heading: transfers_heading.clone(),
            clear_all_row: libadwaita::ActionRow::new(),
            clear_all_btn: gtk4::Button::from_icon_name("user-trash-symbolic"),
            stack: gtk4::Stack::new(),
            toast_overlay: toast_overlay.clone(),
        };

        let history_dialog = build_receive_history_dialog(&recent_list, &history_controls);
        {
            let history_dialog = history_dialog.clone();
            history_button.connect_clicked(move |btn| {
                let Some(window) = btn.root().and_downcast::<gtk4::Window>() else {
                    return;
                };
                history_dialog.present(Some(&window));
            });
        }
        load_receive_history(&recent_list, &history_controls);

        let stack = gtk4::Stack::new();
        stack.set_vexpand(true);
        stack.add_child(&empty_page);
        stack.add_child(&scroll);
        stack.set_visible_child(&empty_page);

        root.append(&transfer_header);
        root.append(&stack);

        // Apply initial visibility state
        let init_vis = Visibility::from_raw_value(settings::get_visibility_raw() as u64);
        apply_vis_indicator(&vis_indicator_icon, &vis_indicator_label, init_vis);

        Self {
            root,
            transfers: Rc::new(RefCell::new(HashMap::new())),
            transfer_list,
            recent_list,
            transfer_header,
            transfers_heading,
            history_button,
            empty_page,
            stack,
            list_scroll: scroll,
            vis_indicator_icon,
            vis_indicator_label,
            from_ui_tx,
            toast_overlay,
            history_controls,
        }
    }

    pub fn handle_channel_message(&self, msg: ChannelMessage) {
        if msg.direction != ChannelDirection::LibToFront {
            return;
        }

        let id = msg.id.clone();
        let state = match &msg.state {
            Some(s) => s.clone(),
            None => return,
        };
        let meta = match &msg.meta {
            Some(m) => m.clone(),
            None => return,
        };

        let mut map = self.transfers.borrow_mut();

        if !map.contains_key(&id) {
            let row = TransferRow::new(id.clone(), self.from_ui_tx.clone());
            {
                let id = id.clone();
                let transfers = Rc::clone(&self.transfers);
                let list = self.transfer_list.clone();
                let recent_list = self.recent_list.clone();
                let stack = self.stack.clone();
                let scroll = self.list_scroll.clone();
                let empty_page = self.empty_page.clone();
                let transfers_heading = self.transfers_heading.clone();
                let transfer_header = self.transfer_header.clone();
                let history_controls = ReceiveHistoryControls {
                    history_button: self.history_button.clone(),
                    transfer_header: self.transfer_header.clone(),
                    transfers_heading: self.transfers_heading.clone(),
                    clear_all_row: self.history_controls.clear_all_row.clone(),
                    clear_all_btn: self.history_controls.clear_all_btn.clone(),
                    stack: self.history_controls.stack.clone(),
                    toast_overlay: self.toast_overlay.clone(),
                };
                row.connect_clear(move || {
                    let mut map = transfers.borrow_mut();
                    if let Some(row) = map.remove(&id) {
                        let (title, subtitle) = row.history_snapshot();
                        let open_target = row.open_target_snapshot();
                        list.remove(&row.row);
                        let entry = HistoryEntry {
                            created_at: 0,
                            direction: HistoryDirection::Receive,
                            title,
                            subtitle,
                            open_target,
                        };
                        if settings::get_save_transfer_history() {
                            let entry = transfer_history::append(entry);
                            prepend_receive_history_row(&recent_list, entry, &history_controls);
                            history_controls.history_button.set_visible(true);
                            history_controls.history_button.set_hexpand(true);
                        }
                    }
                    if map.is_empty() {
                        list.set_visible(false);
                        transfers_heading.set_visible(false);
                        transfer_header.set_visible(history_controls.history_button.is_visible());
                        stack.set_visible_child(&empty_page);
                    } else {
                        transfers_heading.set_visible(true);
                        transfer_header.set_visible(true);
                        stack.set_visible_child(&scroll);
                    }
                });
            }
            self.transfer_list.append(&row.row);
            self.transfer_list.set_visible(true);
            self.transfers_heading.set_visible(true);
            self.history_button.set_hexpand(false);
            self.transfer_header.set_visible(true);
            self.stack.set_visible_child(&self.list_scroll);
            row.update_state(&state, &meta);
            map.insert(id, row);
        } else if let Some(row) = map.get(&id) {
            row.update_state(&state, &meta);
        }
    }

    pub fn update_visibility(&self, vis: Visibility) {
        apply_vis_indicator(&self.vis_indicator_icon, &self.vis_indicator_label, vis);
    }

    pub fn clear_history(&self) {
        clear_list_box(&self.recent_list);
        update_history_controls(&self.recent_list, &self.history_controls);
        self.update_transfer_header_visibility();
    }

    fn update_transfer_header_visibility(&self) {
        let has_transfers = !self.transfers.borrow().is_empty();
        self.transfers_heading.set_visible(has_transfers);
        self.transfer_header
            .set_visible(has_transfers || self.history_button.is_visible());
    }
}

fn apply_vis_indicator(icon: &gtk4::Image, label: &gtk4::Label, vis: Visibility) {
    icon.remove_css_class("visibility-visible");
    icon.remove_css_class("visibility-hidden");
    icon.remove_css_class("visibility-temporary");

    match vis {
        Visibility::Visible => {
            icon.set_icon_name(Some("eye-open-negative-filled-symbolic"));
            icon.add_css_class("visibility-visible");
            label.set_text(&tr!("Visible"));
        }
        Visibility::Invisible => {
            icon.set_icon_name(Some("eye-not-looking-symbolic"));
            icon.add_css_class("visibility-hidden");
            label.set_text(&tr!("Hidden"));
        }
        Visibility::Temporarily => {
            icon.set_icon_name(Some("eye-open-negative-filled-symbolic"));
            icon.add_css_class("visibility-temporary");
            label.set_text(&tr!("Temporarily visible"));
        }
    }
}

fn build_receive_history_dialog(
    list: &gtk4::ListBox,
    controls: &ReceiveHistoryControls,
) -> libadwaita::PreferencesDialog {
    let dialog = libadwaita::PreferencesDialog::new();
    dialog.set_title(&tr!("Receive history"));
    dialog.set_search_enabled(false);

    let page = libadwaita::PreferencesPage::new();
    let group = libadwaita::PreferencesGroup::new();
    group.set_description(Some(&tr!("Transfer history is stored locally for up to {} days by default, unless changed in Settings.")
        .replace("{}", &settings::get_history_retention_days().to_string())));

    let clear_all_row = controls.clear_all_row.clone();
    clear_all_row.set_title(&tr!("Clear all"));
    clear_all_row.set_subtitle(&tr!("Remove all receive history."));
    let clear_all_btn = controls.clear_all_btn.clone();
    clear_all_btn.add_css_class("flat");
    clear_all_btn.add_css_class("destructive-action");
    clear_all_btn.set_tooltip_text(Some(&tr!("Clear all")));
    clear_all_btn.set_valign(gtk4::Align::Center);
    set_pointer_cursor(&clear_all_btn);
    clear_all_row.add_suffix(&clear_all_btn);
    clear_all_row.set_activatable_widget(Some(&clear_all_btn));
    {
        let list = list.clone();
        let controls = controls.clone();
        clear_all_btn.connect_clicked(move |btn| {
            let list = list.clone();
            let controls = controls.clone();
            confirm_clear_receive_history(btn, move || {
                if let Err(e) = transfer_history::clear_direction(HistoryDirection::Receive) {
                    log::warn!("failed to clear receive history: {e}");
                    add_history_toast(&controls, &tr!("Could not clear receive history"));
                    return;
                }
                clear_list_box(&list);
                update_history_controls(&list, &controls);
                add_history_toast(&controls, &tr!("Receive history cleared"));
            });
        });
    }
    group.add(&clear_all_row);

    let empty_state = build_history_empty_state();
    controls.stack.add_named(list, Some("history"));
    controls.stack.add_named(&empty_state, Some("empty"));

    let scroll = gtk4::ScrolledWindow::new();
    scroll.set_policy(gtk4::PolicyType::Never, gtk4::PolicyType::Automatic);
    scroll.set_min_content_width(300);
    scroll.set_min_content_height(220);
    scroll.set_max_content_height(420);
    scroll.set_child(Some(&controls.stack));

    group.add(&scroll);
    page.add(&group);
    dialog.add(&page);
    dialog
}

fn load_receive_history(list: &gtk4::ListBox, controls: &ReceiveHistoryControls) {
    let entries = transfer_history::load(HistoryDirection::Receive);
    for entry in entries.into_iter().rev() {
        prepend_receive_history_row(list, entry, controls);
    }
    controls.transfers_heading.set_visible(false);
    update_history_controls(list, controls);
}

fn clear_list_box(list: &gtk4::ListBox) {
    while let Some(child) = list.first_child() {
        list.remove(&child);
    }
}

fn prepend_receive_history_row(
    list: &gtk4::ListBox,
    entry: HistoryEntry,
    controls: &ReceiveHistoryControls,
) {
    let row = gtk4::ListBoxRow::new();
    row.add_css_class("history-row");
    let row_title = if entry.title.is_empty() {
        tr!("Recent transfer")
    } else {
        entry.title.clone()
    };

    let body = gtk4::Box::new(gtk4::Orientation::Horizontal, 12);
    body.set_valign(gtk4::Align::Center);
    body.set_size_request(-1, 68);
    body.set_margin_top(10);
    body.set_margin_bottom(10);
    body.set_margin_start(12);
    body.set_margin_end(10);

    let icon = gtk4::Image::from_icon_name("folder-download-symbolic");
    icon.set_pixel_size(22);
    icon.set_halign(gtk4::Align::Center);
    icon.set_valign(gtk4::Align::Center);
    icon.set_margin_top(13);
    icon.set_margin_bottom(13);
    icon.set_margin_start(13);
    icon.set_margin_end(13);
    let icon_chip = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
    icon_chip.add_css_class("history-icon-chip");
    icon_chip.set_valign(gtk4::Align::Center);
    icon_chip.set_halign(gtk4::Align::Center);
    icon_chip.set_size_request(48, 48);
    icon_chip.append(&icon);
    body.append(&icon_chip);

    let text_box = gtk4::Box::new(gtk4::Orientation::Vertical, 2);
    text_box.set_hexpand(true);
    text_box.set_valign(gtk4::Align::Center);

    let title_label = gtk4::Label::new(Some(&row_title));
    title_label.add_css_class("history-title");
    title_label.set_halign(gtk4::Align::Start);
    title_label.set_xalign(0.0);
    title_label.set_ellipsize(gtk4::pango::EllipsizeMode::End);

    let subtitle_label = gtk4::Label::new(Some(&entry.subtitle));
    subtitle_label.add_css_class("history-subtitle");
    subtitle_label.set_halign(gtk4::Align::Start);
    subtitle_label.set_xalign(0.0);
    subtitle_label.set_ellipsize(gtk4::pango::EllipsizeMode::End);

    text_box.append(&title_label);
    text_box.append(&subtitle_label);
    body.append(&text_box);

    if let Some(path) = entry.open_target.clone() {
        let show_btn = gtk4::Button::from_icon_name("folder-open-symbolic");
        show_btn.set_tooltip_text(Some(&tr!("Show folder")));
        show_btn.add_css_class("flat");
        show_btn.add_css_class("history-icon-button");
        show_btn.set_halign(gtk4::Align::Center);
        show_btn.set_valign(gtk4::Align::Center);
        show_btn.set_size_request(34, 34);
        set_pointer_cursor(&show_btn);
        show_btn.connect_clicked(move |_| {
            let folder = std::path::Path::new(&path)
                .parent()
                .map(std::path::Path::to_path_buf)
                .unwrap_or_else(|| std::path::Path::new(&path).to_path_buf());
            let uri = gio::File::for_path(folder).uri().to_string();
            if let Err(e) =
                gio::AppInfo::launch_default_for_uri(&uri, None::<&gio::AppLaunchContext>)
            {
                log::warn!("Receive history show folder failed: {e}");
            }
        });
        body.append(&show_btn);
    }

    let remove_btn = gtk4::Button::from_icon_name("window-close-symbolic");
    remove_btn.set_tooltip_text(Some(&tr!("Remove")));
    remove_btn.add_css_class("flat");
    remove_btn.add_css_class("destructive-action");
    remove_btn.add_css_class("history-icon-button");
    remove_btn.set_halign(gtk4::Align::Center);
    remove_btn.set_valign(gtk4::Align::Center);
    remove_btn.set_size_request(34, 34);
    set_pointer_cursor(&remove_btn);
    {
        let list = list.clone();
        let row = row.clone();
        let controls = controls.clone();
        remove_btn.connect_clicked(move |_| {
            if let Err(e) = transfer_history::remove(&entry) {
                log::warn!("failed to remove receive history item: {e}");
                add_history_toast(&controls, &tr!("Could not remove history item"));
                return;
            }
            list.remove(&row);
            update_history_controls(&list, &controls);
            add_history_toast(&controls, &tr!("History item removed"));
        });
    }
    body.append(&remove_btn);

    row.set_child(Some(&body));
    list.insert(&row, 0);
    list.set_visible(true);

    while let Some(last) = list.row_at_index(6) {
        list.remove(&last);
    }
    update_history_controls(list, controls);
}

fn update_history_controls(list: &gtk4::ListBox, controls: &ReceiveHistoryControls) {
    let has_history = list.first_child().is_some();
    controls.history_button.set_visible(has_history);
    controls.history_button.set_hexpand(has_history);
    controls
        .transfer_header
        .set_visible(has_history || controls.transfers_heading.is_visible());
    controls.clear_all_row.set_sensitive(has_history);
    controls.clear_all_btn.set_sensitive(has_history);
    controls
        .stack
        .set_visible_child_name(if has_history { "history" } else { "empty" });
}

fn build_history_empty_state() -> gtk4::Box {
    let empty = gtk4::Box::new(gtk4::Orientation::Vertical, 8);
    empty.set_vexpand(true);
    empty.set_valign(gtk4::Align::Center);
    empty.set_halign(gtk4::Align::Center);

    let icon = gtk4::Image::from_icon_name("document-open-recent-symbolic");
    icon.set_pixel_size(36);
    icon.add_css_class("dim-label");

    let label = gtk4::Label::new(Some(&tr!("No history yet")));
    label.add_css_class("dim-label");
    label.set_halign(gtk4::Align::Center);

    empty.append(&icon);
    empty.append(&label);
    empty
}

fn add_history_toast(controls: &ReceiveHistoryControls, message: &str) {
    controls
        .toast_overlay
        .add_toast(libadwaita::Toast::new(message));
}

fn confirm_clear_receive_history(parent: &impl IsA<gtk4::Widget>, on_confirm: impl Fn() + 'static) {
    let alert = libadwaita::AlertDialog::new(
        Some(&tr!("Clear receive history?")),
        Some(&tr!(
            "This will remove all received transfer history stored locally."
        )),
    );
    alert.add_responses(&[("cancel", &tr!("Cancel")), ("clear", &tr!("Clear all"))]);
    alert.set_default_response(Some("cancel"));
    alert.set_close_response("cancel");
    alert.set_response_appearance("clear", libadwaita::ResponseAppearance::Destructive);
    alert.choose(parent, None::<&gio::Cancellable>, move |response| {
        if response.as_str() == "clear" {
            on_confirm();
        }
    });
}
