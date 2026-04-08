use gtk4::prelude::*;
use std::time::{SystemTime, UNIX_EPOCH};

use gnomeqs_core::{DeviceType, EndpointInfo, SendInfo, OutboundPayload};

use crate::bridge::FromUi;
use super::cursor::set_pointer_cursor;

/// A button tile representing a single discovered nearby device.
pub struct DeviceTile {
    pub button: gtk4::Button,
}

impl DeviceTile {
    pub fn new(
        endpoint: EndpointInfo,
        get_files: impl Fn() -> Vec<String> + 'static,
        from_ui_tx: async_channel::Sender<FromUi>,
    ) -> Self {
        let icon_name = match &endpoint.rtype {
            Some(DeviceType::Phone)  => "phone-symbolic",
            Some(DeviceType::Tablet) => "tablet-symbolic",
            _                        => "computer-symbolic",
        };

        let name = endpoint.name.clone().unwrap_or_else(|| endpoint.id.clone());

        let vbox = gtk4::Box::new(gtk4::Orientation::Vertical, 8);
        vbox.set_margin_top(12);
        vbox.set_margin_bottom(12);
        vbox.set_margin_start(16);
        vbox.set_margin_end(16);
        vbox.set_halign(gtk4::Align::Center);
        vbox.set_hexpand(false);
        vbox.set_vexpand(false);
        vbox.set_valign(gtk4::Align::Center);

        let icon = gtk4::Image::from_icon_name(icon_name);
        icon.set_icon_size(gtk4::IconSize::Large);
        icon.set_pixel_size(48);

        let label = gtk4::Label::new(Some(&name));
        label.set_ellipsize(gtk4::pango::EllipsizeMode::End);
        label.set_max_width_chars(12);
        label.set_halign(gtk4::Align::Center);
        label.set_justify(gtk4::Justification::Center);

        vbox.append(&icon);
        vbox.append(&label);

        let button = gtk4::Button::new();
        button.set_child(Some(&vbox));
        button.add_css_class("flat");
        button.add_css_class("device-tile");
        button.set_halign(gtk4::Align::Center);
        button.set_valign(gtk4::Align::Start);
        button.set_hexpand(false);
        button.set_vexpand(false);
        button.set_size_request(150, 150);
        set_pointer_cursor(&button);

        let endpoint_clone = endpoint.clone();
        button.connect_clicked(move |_| {
            let files = get_files();
            if files.is_empty() {
                return;
            }
            let transfer_id = format!(
                "{}-{}",
                endpoint_clone.id,
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map(|d| d.as_micros())
                    .unwrap_or_default()
            );
            let send_info = SendInfo {
                id: transfer_id,
                name: endpoint_clone.name.clone().unwrap_or_default(),
                device_type: endpoint_clone.rtype.clone().unwrap_or(DeviceType::Unknown),
                addr: format!(
                    "{}:{}",
                    endpoint_clone.ip.as_deref().unwrap_or(""),
                    endpoint_clone.port.as_deref().unwrap_or("0")
                ),
                ob: OutboundPayload::Files(files),
            };
            if let Err(e) = from_ui_tx.try_send(FromUi::SendPayload(send_info)) {
                log::warn!("SendPayload failed: {e}");
            }
        });

        let _ = endpoint;
        Self { button }
    }
}
