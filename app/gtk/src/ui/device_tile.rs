use super::cursor::set_pointer_cursor;
use crate::tr;
use gnomeqs_core::{DeviceType, EndpointInfo, EndpointTransport};
use gtk4::prelude::*;

pub struct DeviceTile {
    pub button: gtk4::Button,
}

impl DeviceTile {
    pub fn new(
        endpoint: EndpointInfo,
        get_files: impl Fn() -> Vec<String> + 'static,
        handle_send: impl Fn(EndpointInfo, Vec<String>) + 'static,
    ) -> Self {
        let icon_name = match &endpoint.rtype {
            Some(DeviceType::Phone) => "phone-symbolic",
            Some(DeviceType::Tablet) => "tablet-symbolic",
            _ => "computer-symbolic",
        };

        let name = endpoint.name.clone().unwrap_or_else(|| endpoint.id.clone());
        let (transport_text, transport_icon, transport_class) = transport_visual(&endpoint);

        let vbox = gtk4::Box::new(gtk4::Orientation::Vertical, 8);
        vbox.set_margin_top(14);
        vbox.set_margin_bottom(14);
        vbox.set_margin_start(12);
        vbox.set_margin_end(12);
        vbox.set_halign(gtk4::Align::Center);
        vbox.set_hexpand(false);
        vbox.set_vexpand(false);
        vbox.set_valign(gtk4::Align::Start);
        vbox.set_size_request(120, 150);

        let icon = gtk4::Image::from_icon_name(icon_name);
        icon.set_pixel_size(42);
        icon.set_halign(gtk4::Align::Center);
        icon.set_valign(gtk4::Align::Center);
        icon.set_margin_top(16);
        icon.set_margin_bottom(16);
        icon.set_margin_start(16);
        icon.set_margin_end(16);

        let icon_circle = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
        icon_circle.add_css_class("device-icon-circle");
        icon_circle.set_halign(gtk4::Align::Center);
        icon_circle.set_valign(gtk4::Align::Center);
        icon_circle.set_size_request(76, 76);
        icon_circle.append(&icon);

        let label = gtk4::Label::new(Some(&name));
        label.add_css_class("device-tile-title");
        label.set_ellipsize(gtk4::pango::EllipsizeMode::End);
        label.set_max_width_chars(13);
        label.set_width_chars(11);
        label.set_lines(2);
        label.set_halign(gtk4::Align::Center);
        label.set_justify(gtk4::Justification::Center);
        label.set_wrap(true);
        label.set_wrap_mode(gtk4::pango::WrapMode::WordChar);
        label.set_size_request(-1, 42);

        let transport_row = gtk4::Box::new(gtk4::Orientation::Horizontal, 5);
        transport_row.add_css_class("device-transport-row");
        transport_row.add_css_class(transport_class);
        transport_row.set_halign(gtk4::Align::Center);
        transport_row.set_valign(gtk4::Align::End);

        let transport_image = gtk4::Image::from_icon_name(transport_icon);
        transport_image.set_pixel_size(15);
        transport_image.set_valign(gtk4::Align::Center);

        let transport_label = gtk4::Label::new(Some(&transport_text));
        transport_label.add_css_class("device-transport-label");
        transport_label.set_valign(gtk4::Align::Center);

        transport_row.append(&transport_image);
        transport_row.append(&transport_label);

        vbox.append(&icon_circle);
        vbox.append(&label);
        vbox.append(&transport_row);

        let button = gtk4::Button::new();
        button.set_child(Some(&vbox));
        button.add_css_class("flat");
        button.add_css_class("device-tile");
        button.set_halign(gtk4::Align::Center);
        button.set_valign(gtk4::Align::Start);
        button.set_hexpand(false);
        button.set_vexpand(false);
        button.set_size_request(144, 178);

        let interactive = match endpoint.transport {
            Some(EndpointTransport::WifiDirectPeer) => true,
            _ => endpoint.ip.is_some() && endpoint.port.is_some(),
        };
        button.set_sensitive(interactive);
        button.set_tooltip_text(Some(&format!("{name}\n{transport_text}")));
        if interactive {
            set_pointer_cursor(&button);
        }

        let endpoint_clone = endpoint.clone();
        button.connect_clicked(move |_| {
            let files = get_files();
            if files.is_empty() {
                return;
            }
            handle_send(endpoint_clone.clone(), files);
        });
        Self { button }
    }
}

fn transport_visual(endpoint: &EndpointInfo) -> (String, &'static str, &'static str) {
    match endpoint.transport {
        Some(EndpointTransport::WifiDirectPeer) => (
            tr!("Wi-Fi Direct"),
            "io.github.weversonl.GnomeQuickShare-airdrop-symbolic",
            "transport-wifi-direct",
        ),
        _ => (tr!("Wi-Fi"), "network-wireless-symbolic", "transport-wifi"),
    }
}
