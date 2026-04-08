use std::collections::HashMap;
use std::rc::Rc;
use std::cell::RefCell;

use gtk4::prelude::*;
use libadwaita::prelude::*;

use gnomeqs_core::channel::{ChannelMessage, ChannelDirection};
use gnomeqs_core::{State, Visibility};

use crate::bridge::FromUi;
use crate::settings;
use crate::tr;
use super::cursor::set_pointer_cursor;
use super::pulse::build_pulse_placeholder;
use super::transfer_row::TransferRow;

pub struct ReceiveView {
    pub root: gtk4::Box,
    transfers: Rc<RefCell<HashMap<String, TransferRow>>>,
    transfer_list: gtk4::ListBox,
    empty_page: gtk4::Box,
    stack: gtk4::Stack,
    list_scroll: gtk4::ScrolledWindow,
    vis_row: libadwaita::ActionRow,
    vis_icon: gtk4::Image,
    from_ui_tx: async_channel::Sender<FromUi>,
}

impl ReceiveView {
    pub fn new(
        from_ui_tx: async_channel::Sender<FromUi>,
        _toast_overlay: libadwaita::ToastOverlay,
    ) -> Self {
        let root = gtk4::Box::new(gtk4::Orientation::Vertical, 0);

        // ── Visibility card ───────────────────────────────────────────────────
        let vis_group = gtk4::ListBox::new();
        vis_group.add_css_class("boxed-list");
        vis_group.add_css_class("glass-card");
        vis_group.set_selection_mode(gtk4::SelectionMode::None);
        vis_group.set_margin_top(12);
        vis_group.set_margin_bottom(6);
        vis_group.set_margin_start(12);
        vis_group.set_margin_end(12);

        let vis_row = libadwaita::ActionRow::new();
        vis_row.set_title(&tr!("Visibility"));
        vis_row.set_activatable(true);
        set_pointer_cursor(&vis_row);

        let vis_icon = gtk4::Image::from_icon_name("eye-open-negative-filled-symbolic");
        vis_icon.set_icon_size(gtk4::IconSize::Normal);
        vis_icon.set_pixel_size(28);
        vis_row.add_suffix(&vis_icon);

        let current_vis = Visibility::from_raw_value(settings::get_visibility_raw() as u64);
        update_visibility_row(&vis_row, &vis_icon, current_vis);

        {
            let tx = from_ui_tx.clone();
            let vis_icon_for_cb = vis_icon.clone();
            vis_row.connect_activated(move |row| {
                let current = settings::get_visibility_raw();
                let new_vis = match current {
                    0 => Visibility::Invisible,
                    _ => Visibility::Visible,
                };
                settings::set_visibility_raw(new_vis as i32);
                update_visibility_row(row, &vis_icon_for_cb, new_vis);
                if let Err(e) = tx.try_send(FromUi::ChangeVisibility(new_vis)) {
                    log::warn!("ChangeVisibility send failed: {e}");
                }
            });
        }

        vis_group.append(&vis_row);
        root.append(&vis_group);

        // ── Status page (empty state) ─────────────────────────────────────────
        let empty_page = build_pulse_placeholder(
            Some(&tr!("Ready to receive")),
            None,
            false,
        );

        // ── Transfer list ─────────────────────────────────────────────────────
        let scroll = gtk4::ScrolledWindow::new();
        scroll.set_vexpand(true);
        scroll.set_policy(gtk4::PolicyType::Never, gtk4::PolicyType::Automatic);
        scroll.set_propagate_natural_height(true);

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

        let stack = gtk4::Stack::new();
        stack.set_vexpand(true);
        stack.add_child(&empty_page);
        stack.add_child(&scroll);
        stack.set_visible_child(&empty_page);

        root.append(&stack);

        Self {
            root,
            transfers: Rc::new(RefCell::new(HashMap::new())),
            transfer_list,
            empty_page,
            stack,
            list_scroll: scroll,
            vis_row,
            vis_icon,
            from_ui_tx,
        }
    }

    /// Handle an inbound ChannelMessage and update the UI accordingly.
    pub fn handle_channel_message(&self, msg: ChannelMessage) {
        // Ignore messages going the other direction
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
            // Create a new row
            let row = TransferRow::new(id.clone(), self.from_ui_tx.clone());
            {
                let id = id.clone();
                let transfers = Rc::clone(&self.transfers);
                let list = self.transfer_list.clone();
                let stack = self.stack.clone();
                let scroll = self.list_scroll.clone();
                let empty_page = self.empty_page.clone();
                row.connect_clear(move || {
                    let mut map = transfers.borrow_mut();
                    if let Some(row) = map.remove(&id) {
                        list.remove(&row.row);
                    }
                    if map.is_empty() {
                        list.set_visible(false);
                        stack.set_visible_child(&empty_page);
                    } else {
                        stack.set_visible_child(&scroll);
                    }
                });
            }
            self.transfer_list.append(&row.row);
            self.transfer_list.set_visible(true);
            self.stack.set_visible_child(&self.list_scroll);
            row.update_state(&state, &meta);
            map.insert(id, row);
        } else if let Some(row) = map.get(&id) {
            row.update_state(&state, &meta);
            // Remove terminal rows from the list after a delay
            match &state {
                State::Disconnected | State::Finished | State::Rejected | State::Cancelled => {
                    // Row already shows Clear button; user dismisses manually
                }
                _ => {}
            }
        }
    }

    /// Update the visibility UI when the Tokio layer reports a change.
    pub fn update_visibility(&self, vis: Visibility) {
        settings::set_visibility_raw(vis as i32);
        update_visibility_row(&self.vis_row, &self.vis_icon, vis);
    }
}

fn update_visibility_row(row: &libadwaita::ActionRow, icon: &gtk4::Image, vis: Visibility) {
    icon.remove_css_class("visibility-visible");
    icon.remove_css_class("visibility-hidden");
    icon.remove_css_class("visibility-temporary");

    match vis {
        Visibility::Visible => {
            row.set_subtitle(&tr!("Always visible"));
            icon.set_icon_name(Some("eye-open-negative-filled-symbolic"));
            icon.add_css_class("visibility-visible");
        }
        Visibility::Invisible => {
            row.set_subtitle(&tr!("Hidden from everyone"));
            icon.set_icon_name(Some("eye-not-looking-symbolic"));
            icon.add_css_class("visibility-hidden");
        }
        Visibility::Temporarily => {
            row.set_subtitle(&tr!("Temporarily visible"));
            icon.set_icon_name(Some("eye-open-negative-filled-symbolic"));
            icon.add_css_class("visibility-temporary");
        }
    }
}
