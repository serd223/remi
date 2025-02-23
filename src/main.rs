#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

mod gemtext;
mod response;
mod verifier;
use std::{
    error::Error,
    io::{Read, Write},
    net::TcpStream,
    str::FromStr,
    sync::Arc,
};

use eframe::egui::{self, Color32, Key, RichText};
use gemtext::{Gemtext, GemtextEntry};
use response::{GeminiResponse, PermanentFailureKind, RedirectionKind};
use rustls::RootCertStore;
use verifier::GeminiCertVerifier;

const BG_COLOR: Color32 = Color32::from_rgb(40, 44, 52);
const PREFORMATTED_BG_COLOR: Color32 = Color32::from_rgb(25, 27, 31);
const RED_COLOR: Color32 = Color32::from_rgb(190, 96, 105);
const TEXT_COLOR: Color32 = Color32::from_rgb(171, 178, 191);
const PREFORMATTED_TEXT_COLOR: Color32 = Color32::from_rgb(156, 163, 176);
const LINK_COLOR: Color32 = Color32::from_rgb(86, 182, 194);
const LIST_ELEM_COLOR: Color32 = Color32::from_rgb(201, 208, 221);

const TEXT_SIZE: f32 = 20.;
const MINOR_SIZE: f32 = 30.;
const MEDIUM_SIZE: f32 = 40.;
const MAJOR_SIZE: f32 = 50.;

const DEFAULT_SERVER: &str = "geminiprotocol.net";
const DEFAULT_URL: &str = "gemini://geminiprotocol.net/";

const BOOKMARKS_STORE_KEY: &str = "bookmarks";

struct App {
    server_name: String,
    request_data: String,
    url_bar_data: String,
    gemtext: Gemtext,
    bookmarks: Vec<String>,
    moving_in_history: bool,
    history: Vec<(String, String)>,
    history_index: usize,
    redir: bool,
}

fn main() -> eframe::Result {
    rustls::crypto::ring::default_provider()
        .install_default()
        .unwrap();
    let mut server_name = DEFAULT_SERVER.to_string(); // make sure this isn't dropped
    let mut request_data = DEFAULT_URL.to_string();
    let url_bar_data = request_data.clone();
    let gemtext = Gemtext {
        data: vec![GemtextEntry::Text(
            "You shouldn't be seeing this".to_string(),
        )],
    };
    let mut bookmarks = Vec::<String>::new();
    let history = Vec::<(String, String)>::new();
    let history_index = 0;
    let mut redir = true;
    let moving_in_history = false;
    {
        let mut args = std::env::args();
        args.next();
        if let Some(url) = args.next() {
            redir = redirect(&mut server_name, &mut request_data, &url);
        }
    }

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([800.0, 600.0]),
        ..Default::default()
    };

    eframe::run_native(
        "remi",
        options,
        Box::new(move |cc| {
            if let Some(storage) = cc.storage {
                if let Some(bookmarks_raw) = storage.get_string(BOOKMARKS_STORE_KEY) {
                    bookmarks = bookmarks_raw.lines().map(|l| l.to_string()).collect();
                }
            }
            Ok(Box::new(App {
                server_name,
                request_data,
                url_bar_data,
                gemtext,
                bookmarks,
                moving_in_history,
                history,
                history_index,
                redir,
            }))
        }),
    )
}

impl eframe::App for App {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        let mut bookmarks_raw = String::new();
        for bookmark in &self.bookmarks {
            bookmarks_raw.push_str(bookmark);
            bookmarks_raw.push('\n');
        }
        bookmarks_raw.pop();
        storage.set_string(BOOKMARKS_STORE_KEY, bookmarks_raw);
    }
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let mut reset_scroll = false;
        if self.redir {
            self.redir = false;

            match request(&self.server_name, self.request_data.as_bytes()) {
                Ok(response) => match response {
                    GeminiResponse::Success { body } => {
                        self.gemtext = Gemtext::from_str(&body)
                            .expect("[ERROR] Data received is not valid Gemtext.");
                        reset_scroll = true;
                        if !self.moving_in_history {
                            self.history.truncate(self.history_index + 1);
                            self.history
                                .push((self.server_name.clone(), self.request_data.clone()));
                            self.history_index = self.history.len() - 1;
                        } else {
                            self.moving_in_history = false;
                        }
                    }
                    GeminiResponse::PermanentFailure {
                        kind: PermanentFailureKind::NotFound,
                        msg,
                    } => {
                        eprintln!(
                            "[ERROR] '{:?}' couldn't be found: '{msg}'",
                            self.request_data
                        );
                        self.moving_in_history = false;
                        self.server_name = self.history[self.history_index].0.clone();
                        self.request_data = self.history[self.history_index].1.clone();
                    }
                    GeminiResponse::Redirection {
                        kind: RedirectionKind::Permanent,
                        to,
                    } => {
                        self.redir = redirect(&mut self.server_name, &mut self.request_data, &to);
                        if !self.redir {
                            todo!("[TODO] Handle incorrect permanent redirection gracefully");
                        }
                    }
                    _ => panic!("[ERROR] Unsupported response: {response:?}"),
                },
                Err(e) => {
                    eprintln!(
                        "[ERROR] Request error from server '{}' with request '{}': {e}",
                        self.server_name, self.request_data
                    );
                    if self.history.len() == 0 {
                        self.history
                            .push((DEFAULT_SERVER.to_string(), DEFAULT_URL.to_string()));
                        self.redir = true;
                    }
                    self.server_name = self.history[self.history_index].0.clone();
                    self.request_data = self.history[self.history_index].1.clone();
                }
            }
            self.url_bar_data = self.request_data.clone();
        }

        ctx.style_mut(|style| {
            style.visuals.panel_fill = BG_COLOR;
            style.visuals.window_fill = BG_COLOR;
            style.visuals.hyperlink_color = LINK_COLOR;
        });

        egui::TopBottomPanel::top("url_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui
                    .button(RichText::new("<").size(TEXT_SIZE).color(TEXT_COLOR))
                    .clicked()
                {
                    if self.history_index > 0 {
                        self.history_index -= 1;
                        self.moving_in_history = true;
                    }
                }

                if ui
                    .button(RichText::new(">").size(TEXT_SIZE).color(TEXT_COLOR))
                    .clicked()
                {
                    if self.history_index + 1 < self.history.len() {
                        self.history_index += 1;
                        self.moving_in_history = true;
                    }
                }

                let popup_button_response =
                    ui.button(RichText::new("#").size(TEXT_SIZE).color(TEXT_COLOR));
                let popup_id = ui.make_persistent_id("bookmarks_popup");
                if popup_button_response.clicked() {
                    ui.memory_mut(|mem| mem.toggle_popup(popup_id))
                }
                egui::popup::popup_below_widget(
                    ui,
                    popup_id,
                    &popup_button_response,
                    egui::PopupCloseBehavior::CloseOnClickOutside,
                    |ui| {
                        ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
                        ui.style_mut().visuals.widgets.inactive.weak_bg_fill = BG_COLOR;
                        ui.style_mut().visuals.widgets.hovered.weak_bg_fill = BG_COLOR;
                        let mut bookmark_to_remove = None;
                        for (i, bookmark) in self.bookmarks.iter().enumerate() {
                            let response = ui
                                .button(RichText::new(bookmark).size(TEXT_SIZE).color(TEXT_COLOR));
                            if response.clicked() {
                                self.redir = redirect(
                                    &mut self.server_name,
                                    &mut self.request_data,
                                    bookmark,
                                );
                            }
                            if response.secondary_clicked() {
                                bookmark_to_remove = Some(i);
                            }
                        }
                        if let Some(i) = bookmark_to_remove {
                            self.bookmarks.remove(i);
                        }
                        ui.style_mut().wrap_mode = None;
                    },
                );

                if ui
                    .button(RichText::new("+").size(TEXT_SIZE).color(TEXT_COLOR))
                    .clicked()
                    && !self.bookmarks.contains(&self.request_data)
                {
                    self.bookmarks.push(self.request_data.clone())
                }

                ui.style_mut().override_font_id = Some(egui::FontId {
                    size: TEXT_SIZE,
                    family: egui::FontFamily::Proportional,
                });
                let text_edit = egui::TextEdit::singleline(&mut self.url_bar_data)
                    .text_color(LIST_ELEM_COLOR)
                    .desired_width(f32::INFINITY);
                let lost_focus = ui.add(text_edit).lost_focus();
                ui.style_mut().override_font_id = None;
                if lost_focus && ui.input(|i| i.key_pressed(Key::Enter)) {
                    self.redir = redirect(
                        &mut self.server_name,
                        &mut self.request_data,
                        &self.url_bar_data,
                    );
                    if !self.redir {
                        self.url_bar_data = self.request_data.clone();
                    }
                };
            })
        });
        if self.moving_in_history {
            let (new_sn, new_data) = self.history[self.history_index].clone();
            self.server_name = new_sn;
            self.request_data = new_data;
            self.redir = true;
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.style_mut().visuals.widgets.inactive.weak_bg_fill = BG_COLOR;
            ui.style_mut().visuals.widgets.hovered.weak_bg_fill = BG_COLOR;
            let mut scroll_area = egui::ScrollArea::vertical()
                .auto_shrink(false)
                .stick_to_right(true);
            if reset_scroll {
                scroll_area = scroll_area.scroll_offset(egui::Vec2 { x: 0., y: 0. })
            }
            scroll_area.show(ui, |ui| {
                for (i, g) in self.gemtext.data.iter().enumerate() {
                    match g {
                        gemtext::GemtextEntry::Text(t) => {
                            ui.label(RichText::new(t).size(TEXT_SIZE).color(TEXT_COLOR));
                        }
                        gemtext::GemtextEntry::Link { url, label } => {
                            ui.horizontal(|ui| {
                                let text = RichText::new(url).size(TEXT_SIZE).color(LINK_COLOR);
                                let response = ui.link(text.clone());
                                if response.clicked() {
                                    self.redir = redirect(
                                        &mut self.server_name,
                                        &mut self.request_data,
                                        url,
                                    );
                                }
                                ui.label(RichText::new(label).size(TEXT_SIZE).color(TEXT_COLOR))
                            });
                        }
                        gemtext::GemtextEntry::MinorHeading(h) => {
                            ui.label(RichText::new(h).size(MINOR_SIZE).color(RED_COLOR));
                        }
                        gemtext::GemtextEntry::MediumHeading(h) => {
                            ui.label(RichText::new(h).size(MEDIUM_SIZE).color(RED_COLOR));
                        }
                        gemtext::GemtextEntry::MajorHeading(h) => {
                            ui.label(RichText::new(h).size(MAJOR_SIZE).color(RED_COLOR));
                        }
                        gemtext::GemtextEntry::List(elems) => {
                            for el in elems {
                                ui.label(
                                    RichText::new(format!("* {el}"))
                                        .size(TEXT_SIZE)
                                        .color(LIST_ELEM_COLOR),
                                );
                            }
                        }
                        gemtext::GemtextEntry::Quote(q) => {
                            ui.label(
                                RichText::new(q)
                                    .size(TEXT_SIZE)
                                    .color(TEXT_COLOR)
                                    .background_color(PREFORMATTED_BG_COLOR),
                            );
                        }
                        gemtext::GemtextEntry::Preformatted { alt_text: _, body } => {
                            ui.push_id(i, |ui| {
                                // Idea taken from egui::TextEdit::show
                                let where_to_put_background = ui.painter().add(egui::Shape::Noop);
                                let output = egui::ScrollArea::horizontal()
                                    .auto_shrink([false, true])
                                    .drag_to_scroll(false)
                                    .min_scrolled_width(0.)
                                    .scroll_bar_visibility(
                                        egui::scroll_area::ScrollBarVisibility::AlwaysVisible,
                                    )
                                    .show(ui, |ui| {
                                        ui.add(
                                            egui::Label::new(
                                                RichText::new(body)
                                                    .monospace()
                                                    .size(TEXT_SIZE)
                                                    .color(PREFORMATTED_TEXT_COLOR),
                                            )
                                            .selectable(true)
                                            .extend(),
                                        )
                                    });
                                let rect = output.inner.rect.with_max_x(output.inner_rect.max.x);
                                ui.painter().set(
                                    where_to_put_background,
                                    egui::epaint::RectShape::filled(rect, 0, PREFORMATTED_BG_COLOR),
                                );
                            });
                        }
                    }
                }
            });
        });
    }
}

pub fn request(server_name: &str, data: &[u8]) -> Result<GeminiResponse, Box<dyn Error>> {
    let recv = request_raw(server_name, data)?;
    Ok(GeminiResponse::from_bytes(&recv)?)
}

pub fn request_raw(server_name: &str, data: &[u8]) -> Result<Vec<u8>, Box<dyn Error>> {
    let server_name = unsafe {
        std::str::from_utf8(std::slice::from_raw_parts(
            server_name.as_ptr(),
            server_name.len(),
        ))
        .unwrap()
    };
    let mut config = rustls::ClientConfig::builder()
        .with_root_certificates(RootCertStore::empty())
        .with_no_client_auth();
    // Completely disables all verification
    config
        .dangerous()
        .set_certificate_verifier(Arc::new(GeminiCertVerifier {}));
    let (server_name, port) = {
        if let Some(split) = server_name.split_once(':') {
            split
        } else {
            (server_name, "1965")
        }
    };
    let mut conn = rustls::ClientConnection::new(Arc::new(config), server_name.try_into()?)?;
    let mut sock = TcpStream::connect(&format!("{server_name}:{port}"))?;

    let mut tls = rustls::Stream::new(&mut conn, &mut sock);
    let mut data = std::str::from_utf8(data)?.trim_end().to_string();
    data.push('\r');
    data.push('\n');
    tls.write_all(data.as_bytes())?;
    let mut recv = Vec::new();
    tls.read_to_end(&mut recv)?;
    Ok(recv)
}

pub fn redirect(server_name: &mut String, request_data: &mut String, url: &str) -> bool {
    if url.contains("://") {
        if url.starts_with("gemini://") {
            if url.len() <= 9 {
                eprintln!("[ERROR] '{url}' is invalid.");
                return false;
            } else {
                *server_name = url[9..].to_string();
                while server_name.contains('/') {
                    server_name.pop();
                }
                *request_data = url.to_string();
                return true;
            }
        } else {
            eprintln!("[ERROR] '{url}' contains unsupported protocol.");
            return false;
        }
    } else if url.starts_with("/") {
        *request_data = format!("gemini://{server_name}{url}");
        return true;
    } else if url.ends_with(".gmi") {
        if request_data.ends_with(".gmi") {
            *request_data = request_data.trim_end_matches(|c| c != '/').to_string();
        }

        request_data.push_str(&url);
        return true;
    } else {
        if !request_data.ends_with('/') {
            request_data.push('/');
        }
        request_data.push_str(&url);
        return true;
    }
}
