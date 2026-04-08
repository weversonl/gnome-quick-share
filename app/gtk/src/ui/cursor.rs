use gtk4::prelude::*;

pub fn set_pointer_cursor(widget: &impl IsA<gtk4::Widget>) {
    widget.as_ref().set_cursor_from_name(Some("pointer"));
}
