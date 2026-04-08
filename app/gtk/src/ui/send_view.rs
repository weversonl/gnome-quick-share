use std::cell::RefCell;
use std::collections::HashMap;
use std::path::Path;
use std::rc::Rc;
use std::time::Duration;

use gtk4::prelude::*;

use gnomeqs_core::channel::ChannelMessage;
use gnomeqs_core::{EndpointInfo, State};

use crate::bridge::FromUi;
use crate::tr;
use super::cursor::set_pointer_cursor;
use super::device_tile::DeviceTile;
use super::pulse::build_pulse_placeholder;
use super::transfer_row::TransferRow;

pub struct SendView {
    pub root: gtk4::Box,
    devices_box: gtk4::FlowBox,
    selected_files: Rc<RefCell<Vec<String>>>,
    from_ui_tx: async_channel::Sender<FromUi>,
    devices: Rc<RefCell<HashMap<String, DeviceTile>>>,
    transfers: Rc<RefCell<HashMap<String, TransferRow>>>,
    transfer_list: gtk4::ListBox,
    devices_stack: gtk4::Stack,
    devices_placeholder: gtk4::Box,
    devices_scroll: gtk4::ScrolledWindow,
    endpoint_tx: Rc<RefCell<Option<tokio::sync::broadcast::Sender<EndpointInfo>>>>,
    discovery_active: Rc<RefCell<bool>>,
    pending_start: Rc<RefCell<Option<glib::SourceId>>>,
}

impl SendView {
    pub fn new(from_ui_tx: async_channel::Sender<FromUi>) -> Self {
        let root = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
        root.set_vexpand(true);

        let content_scroll = gtk4::ScrolledWindow::new();
        content_scroll.set_policy(gtk4::PolicyType::Never, gtk4::PolicyType::Automatic);
        content_scroll.set_vexpand(true);
        content_scroll.set_hexpand(true);

        let content = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
        content.set_margin_bottom(128);
        content_scroll.set_child(Some(&content));
        root.append(&content_scroll);

        let selected_files: Rc<RefCell<Vec<String>>> = Rc::new(RefCell::new(Vec::new()));
        let devices: Rc<RefCell<HashMap<String, DeviceTile>>> =
            Rc::new(RefCell::new(HashMap::new()));
        let transfers: Rc<RefCell<HashMap<String, TransferRow>>> =
            Rc::new(RefCell::new(HashMap::new()));
        let endpoint_tx: Rc<RefCell<Option<tokio::sync::broadcast::Sender<EndpointInfo>>>> =
            Rc::new(RefCell::new(None));
        let discovery_active = Rc::new(RefCell::new(false));
        let pending_start: Rc<RefCell<Option<glib::SourceId>>> =
            Rc::new(RefCell::new(None));

        // ── File selection area ───────────────────────────────────────────────
        let files_group = gtk4::Box::new(gtk4::Orientation::Vertical, 6);
        files_group.add_css_class("glass-card");
        files_group.add_css_class("send-drop-card");
        files_group.set_margin_top(12);
        files_group.set_margin_bottom(8);
        files_group.set_margin_start(12);
        files_group.set_margin_end(12);
        files_group.set_valign(gtk4::Align::Start);

        let upload_icon = gtk4::Image::from_icon_name("io.github.weversonl.GnomeQuickShare-airdrop-symbolic");
        upload_icon.add_css_class("send-drop-icon");
        upload_icon.set_halign(gtk4::Align::Center);

        let files_title = gtk4::Label::new(Some(&tr!("Drop files to send")));
        files_title.add_css_class("send-drop-title");
        files_title.set_halign(gtk4::Align::Center);

        let files_subtitle = gtk4::Label::new(Some(&tr!("Select")));
        files_subtitle.add_css_class("send-drop-subtitle");
        files_subtitle.set_halign(gtk4::Align::Center);

        let actions = gtk4::Box::new(gtk4::Orientation::Horizontal, 6);
        actions.set_halign(gtk4::Align::Center);

        let select_btn = gtk4::Button::with_label(&tr!("Select"));
        select_btn.add_css_class("send-select-button");
        select_btn.set_valign(gtk4::Align::Center);
        set_pointer_cursor(&select_btn);
        actions.append(&select_btn);

        let clear_files_btn = gtk4::Button::from_icon_name("user-trash-symbolic");
        clear_files_btn.add_css_class("flat");
        clear_files_btn.add_css_class("clear-files-button");
        clear_files_btn.set_valign(gtk4::Align::Center);
        clear_files_btn.set_visible(false);
        clear_files_btn.set_tooltip_text(Some(&tr!("Clear")));
        set_pointer_cursor(&clear_files_btn);
        actions.append(&clear_files_btn);

        let selected_files_flow = gtk4::FlowBox::new();
        selected_files_flow.set_selection_mode(gtk4::SelectionMode::None);
        selected_files_flow.set_halign(gtk4::Align::Start);
        selected_files_flow.set_valign(gtk4::Align::Start);
        selected_files_flow.set_max_children_per_line(8);
        selected_files_flow.set_min_children_per_line(1);
        selected_files_flow.set_column_spacing(8);
        selected_files_flow.set_row_spacing(8);
        selected_files_flow.set_visible(false);

        files_group.append(&upload_icon);
        files_group.append(&files_title);
        files_group.append(&files_subtitle);
        files_group.append(&actions);
        files_group.append(&selected_files_flow);
        content.append(&files_group);

        // ── Outbound transfer list ──────────────────────────────────────────
        let transfer_list = gtk4::ListBox::new();
        transfer_list.add_css_class("boxed-list");
        transfer_list.add_css_class("glass-card");
        transfer_list.set_selection_mode(gtk4::SelectionMode::None);
        transfer_list.set_visible(false);
        transfer_list.set_margin_top(6);
        transfer_list.set_margin_bottom(6);
        transfer_list.set_margin_start(12);
        transfer_list.set_margin_end(12);
        content.append(&transfer_list);

        // ── File picker button ────────────────────────────────────────────────
        {
            let selected_files = Rc::clone(&selected_files);
            let files_subtitle_clone = files_subtitle.clone();
            let clear_btn_clone = clear_files_btn.clone();
            let selected_files_flow_clone = selected_files_flow.clone();
            let upload_icon_clone = upload_icon.clone();
            select_btn.connect_clicked(move |btn| {
                let files_ref = Rc::clone(&selected_files);
                let subtitle_ref = files_subtitle_clone.clone();
                let clear_ref = clear_btn_clone.clone();
                let flow_ref = selected_files_flow_clone.clone();
                let upload_icon_ref = upload_icon_clone.clone();

                // Get the root window
                let window = btn.root().and_downcast::<gtk4::Window>();
                let dialog = gtk4::FileDialog::new();
                dialog.set_title(&tr!("Select files to send"));
                dialog.set_modal(true);

                dialog.open_multiple(
                    window.as_ref(),
                    gio::Cancellable::NONE,
                    move |result| {
                        if let Ok(files) = result {
                            let mut paths = Vec::new();
                            for i in 0..files.n_items() {
                                if let Some(obj) = files.item(i) {
                                    if let Ok(file) = obj.downcast::<gio::File>() {
                                        if let Some(p) = file.path() {
                                            paths.push(p.to_string_lossy().into_owned());
                                        }
                                    }
                                }
                            }
                            if !paths.is_empty() {
                                *files_ref.borrow_mut() = paths;
                                rebuild_selected_files_ui(
                                    &files_ref,
                                    &flow_ref,
                                    &subtitle_ref,
                                    &clear_ref,
                                    &upload_icon_ref,
                                );
                            }
                        }
                    },
                );
            });
        }

        // ── Clear files button ────────────────────────────────────────────────
        {
            let selected_files = Rc::clone(&selected_files);
            let files_subtitle_clone = files_subtitle.clone();
            let selected_files_flow_clone = selected_files_flow.clone();
            let upload_icon_clone = upload_icon.clone();
            clear_files_btn.connect_clicked(move |btn| {
                *selected_files.borrow_mut() = Vec::new();
                rebuild_selected_files_ui(
                    &selected_files,
                    &selected_files_flow_clone,
                    &files_subtitle_clone,
                    btn,
                    &upload_icon_clone,
                );
            });
        }

        // ── Drop target ───────────────────────────────────────────────────────
        let drop_target = gtk4::DropTarget::new(
            gio::File::static_type(),
            gtk4::gdk::DragAction::COPY,
        );
        {
            let selected_files = Rc::clone(&selected_files);
            let files_subtitle_clone = files_subtitle.clone();
            let clear_btn_clone = clear_files_btn.clone();
            let selected_files_flow_clone = selected_files_flow.clone();
            let upload_icon_clone = upload_icon.clone();
            drop_target.connect_drop(move |_, value, _, _| {
                if let Ok(file) = value.get::<gio::File>() {
                    if let Some(path) = file.path() {
                        let path_str = path.to_string_lossy().into_owned();
                        selected_files.borrow_mut().push(path_str.clone());
                        rebuild_selected_files_ui(
                            &selected_files,
                            &selected_files_flow_clone,
                            &files_subtitle_clone,
                            &clear_btn_clone,
                            &upload_icon_clone,
                        );
                        return true;
                    }
                }
                false
            });
        }
        root.add_controller(drop_target);

        // ── Nearby devices area ───────────────────────────────────────────────
        let devices_card = gtk4::Box::new(gtk4::Orientation::Vertical, 10);
        devices_card.add_css_class("glass-card");
        devices_card.add_css_class("devices-card");
        devices_card.set_margin_top(8);
        devices_card.set_margin_bottom(8);
        devices_card.set_margin_start(12);
        devices_card.set_margin_end(12);

        let devices_header = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);

        let devices_label = gtk4::Label::new(Some(&tr!("Nearby devices")));
        devices_label.add_css_class("caption-heading");
        devices_label.set_hexpand(true);
        devices_label.set_halign(gtk4::Align::Start);

        let refresh_btn = gtk4::Button::from_icon_name("view-refresh-symbolic");
        refresh_btn.add_css_class("flat");
        refresh_btn.set_tooltip_text(Some(&tr!("Refresh")));
        set_pointer_cursor(&refresh_btn);

        devices_header.append(&devices_label);
        devices_header.append(&refresh_btn);
        devices_card.append(&devices_header);

        let scroll = gtk4::ScrolledWindow::new();
        scroll.set_vexpand(true);
        scroll.set_policy(gtk4::PolicyType::Never, gtk4::PolicyType::Automatic);

        let devices_box = gtk4::FlowBox::new();
        devices_box.set_selection_mode(gtk4::SelectionMode::None);
        devices_box.set_valign(gtk4::Align::Start);
        devices_box.set_halign(gtk4::Align::Start);
        devices_box.set_margin_top(6);
        devices_box.set_margin_bottom(12);
        devices_box.set_margin_start(6);
        devices_box.set_margin_end(6);
        devices_box.set_column_spacing(12);
        devices_box.set_row_spacing(12);
        scroll.set_child(Some(&devices_box));

        let devices_placeholder = build_pulse_placeholder(None, Some(&tr!("Nearby devices")), false);
        devices_placeholder.set_margin_top(12);
        devices_placeholder.set_margin_bottom(12);

        let devices_stack = gtk4::Stack::new();
        devices_stack.set_vexpand(true);
        devices_stack.set_size_request(-1, 240);
        devices_stack.add_child(&devices_placeholder);
        devices_stack.add_child(&scroll);
        devices_stack.set_visible_child(&devices_placeholder);
        devices_card.append(&devices_stack);
        content.append(&devices_card);

        // ── Refresh button ────────────────────────────────────────────────────
        {
            let tx = from_ui_tx.clone();
            let ep_tx = Rc::clone(&endpoint_tx);
            let active = Rc::clone(&discovery_active);
            let pending = Rc::clone(&pending_start);
            refresh_btn.connect_clicked(move |_| {
                // Stop existing discovery
                if *active.borrow() {
                    if let Err(e) = tx.try_send(FromUi::StopDiscovery) {
                        log::warn!("StopDiscovery: {e}");
                    }
                    *active.borrow_mut() = false;
                }

                if let Some(id) = pending.borrow_mut().take() {
                    id.remove();
                }

                let tx2 = tx.clone();
                let ep_tx2 = Rc::clone(&ep_tx);
                let active2 = Rc::clone(&active);
                let pending2 = Rc::clone(&pending);
                let id = glib::timeout_add_local(Duration::from_millis(300), move || {
                    *pending2.borrow_mut() = None;
                    if *active2.borrow() {
                        return glib::ControlFlow::Break;
                    }
                    let (sender, _) = tokio::sync::broadcast::channel(20);
                    *ep_tx2.borrow_mut() = Some(sender.clone());
                    if let Err(e) = tx2.try_send(FromUi::StartDiscovery(sender)) {
                        log::warn!("StartDiscovery: {e}");
                    } else {
                        *active2.borrow_mut() = true;
                    }
                    glib::ControlFlow::Break
                });
                *pending.borrow_mut() = Some(id);
            });
        }

        Self {
            root,
            devices_box,
            selected_files,
            from_ui_tx,
            devices,
            transfers,
            transfer_list,
            devices_stack,
            devices_placeholder,
            devices_scroll: scroll,
            endpoint_tx,
            discovery_active,
            pending_start,
        }
    }

    /// Update the device list when an endpoint appears or disappears.
    pub fn update_endpoint(&self, info: EndpointInfo) {
        let present = info.present.unwrap_or(true);
        let mut devices = self.devices.borrow_mut();

        if !present {
            // Remove the tile
            if let Some(tile) = devices.remove(&info.id) {
                self.devices_box.remove(&tile.button);
            }
            if devices.is_empty() {
                self.devices_stack.set_visible_child(&self.devices_placeholder);
            }
            return;
        }

        if devices.contains_key(&info.id) {
            return; // Already present, no update needed
        }

        let files = Rc::clone(&self.selected_files);
        let tx = self.from_ui_tx.clone();
        let tile = DeviceTile::new(
            info.clone(),
            move || files.borrow().clone(),
            tx,
        );
        self.devices_box.append(&tile.button);
        devices.insert(info.id.clone(), tile);
        self.devices_stack.set_visible_child(&self.devices_scroll);
    }

    /// Kick off mDNS discovery when the Send tab is shown.
    pub fn start_discovery(&self) {
        if *self.discovery_active.borrow() {
            return;
        }
        if let Some(id) = self.pending_start.borrow_mut().take() {
            let _ = std::panic::catch_unwind(|| id.remove());
        }
        let (sender, _) = tokio::sync::broadcast::channel(20);
        *self.endpoint_tx.borrow_mut() = Some(sender.clone());
        if let Err(e) = self.from_ui_tx.try_send(FromUi::StartDiscovery(sender)) {
            log::warn!("StartDiscovery: {e}");
        } else {
            *self.discovery_active.borrow_mut() = true;
        }
    }

    /// Stop mDNS discovery when the Send tab is hidden.
    pub fn stop_discovery(&self) {
        if let Some(id) = self.pending_start.borrow_mut().take() {
            let _ = std::panic::catch_unwind(|| id.remove());
        }
        if let Err(e) = self.from_ui_tx.try_send(FromUi::StopDiscovery) {
            log::warn!("StopDiscovery: {e}");
        } else {
            *self.discovery_active.borrow_mut() = false;
        }
    }

    pub fn handle_channel_message(&self, msg: ChannelMessage) {
        let state = match &msg.state {
            Some(state) => state.clone(),
            None => return,
        };
        let meta = match &msg.meta {
            Some(meta) => meta.clone(),
            None => return,
        };
        let id = msg.id.clone();

        let mut map = self.transfers.borrow_mut();

        if !map.contains_key(&id) {
            let row = TransferRow::new(id.clone(), self.from_ui_tx.clone());
            {
                let id = id.clone();
                let transfers = Rc::clone(&self.transfers);
                let list = self.transfer_list.clone();
                row.connect_clear(move || {
                    let mut map = transfers.borrow_mut();
                    if let Some(row) = map.remove(&id) {
                        list.remove(&row.row);
                    }
                    list.set_visible(!map.is_empty());
                });
            }
            self.transfer_list.append(&row.row);
            self.transfer_list.set_visible(true);
            row.update_state(&state, &meta);
            map.insert(id, row);
        } else if let Some(row) = map.get(&id) {
            row.update_state(&state, &meta);
            match state {
                State::Disconnected | State::Finished | State::Rejected | State::Cancelled => {}
                _ => {}
            }
        }
    }
}

fn rebuild_selected_files_ui(
    selected_files: &Rc<RefCell<Vec<String>>>,
    flow: &gtk4::FlowBox,
    subtitle: &gtk4::Label,
    clear_btn: &gtk4::Button,
    upload_icon: &gtk4::Image,
) {
    while let Some(child) = flow.first_child() {
        flow.remove(&child);
    }

    let files = selected_files.borrow().clone();
    let count = files.len();

    if count == 0 {
        subtitle.set_text(&tr!("Select"));
        clear_btn.set_visible(false);
        flow.set_visible(false);
        upload_icon.set_visible(true);
        return;
    }

    subtitle.set_text(&format!(
        "{count} {}",
        if count == 1 { tr!("file") } else { tr!("files") }
    ));
    clear_btn.set_visible(true);
    flow.set_visible(true);
    upload_icon.set_visible(false);

    for (index, path) in files.iter().enumerate() {
        let file_name = Path::new(path)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or(path)
            .to_string();

        let tile = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
        tile.add_css_class("selected-file-tile");
        tile.set_size_request(52, 52);
        tile.set_halign(gtk4::Align::Center);
        tile.set_valign(gtk4::Align::Center);
        tile.set_tooltip_text(Some(&file_name));
        tile.set_hexpand(true);
        tile.set_vexpand(true);
        tile.set_homogeneous(true);

        let icon = gtk4::Image::from_icon_name(file_icon_name(path));
        icon.add_css_class("selected-file-tile-icon");
        icon.set_icon_size(gtk4::IconSize::Large);
        icon.set_halign(gtk4::Align::Center);
        icon.set_valign(gtk4::Align::Center);
        icon.set_hexpand(true);
        icon.set_vexpand(true);
        tile.append(&icon);

        let remove_btn = gtk4::Button::from_icon_name("window-close-symbolic");
        remove_btn.add_css_class("selected-file-remove-badge");
        remove_btn.set_tooltip_text(Some(&tr!("Remove")));
        remove_btn.set_halign(gtk4::Align::End);
        remove_btn.set_valign(gtk4::Align::Start);
        remove_btn.set_margin_top(0);
        remove_btn.set_margin_end(0);
        set_pointer_cursor(&remove_btn);

        {
            let selected_files = Rc::clone(selected_files);
            let flow = flow.clone();
            let subtitle = subtitle.clone();
            let clear_btn = clear_btn.clone();
            let upload_icon = upload_icon.clone();
            remove_btn.connect_clicked(move |_| {
                let len = selected_files.borrow().len();
                if index < len {
                    selected_files.borrow_mut().remove(index);
                    rebuild_selected_files_ui(
                        &selected_files,
                        &flow,
                        &subtitle,
                        &clear_btn,
                        &upload_icon,
                    );
                }
            });
        }

        let overlay = gtk4::Overlay::new();
        overlay.add_css_class("selected-file-overlay");
        overlay.set_size_request(56, 56);
        overlay.set_halign(gtk4::Align::Start);
        overlay.set_valign(gtk4::Align::Start);
        overlay.set_tooltip_text(Some(&file_name));
        overlay.set_child(Some(&tile));
        overlay.add_overlay(&remove_btn);
        overlay.set_measure_overlay(&remove_btn, true);

        flow.insert(&overlay, -1);
    }
}

fn file_icon_name(path: &str) -> &'static str {
    let ext = Path::new(path)
        .extension()
        .and_then(|s| s.to_str())
        .map(|s| s.to_ascii_lowercase());

    match ext.as_deref() {
        Some("png" | "jpg" | "jpeg" | "webp" | "gif" | "svg" | "bmp" | "avif") => "image-x-generic-symbolic",
        Some("mp4" | "mkv" | "avi" | "mov" | "webm" | "m4v") => "video-x-generic-symbolic",
        Some("mp3" | "flac" | "wav" | "ogg" | "m4a" | "aac") => "audio-x-generic-symbolic",
        Some("pdf") => "application-pdf-symbolic",
        Some("zip" | "rar" | "7z" | "tar" | "gz" | "xz") => "package-x-generic-symbolic",
        Some("txt" | "md" | "json" | "toml" | "yaml" | "yml" | "rs" | "c" | "h" | "cpp" | "py" | "js" | "ts") => "text-x-generic-symbolic",
        _ => "text-x-generic-symbolic",
    }
}
