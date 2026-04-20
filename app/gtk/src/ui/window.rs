use std::cell::RefCell;
use std::rc::Rc;

use gtk4::gdk;
use gtk4::prelude::*;
use libadwaita::prelude::*;

use gnomeqs_core::channel::ChannelDirection;
use crate::bridge::ToUi;
use crate::settings;
use crate::state::AppState;
use crate::tr;
use super::cursor::set_pointer_cursor;
use super::receive_view::ReceiveView;
use super::send_view::SendView;
use super::settings_window::build_settings_window;

thread_local! {
    static APP_CSS_PROVIDER: RefCell<Option<gtk4::CssProvider>> = const { RefCell::new(None) };
}

pub fn build_window(app: &libadwaita::Application, state: AppState) -> libadwaita::ApplicationWindow {
    let (width, height, maximized) = settings::window_state();

    apply_custom_css();
    register_debug_icon_search_path();

    let win = libadwaita::ApplicationWindow::new(app);
    win.set_title(Some("GnomeQS"));
    win.set_default_size(width, height);
    if maximized {
        win.maximize();
    }
    win.add_css_class("app-window");
    sync_theme_class(&win);
    {
        let win = win.clone();
        libadwaita::StyleManager::default().connect_dark_notify(move |_| {
            sync_theme_class(&win);
        });
    }

    let toast_overlay = libadwaita::ToastOverlay::new();

    let header_bar = libadwaita::HeaderBar::new();

    let settings_btn = gtk4::Button::from_icon_name("preferences-system-symbolic");
    settings_btn.set_tooltip_text(Some(&tr!("Settings")));
    settings_btn.add_css_class("flat");
    set_pointer_cursor(&settings_btn);
    header_bar.pack_end(&settings_btn);

    let stack = libadwaita::ViewStack::new();

    let receive_view = Rc::new(ReceiveView::new(state.from_ui_tx.clone(), toast_overlay.clone()));
    let send_view = Rc::new(SendView::new(state.from_ui_tx.clone()));

    let _recv_page = stack.add_titled_with_icon(
        &receive_view.root,
        Some("receive"),
        &tr!("Receive"),
        "folder-download-symbolic",
    );

    let _send_page = stack.add_titled_with_icon(
        &send_view.root,
        Some("send"),
        &tr!("Send"),
        "share-symbolic",
    );

    {
        let send_view_clone = Rc::clone(&send_view);
        let last_page: Rc<RefCell<Option<String>>> = Rc::new(RefCell::new(None));
        let last_page_ref = Rc::clone(&last_page);
        stack.connect_visible_child_notify(move |s| {
            let current_page = s.visible_child_name().map(|name| name.to_string());
            if *last_page_ref.borrow() == current_page {
                log::debug!(
                    "view stack notify ignored: visible page unchanged ({:?})",
                    current_page
                );
                return;
            }
            *last_page_ref.borrow_mut() = current_page.clone();

            match current_page.as_deref() {
                Some("send") => {
                    log::debug!("view stack changed to send");
                    send_view_clone.start_discovery();
                }
                _ => {
                    log::debug!("view stack changed away from send");
                    send_view_clone.stop_discovery();
                }
            }
        });
    }

    let bottom_switcher = libadwaita::ViewSwitcher::new();
    bottom_switcher.set_policy(libadwaita::ViewSwitcherPolicy::Wide);
    bottom_switcher.set_stack(Some(&stack));
    bottom_switcher.set_size_request(204, 43);
    set_pointer_cursor(&bottom_switcher);

    let switcher_wrap = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
    switcher_wrap.add_css_class("pill-switcher");
    switcher_wrap.set_size_request(208, 45);
    switcher_wrap.append(&bottom_switcher);
    switcher_wrap.set_halign(gtk4::Align::Center);
    switcher_wrap.set_valign(gtk4::Align::End);
    switcher_wrap.set_margin_bottom(12);
    switcher_wrap.set_margin_start(16);
    switcher_wrap.set_margin_end(16);
    switcher_wrap.set_hexpand(false);

    let overlay = gtk4::Overlay::new();
    overlay.set_child(Some(&stack));
    overlay.add_overlay(&switcher_wrap);
    overlay.set_vexpand(true);

    let vbox = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
    vbox.add_css_class("app-root");
    vbox.append(&header_bar);
    vbox.append(&overlay);

    toast_overlay.set_child(Some(&vbox));
    win.set_content(Some(&toast_overlay));

    {
        let win_clone = win.clone();
        let tx = state.from_ui_tx.clone();
        settings_btn.connect_clicked(move |_| {
            let settings_win = build_settings_window(&win_clone, tx.clone());
            settings_win.present(Some(&win_clone));
        });
    }

    win.connect_close_request(move |w| {
        settings::save_window_state(w.width(), w.height(), w.is_maximized());
        if settings::get_keep_running_on_close() {
            w.set_visible(false);
            return glib::Propagation::Stop;
        }
        glib::Propagation::Proceed
    });

    let rx = state.to_ui_rx.clone();
    let win_weak = win.downgrade();
    let receive_view_clone = Rc::clone(&receive_view);
    let send_view_clone = Rc::clone(&send_view);
    let toast_overlay_clone = toast_overlay.clone();
    let stack_clone = stack.clone();
    let from_ui_tx_clone = state.from_ui_tx.clone();

    glib::MainContext::default().spawn_local(async move {
        while let Ok(msg) = rx.recv().await {
            let Some(win) = win_weak.upgrade() else { break };

            match msg {
                ToUi::TransferUpdate(cm) => {
                    match cm.direction {
                        ChannelDirection::LibToFront => {
                            if let Some(rtype) = &cm.rtype {
                                use gnomeqs_core::channel::TransferType;
                                match rtype {
                                    TransferType::Inbound => {
                                        if cm.state == Some(gnomeqs_core::State::WaitingForUserConsent) {
                                            if let Some(meta) = &cm.meta {
                                                let name = meta.source.as_ref()
                                                    .map(|s| s.name.as_str())
                                                    .unwrap_or("Unknown");
                                                let app = win
                                                    .application()
                                                    .and_then(|a| a.downcast::<libadwaita::Application>().ok());
                                                send_notification(app.as_ref(), name, &cm.id);
                                            }
                                        }
                                        receive_view_clone.handle_channel_message(cm);
                                    }
                                    TransferType::Outbound => {
                                        send_view_clone.handle_channel_message(cm);
                                    }
                                }
                            } else {
                                receive_view_clone.handle_channel_message(cm);
                            }
                        }
                        ChannelDirection::FrontToLib => {}
                    }
                }
                ToUi::EndpointUpdate(info) => {
                    send_view_clone.update_endpoint(info);
                }
                ToUi::VisibilityChanged(vis) => {
                    receive_view_clone.update_visibility(vis);
                }
                ToUi::BleNearby => {
                    let toast = libadwaita::Toast::new(
                        "A nearby device wants to share. Tap to become visible.",
                    );
                    toast_overlay_clone.add_toast(toast);
                }
                ToUi::Toast(message) => {
                    let toast = libadwaita::Toast::new(&message);
                    toast_overlay_clone.add_toast(toast);
                }
                ToUi::WifiDirectSessionReady(ready) => {
                    send_view_clone.handle_wifi_direct_session_ready(ready);
                }
                ToUi::ShowWindow => {
                    win.set_visible(true);
                    win.present();
                }
                ToUi::ShowWindowOnPage(page) => {
                    stack_clone.set_visible_child_name(&page);
                    win.set_visible(true);
                    win.present();
                }
                ToUi::ShowSettings => {
                    win.set_visible(true);
                    win.present();
                    let settings_win = build_settings_window(&win, from_ui_tx_clone.clone());
                    settings_win.present(Some(&win));
                }
                ToUi::Quit => {
                    if let Some(app) = win.application() {
                        app.quit();
                    }
                    break;
                }
            }
        }
    });

    win
}
pub fn apply_custom_css() {
    let Some(display) = gdk::Display::default() else { return };
    let font_size_px = settings::font_size_css_px();
    let css = r#"
/* ── Window backgrounds ────────────────────────────────────── */
.app-window.dark-mode {
  background:
    radial-gradient(ellipse 64% 42% at 16% 6%,  rgba(110, 65, 255, 0.38) 0%, transparent 100%),
    radial-gradient(ellipse 40% 28% at 84% 14%, rgba(50, 200, 255, 0.10) 0%, transparent 100%),
    radial-gradient(ellipse 70% 55% at 50% 95%, rgba(220, 60, 200, 0.05) 0%, transparent 100%),
    linear-gradient(162deg, #160f3a 0%, #0d0b26 50%, #060511 100%);
  color: #ebe7ff;
  font-size: __FONT_SIZE_PX__px;
}
.app-window.light-mode {
  background:
    radial-gradient(ellipse 60% 50% at 20% 0%,  rgba(155, 115, 255, 0.14) 0%, transparent 60%),
    radial-gradient(ellipse 40% 30% at 86% 96%, rgba(70, 195, 255, 0.07) 0%, transparent 100%),
    linear-gradient(175deg, #f5f1ff 0%, #ece5ff 55%, #e1d9ff 100%);
  color: #1c1338;
  font-size: __FONT_SIZE_PX__px;
}

/* ── Global font size ───────────────────────────────────────── */
.app-window label,
.app-window entry,
.app-window textview,
.app-window button,
.app-window row,
.app-window spinbutton,
.app-window combobox,
.app-window dropdown,
.app-window box,
.app-window listview {
  font-size: __FONT_SIZE_PX__px;
}

/* ── Root & HeaderBar ───────────────────────────────────────── */
.app-window .app-root { background: transparent; }
.app-window .headerbar,
.app-window headerbar {
  background: transparent;
  border: none;
  box-shadow: none;
}

/* ── Glass Cards ────────────────────────────────────────────── */
.app-window.dark-mode .glass-card,
.app-window.dark-mode .boxed-list {
  background: linear-gradient(150deg, rgba(255,255,255,0.09) 0%, rgba(72,52,175,0.18) 100%);
  border-radius: 18px;
  border: 1px solid rgba(255,255,255,0.13);
  box-shadow:
    inset 0 1px 0 rgba(255,255,255,0.11),
    inset 0 -1px 0 rgba(0,0,0,0.10),
    0 16px 44px rgba(0,0,0,0.38);
}
.app-window.light-mode .glass-card,
.app-window.light-mode .boxed-list {
  background: linear-gradient(150deg, rgba(255,255,255,0.92) 0%, rgba(212,196,255,0.62) 100%);
  border-radius: 18px;
  border: 1px solid rgba(138,110,210,0.17);
  box-shadow:
    inset 0 1px 0 rgba(255,255,255,0.95),
    0 12px 34px rgba(76,52,160,0.14);
}
.app-window .boxed-list row,
.app-window .boxed-list listitem { background: transparent; }

/* ── Drop Zone ──────────────────────────────────────────────── */
.app-window .send-drop-card {
  padding: 26px 20px;
  border-radius: 20px;
  transition: box-shadow 220ms ease, border-color 220ms ease;
}
.app-window.dark-mode .send-drop-card {
  background: linear-gradient(165deg, rgba(95,70,195,0.22) 0%, rgba(48,36,98,0.18) 100%);
}
.app-window.light-mode .send-drop-card {
  background: linear-gradient(165deg, rgba(195,178,255,0.32) 0%, rgba(255,255,255,0.65) 100%);
}
.app-window .send-drop-icon {
  opacity: 0.84;
  transition: transform 220ms cubic-bezier(0.34,1.56,0.64,1), opacity 200ms ease;
}
.app-window .send-drop-card.send-drop-active .send-drop-icon {
  transform: scale(1.14);
  opacity: 1.0;
}
.app-window .send-drop-title {
  font-size: 1.12em;
  font-weight: 700;
  letter-spacing: -0.01em;
}
.app-window.dark-mode .send-drop-title { color: #f0ecff; }
.app-window.light-mode .send-drop-title { color: #28195a; }
.app-window .send-drop-subtitle { font-size: 0.95em; opacity: 0.76; }
.app-window .send-drop-meta     { font-size: 0.84em; opacity: 0.68; }
.app-window .send-drop-card.send-drop-active {
  border: 1.5px solid rgba(118,194,255,0.68);
  box-shadow:
    0 0 0 3px rgba(118,194,255,0.14),
    inset 0 1px 0 rgba(255,255,255,0.14),
    0 22px 44px rgba(50,118,255,0.18);
}

/* ── Select / Clear buttons ─────────────────────────────────── */
.app-window .send-select-button {
  border-radius: 12px;
  padding: 10px 20px;
  font-weight: 700;
  letter-spacing: 0.01em;
  transition: transform 130ms cubic-bezier(0.34,1.56,0.64,1), box-shadow 160ms ease, filter 140ms ease;
}
.app-window .send-select-button:hover  { transform: translateY(-2px); }
.app-window .send-select-button:active { transform: translateY(0px); filter: brightness(0.94); }
.app-window.dark-mode .send-select-button {
  background: linear-gradient(180deg, rgba(110,84,218,0.52) 0%, rgba(76,54,172,0.38) 100%);
  border: 1px solid rgba(255,255,255,0.14);
  box-shadow: inset 0 1px 0 rgba(255,255,255,0.14), 0 8px 22px rgba(0,0,0,0.26);
}
.app-window.light-mode .send-select-button {
  background: linear-gradient(180deg, rgba(118,94,228,0.24) 0%, rgba(255,255,255,0.62) 100%);
  border: 1px solid rgba(98,76,188,0.22);
  box-shadow: 0 8px 22px rgba(76,58,150,0.14);
}
.app-window .clear-files-button {
  border-radius: 999px;
  transition: background 160ms ease, transform 130ms cubic-bezier(0.34,1.56,0.64,1);
}
.app-window.dark-mode  .clear-files-button { color: #ff8d86; }
.app-window.light-mode .clear-files-button { color: #c53030; }
.app-window.dark-mode  .clear-files-button:hover { background: rgba(255,107,95,0.14); transform: scale(1.06); }
.app-window.light-mode .clear-files-button:hover { background: rgba(197,48,48,0.12);  transform: scale(1.06); }

/* ── Selected file tiles ────────────────────────────────────── */
.app-window .selected-file-overlay  { margin: 1px 5px 5px 1px; }
.app-window .selected-file-tile {
  border-radius: 16px;
  min-width: 52px;
  min-height: 52px;
  padding: 0;
  transition: transform 160ms cubic-bezier(0.34,1.56,0.64,1);
}
.app-window.dark-mode .selected-file-tile {
  background: linear-gradient(180deg, rgba(255,255,255,0.11) 0%, rgba(76,58,155,0.26) 100%);
  border: 1px solid rgba(255,255,255,0.12);
  box-shadow: inset 0 1px 0 rgba(255,255,255,0.09), 0 8px 18px rgba(0,0,0,0.16);
}
.app-window.light-mode .selected-file-tile {
  background: linear-gradient(180deg, rgba(255,255,255,0.90) 0%, rgba(218,208,255,0.74) 100%);
  border: 1px solid rgba(104,82,183,0.16);
  box-shadow: 0 8px 18px rgba(78,60,144,0.12);
}
.app-window .selected-file-tile-icon { opacity: 0.95; }
.app-window .selected-file-preview   { border-radius: 12px; }
.app-window .selected-file-remove-badge {
  min-width: 14px;
  min-height: 14px;
  padding: 0;
  border-radius: 999px;
  margin-top: -1px;
  margin-right: -1px;
  box-shadow: 0 1px 4px rgba(0,0,0,0.12);
  transition: transform 130ms cubic-bezier(0.34,1.56,0.64,1), filter 120ms ease;
}
.app-window .selected-file-remove-badge image { -gtk-icon-size: 7px; opacity: 0.90; }
.app-window .selected-file-remove-badge:hover { transform: scale(1.14); filter: brightness(1.12); }
.app-window.dark-mode  .selected-file-remove-badge { background: rgb(185,28,28);  border: 1px solid rgba(255,255,255,0.72); color: white; }
.app-window.light-mode .selected-file-remove-badge { background: rgb(185,28,28);  border: 1px solid rgba(255,255,255,0.80); color: white; }

/* ── Devices Card ───────────────────────────────────────────── */
.app-window .devices-card {
  padding: 14px 14px 10px 14px;
  border-radius: 20px;
}
.app-window .network-summary-card {
  border-radius: 14px;
  padding: 10px 14px;
  margin-top: 2px;
  margin-bottom: 2px;
}
.app-window.dark-mode .network-summary-card {
  background: linear-gradient(180deg, rgba(255,255,255,0.06) 0%, rgba(58,44,118,0.12) 100%);
  border: 1px solid rgba(255,255,255,0.07);
}
.app-window.light-mode .network-summary-card {
  background: linear-gradient(180deg, rgba(255,255,255,0.76) 0%, rgba(224,214,255,0.90) 100%);
  border: 1px solid rgba(108,87,198,0.13);
}
.app-window .network-summary-title    { font-size: 0.87em; font-weight: 700; opacity: 0.80; }
.app-window .network-summary-subtitle { font-size: 0.87em; opacity: 0.82; }

/* ── History ────────────────────────────────────────────────── */
.app-window .history-list { margin-top: 0; }
.app-window .history-button {
  border-radius: 999px;
  padding: 4px 12px;
  font-size: 0.88em;
  font-weight: 700;
  transition: transform 130ms cubic-bezier(0.34,1.56,0.64,1), box-shadow 160ms ease;
}
.app-window .history-button:hover { transform: translateY(-1px); }
.app-window .history-title    { font-weight: 700; font-size: 0.96em; }
.app-window .history-subtitle { opacity: 0.70; font-size: 0.87em; }
.app-window .history-icon-button {
  border-radius: 999px;
  min-width: 32px;
  min-height: 32px;
  padding: 0;
  transition: transform 130ms cubic-bezier(0.34,1.56,0.64,1);
}
.app-window .history-icon-button:hover { transform: scale(1.10); }
.app-window .boxed-list row.history-row {
  border-radius: 14px;
  transition: background 150ms ease;
}
.app-window.dark-mode .boxed-list row.history-row {
  background: linear-gradient(180deg, rgba(255,255,255,0.07) 0%, rgba(56,42,118,0.14) 100%);
  border: 1px solid rgba(255,255,255,0.06);
}
.app-window.light-mode .boxed-list row.history-row {
  background: linear-gradient(180deg, rgba(255,255,255,0.84) 0%, rgba(228,220,255,0.74) 100%);
  border: 1px solid rgba(108,87,198,0.11);
}

/* ── Caption & section headings ─────────────────────────────── */
.app-window .caption-heading {
  letter-spacing: 0.02em;
  font-size: 0.92em;
  font-weight: 700;
  opacity: 0.72;
  margin-top: 6px;
  margin-bottom: 6px;
}

/* ── Generic row hover ──────────────────────────────────────── */
.app-window.dark-mode  .boxed-list row:hover { background: color-mix(in srgb, #ffffff 7%, transparent); }
.app-window.light-mode .boxed-list row:hover { background: color-mix(in srgb, #5e4ab5 9%, white); }

/* ── Transfer states ────────────────────────────────────────── */
.app-window .boxed-list row.transfer-row.transfer-active {
  background: color-mix(in srgb, #5b8ef8 12%, transparent);
  border-radius: 14px;
  border: 1px solid color-mix(in srgb, #93bafd 15%, transparent);
}
.app-window .boxed-list row.transfer-row.transfer-success {
  background: color-mix(in srgb, #22c55e 16%, transparent);
  border-radius: 14px;
  border: 1px solid color-mix(in srgb, #86efac 24%, transparent);
}
.app-window .boxed-list row.transfer-row.transfer-success:hover {
  background: color-mix(in srgb, #22c55e 22%, transparent);
}
.app-window .boxed-list row.transfer-row.transfer-error {
  background: color-mix(in srgb, #ef4444 14%, transparent);
  border-radius: 14px;
  border: 1px solid color-mix(in srgb, #fca5a5 22%, transparent);
}
.app-window .boxed-list row.transfer-row.transfer-error:hover {
  background: color-mix(in srgb, #ef4444 20%, transparent);
}

/* ── Progress bar ───────────────────────────────────────────── */
.app-window progressbar { border-radius: 6px; }
.app-window progressbar trough {
  background: rgba(255,255,255,0.11);
  border-radius: 6px;
  min-height: 5px;
  border: none;
  box-shadow: none;
}
.app-window.light-mode progressbar trough {
  background: rgba(0,0,0,0.08);
}
.app-window progressbar trough progress {
  background: linear-gradient(90deg, #7c52f0, #4ac5f5);
  border-radius: 6px;
  border: none;
  box-shadow: 0 0 8px rgba(122,82,240,0.42);
  transition: min-width 120ms ease;
}

/* ── PIN badge ──────────────────────────────────────────────── */
.app-window .pin-badge {
  font-size: 0.83em;
  font-weight: 700;
  letter-spacing: 0.08em;
  padding: 4px 11px;
  border-radius: 999px;
}
.app-window.dark-mode .pin-badge {
  color: #eee8ff;
  background: linear-gradient(180deg, rgba(255,255,255,0.14) 0%, rgba(142,98,250,0.20) 100%);
  border: 1px solid rgba(255,255,255,0.14);
  box-shadow: inset 0 1px 0 rgba(255,255,255,0.12), 0 4px 12px rgba(18,9,50,0.26);
}
.app-window.light-mode .pin-badge {
  color: #3c2870;
  background: linear-gradient(180deg, rgba(255,255,255,0.90) 0%, rgba(194,180,255,0.60) 100%);
  border: 1px solid rgba(108,88,198,0.18);
  box-shadow: 0 4px 12px rgba(78,58,144,0.12);
}

/* ── Visibility icons ───────────────────────────────────────── */
.app-window .visibility-visible {
  color: #4de8c2;
  -gtk-icon-shadow:
    0 0 8px  rgba(77,232,194,0.95),
    0 0 18px rgba(77,232,194,0.62),
    0 0 32px rgba(77,232,194,0.34);
}
.app-window .visibility-hidden {
  color: #f87171;
  -gtk-icon-shadow:
    0 0 8px  rgba(248,113,113,0.85),
    0 0 18px rgba(248,113,113,0.50),
    0 0 28px rgba(248,113,113,0.26);
}
.app-window .visibility-temporary { color: #cbd5e1; }

/* ── Status page ────────────────────────────────────────────── */
.app-window.dark-mode  .status-page { color: #d6d0ff; }
.app-window.light-mode .status-page { color: #4a3e72; }

/* ── Pill Switcher ──────────────────────────────────────────── */
.app-window.dark-mode .pill-switcher {
  background: linear-gradient(180deg, rgba(255,255,255,0.11) 0%, rgba(58,44,128,0.22) 100%);
  border-radius: 16px;
  padding: 2px;
  box-shadow:
    inset 0 1px 0 rgba(255,255,255,0.10),
    inset 0 -1px 0 rgba(0,0,0,0.14),
    0 16px 38px rgba(0,0,0,0.40);
  border: 1px solid rgba(255,255,255,0.11);
  outline: 1px solid transparent;
}
.app-window.light-mode .pill-switcher {
  background: linear-gradient(180deg, rgba(86,62,192,0.16) 0%, rgba(255,255,255,0.56) 100%);
  border-radius: 12px;
  padding: 2px;
  box-shadow: 0 10px 28px rgba(72,48,154,0.20);
  border: 1px solid rgba(98,76,183,0.16);
  outline: 1px solid transparent;
}
.app-window .pill-switcher .viewswitcher { background: transparent; }
.app-window .pill-switcher .viewswitcher button {
  min-height: 32px;
  min-width: 84px;
  padding: 6px 10px;
  border-radius: 10px;
  box-shadow: none;
  border: none;
  outline: none;
  transition: background 160ms ease, color 160ms ease, box-shadow 160ms ease;
}
.app-window .pill-switcher .viewswitcher button:focus,
.app-window .pill-switcher .viewswitcher button:focus-visible,
.app-window .pill-switcher .viewswitcher button:focus-within { outline: none; box-shadow: none; }
.app-window.dark-mode  .pill-switcher .viewswitcher button { color: rgba(255,255,255,0.72); }
.app-window.light-mode .pill-switcher .viewswitcher button { color: rgba(36,24,90,0.80); }
.app-window.dark-mode  .pill-switcher .viewswitcher button:hover {
  background: linear-gradient(180deg, rgba(255,255,255,0.11) 0%, rgba(255,255,255,0.04) 100%);
  color: rgba(255,255,255,0.92);
}
.app-window.light-mode .pill-switcher .viewswitcher button:hover {
  background: linear-gradient(180deg, rgba(98,76,188,0.12) 0%, rgba(255,255,255,0.36) 100%);
  color: rgba(36,24,90,1.0);
}
.app-window.dark-mode .pill-switcher .viewswitcher button:checked {
  background: linear-gradient(165deg, rgba(115,88,228,0.96) 0%, rgba(77,54,168,0.98) 100%);
  color: #fff;
  box-shadow:
    inset 0 1px 0 rgba(255,255,255,0.18),
    inset 0 -1px 0 rgba(0,0,0,0.16),
    0 4px 14px rgba(88,62,192,0.32);
}
.app-window.light-mode .pill-switcher .viewswitcher button:checked {
  background: linear-gradient(165deg, rgba(115,88,228,0.92) 0%, rgba(77,54,168,0.96) 100%);
  color: #fff;
  box-shadow:
    inset 0 1px 0 rgba(255,255,255,0.28),
    inset 0 -1px 0 rgba(0,0,0,0.08),
    0 6px 18px rgba(88,62,192,0.28);
}
.app-window .pill-switcher .viewswitcher button label { font-size: 1.0em; font-weight: 600; }

/* ── Device Tiles ───────────────────────────────────────────── */
.app-window.dark-mode .device-tile {
  padding: 0;
  background: linear-gradient(155deg, rgba(96,74,182,0.32) 0%, rgba(53,40,112,0.26) 100%);
  border-radius: 16px;
  border: 1px solid rgba(255,255,255,0.09);
  box-shadow: inset 0 1px 0 rgba(255,255,255,0.08), 0 8px 26px rgba(0,0,0,0.26);
  transition:
    background 200ms ease,
    box-shadow 220ms ease,
    transform  200ms cubic-bezier(0.34,1.56,0.64,1),
    border-color 200ms ease;
}
.app-window.dark-mode .device-tile:hover {
  background: linear-gradient(155deg, rgba(118,92,210,0.42) 0%, rgba(66,50,136,0.36) 100%);
  box-shadow:
    inset 0 1px 0 rgba(255,255,255,0.13),
    0 18px 44px rgba(0,0,0,0.32),
    0 0 0 1px rgba(255,255,255,0.11);
  transform: translateY(-3px);
}
.app-window.dark-mode .device-tile:active { transform: translateY(-1px); }
.app-window.light-mode .device-tile {
  padding: 0;
  background: linear-gradient(155deg, rgba(255,255,255,0.78) 0%, rgba(236,230,255,0.94) 100%);
  color: #281e56;
  border-radius: 16px;
  border: 1px solid rgba(112,90,190,0.17);
  box-shadow: 0 8px 26px rgba(78,58,144,0.15);
  transition:
    background 200ms ease,
    box-shadow 220ms ease,
    transform  200ms cubic-bezier(0.34,1.56,0.64,1);
}
.app-window.light-mode .device-tile:hover {
  background: linear-gradient(155deg, rgba(255,255,255,0.92) 0%, rgba(246,242,255,0.99) 100%);
  box-shadow: 0 20px 44px rgba(78,58,144,0.22);
  transform: translateY(-3px);
}
.app-window.light-mode .device-tile:active { transform: translateY(-1px); }
.app-window.light-mode .device-tile image,
.app-window.light-mode .device-tile label { color: #281e56; }
.app-window.dark-mode  .device-tile image,
.app-window.dark-mode  .device-tile label { color: #f0ecff; }

/* icon circle background */
.app-window.dark-mode .device-icon-circle {
  background: linear-gradient(145deg, rgba(255,255,255,0.10) 0%, rgba(80,60,178,0.20) 100%);
  border-radius: 999px;
  border: 1px solid rgba(255,255,255,0.10);
}
.app-window.light-mode .device-icon-circle {
  background: linear-gradient(145deg, rgba(255,255,255,0.82) 0%, rgba(210,194,255,0.62) 100%);
  border-radius: 999px;
  border: 1px solid rgba(138,110,210,0.15);
}

.app-window .device-tile-title { font-weight: 700; letter-spacing: -0.01em; }
.app-window .device-tile-meta  { font-size: 0.80em; opacity: 0.68; }
.app-window .device-transport-badge {
  padding: 3px 9px;
  border-radius: 999px;
  font-size: 0.76em;
  font-weight: 700;
  letter-spacing: 0.02em;
}
.app-window.dark-mode  .transport-wifi         { background: rgba(77,232,194,0.12);  color: #80f5e0; border: 1px solid rgba(77,232,194,0.22); }
.app-window.light-mode .transport-wifi         { background: rgba(20,158,128,0.10);  color: #0f7c65; border: 1px solid rgba(20,158,128,0.18); }
.app-window.dark-mode  .transport-wifi-direct  { background: rgba(96,165,250,0.12);  color: #aaceff; border: 1px solid rgba(96,165,250,0.22); }
.app-window.light-mode .transport-wifi-direct  { background: rgba(64,108,225,0.10);  color: #2a4ea0; border: 1px solid rgba(64,108,225,0.18); }
"#
    .replace("__FONT_SIZE_PX__", &font_size_px.to_string());

    APP_CSS_PROVIDER.with(|cell| {
        let provider = cell.borrow_mut().take().unwrap_or_else(gtk4::CssProvider::new);
        provider.load_from_string(&css);
        gtk4::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
        *cell.borrow_mut() = Some(provider);
    });
}

fn register_debug_icon_search_path() {
    #[cfg(debug_assertions)]
    if let Some(display) = gdk::Display::default() {
        let icon_theme = gtk4::IconTheme::for_display(&display);
        icon_theme.add_search_path(format!("{}/data/icons", env!("CARGO_MANIFEST_DIR")));
    }
}

fn sync_theme_class(win: &impl IsA<gtk4::Widget>) {
    let widget = win.as_ref();
    widget.remove_css_class("light-mode");
    widget.remove_css_class("dark-mode");
    if libadwaita::StyleManager::default().is_dark() {
        widget.add_css_class("dark-mode");
    } else {
        widget.add_css_class("light-mode");
    }
}

fn send_notification(
    app: Option<&libadwaita::Application>,
    sender_name: &str,
    transfer_id: &str,
) {
    let Some(app) = app else { return };
    let n = gio::Notification::new("GnomeQS");
    n.set_body(Some(&format!("{sender_name} wants to share")));

    let id_variant = glib::Variant::from(transfer_id);
    n.add_button_with_target_value(
        &tr!("Accept"),
        "app.accept-transfer",
        Some(&id_variant),
    );
    n.add_button_with_target_value(
        &tr!("Decline"),
        "app.reject-transfer",
        Some(&id_variant),
    );

    app.send_notification(Some(transfer_id), &n);
}
