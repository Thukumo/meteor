#![windows_subsystem = "windows"]

use std::{sync::{Arc, Mutex}, thread};

use eframe::egui::{Frame, RichText, ScrollArea, Slider};
use futures_util::{stream::SplitSink, SinkExt, StreamExt};
use tokio::{net::TcpStream, sync::oneshot};
use tokio_tungstenite::{connect_async, tungstenite::Message, MaybeTlsStream, WebSocketStream};

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title("Meteor native app")
            .with_inner_size(eframe::egui::vec2(300.0, 400.0))
            .with_resizable(true),
        ..Default::default()
    };
    eframe::run_native(
        "Meteor native app",
        options,
        Box::new(|_cc| {
            Ok(Box::new(App::new()))
        }),
    )
}

struct App {
    messages: Arc<Mutex<Vec<String>>>,
    url: String,
    thread_handle: Arc<Mutex<Option<std::thread::JoinHandle<()>>>>,
    thread_abort: Option<oneshot::Sender<()>>,
    sender: Arc<Mutex<Option<SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>>>>,
    runtime_for_sender: tokio::runtime::Runtime,
    comment: String,
    font_size: f32,
}

impl App {
    fn new() -> Self {
        Self {
            messages: Arc::new(Mutex::new(Vec::new())),
            url: String::new(),
            thread_handle: Arc::new(Mutex::new(None)),
            thread_abort: None,
            sender: Arc::new(Mutex::new(None)),
            runtime_for_sender: tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime"),
            comment: String::new(),
            font_size: 16.0,
        }
    }
    fn connect(&mut self) {
        if let Some(terminator) = self.thread_abort.take() {
            let _ = terminator.send(());
        }
        let messages_clone = self.messages.clone();
        let connection = connect_async(format!("ws://localhost:3000/{}/api/v1/ws", self.url));
        let sender = self.sender.clone();
        let (terminator, thread_abort) = oneshot::channel::<()>();
        self.thread_abort = Some(terminator);
        *self.thread_handle.lock().unwrap() = Some(std::thread::spawn(move || {
            let runtime = tokio::runtime::Runtime::new().unwrap();
            let sender_clone = sender.clone();
            runtime.block_on(async move {
                tokio::select! {
                    _ = async {
                        let (socket, _) = connection.await.expect("Failed to connect");
                        let (write, mut read) = socket.split();
                        *sender.lock().unwrap() = Some(write);
                        loop {
                            match read.next().await {
                                Some(Ok(Message::Text(text))) => {
                                    let mut messages = messages_clone.lock().unwrap();
                                    messages.push(text.to_string());
                                }
                                Some(Ok(Message::Close(_))) | Some(Err(_)) => break,
                                _ => {} // tokio-tungsteniteもpingに自動で応答するらしい
                            }
                        }
                    } => {},
                    _ = thread_abort => {
                    }
                }
            });
            *sender_clone.lock().unwrap() = None;
        }));
    }
    fn send_message(&mut self, message: String) {
        if let Some(sender) = self.sender.lock().unwrap().as_mut() {
            self.runtime_for_sender.block_on(async move {
                sender.send(Message::Text(message.into())).await.expect("Failed to send message");
            });
        } else {
            self.connect();
            thread::sleep(std::time::Duration::from_secs(1));
            self.send_message(message);
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
        eframe::egui::CentralPanel::default().show(ctx, |ui| {
            if self.sender.lock().unwrap().is_none() {
                ui.label("Enter room name:");
                ui.text_edit_singleline(&mut self.url);
                if ui.button("Connect").clicked() {
                    ui.close_menu();
                    ui.label("Connecting...");
                    self.connect();
                }
            } else {
                Frame::group(ui.style()).show(ui, |ui| {
                    ui.label(RichText::new(format!("room name: {}", self.url)).size(16.0).strong());
                    ui.add(Slider::new(&mut self.font_size, 8.0..=46.0).text("font size"));
                });
                Frame::group(ui.style()).show(ui, |ui| {
                    ui.label("Send a message:");
                    ui.text_edit_singleline(&mut self.comment);
                    if ui.button("Send").clicked() {
                        self.send_message(self.comment.clone());
                        self.comment.clear();
                    }
                });
                ScrollArea::vertical().show(ui, |ui| {
                    for message in self.messages.lock().unwrap().iter().rev() {
                        ui.separator();
                        ui.label(RichText::new(message).size(self.font_size).strong());
                    }
                });
            }
            ctx.request_repaint_after_secs(0.2); // フォーカスが外れている際も、0.2秒に1回は画面を更新する
        });
    }
}
