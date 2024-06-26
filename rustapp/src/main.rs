use std::env;
use dotenv::dotenv;
use ureq;
use std::io::Read;
use std::time::Instant;
use eframe::egui::{self, Pos2, Vec2,ViewportCommand};
use std::thread;
use std::sync::mpsc;
use serde_json::Value;
use std::time::Duration;
use egui_commonmark::*;
use global_hotkey::{GlobalHotKeyManager, GlobalHotKeyEvent, HotKeyState, hotkey::{HotKey, Modifiers, Code}};
use serde::{Serialize, Deserialize};



fn main() -> eframe::Result<()> {
    dotenv().ok();
    let (tx, rx) = mpsc::channel::<String>();

    let manager = GlobalHotKeyManager::new().unwrap();
    let hotkey = HotKey::new(Some(Modifiers::CONTROL), Code::KeyJ);
    manager.register(hotkey);


    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([360.0, 40.0]).with_always_on_top().with_position(Pos2{x: 1200.0, y: 10.0}).with_transparent(true),
        ..Default::default()
    };
    eframe::run_native(
        "Wirth",
        options,
        Box::new(move |cc| {
            Box::new(MyApp::new(tx, rx, cc))
        }),
    )
}


struct MyApp {
    query: String,
    output: String,
    tx: mpsc::Sender<String>,
    rx: mpsc::Receiver<String>,
    first_paint: bool,
    md_cache: CommonMarkCache,
    show: bool,
    submitted: bool,
    messages: Vec<Message>
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Message {
    role: String,
    content: String,
}


impl MyApp {
    fn new(tx: mpsc::Sender<String>, rx: mpsc::Receiver<String>, cc: &eframe::CreationContext<'_>) -> Self {
        setup_custom_fonts(&cc.egui_ctx);

        Self {
            query: "".to_owned(),
            output: "".to_owned(),
            tx,
            rx,
            first_paint: true,
            md_cache: CommonMarkCache::default(),
            show: true,
            submitted: false,
            messages: vec![
                Message {role: "user".to_string(), content: "interposition".to_string(),},
                Message {role: "assistant".to_string(), content: "Here's a revised Markdown version with fewer lists:\n\n# Interposition\n\nInterposition is the act of placing something between two other things. This concept can be applied to physical objects, people, or abstract ideas.\n\n## In Physics\n\nIn the realm of physics, interposition manifests when an object blocks the path between a light source and an observer, resulting in the creation of a shadow.\n\n## In Social Contexts\n\nSocially, interposition often takes the form of mediation. This occurs when a third party intervenes to help resolve a conflict between two other parties.".to_string(),},
                Message {role: "user".to_string(), content: "free list".to_string(),},
                Message {role: "assistant".to_string(), content: "A memory management technique used in computer programming. It maintains a list of memory blocks that are currently not in use and available for allocation.\\n\\n```\nHead\n |\n v\n+---+---+    +---+---+    +---+---+\n| S | *-|--->| S | *-|--->| S | X |\n+---+---+    +---+---+    +---+---+\\n\\nS: Size of free block\n*: Pointer to next block\nX: NULL (end of list)\n```".to_string(),},
            ],
        }
    }
}


impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {

        ctx.request_repaint_after(Duration::from_millis(16));

        if let Ok(s) = self.rx.try_recv() {
            if &s == "!STOP!" {
                self.messages.push(Message {
                    role: "assistant".to_owned(),
                    content: self.output.clone(),
                })
            }
            else{
                self.output.push_str(&s);
            }
        }


        egui::CentralPanel::default().show(ctx, |ui| {
            ctx.set_pixels_per_point(2.25);

            // CTRL-J is pressed
            if let Ok(event) = GlobalHotKeyEvent::receiver().try_recv() {
                if event.state == HotKeyState::Pressed {
                    // Hide the app
                    if self.show {
                        ctx.send_viewport_cmd(egui::ViewportCommand::WindowLevel(egui::WindowLevel::AlwaysOnBottom));
                        self.show = false;
                    }
                    // Show the app
                    else{
                        ctx.send_viewport_cmd(egui::ViewportCommand::WindowLevel(egui::WindowLevel::AlwaysOnTop));
                        ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
                        ctx.send_viewport_cmd(ViewportCommand::InnerSize(Vec2 {x: ctx.screen_rect().width(), y: 40.0 }));
                        self.show = true;
                        self.submitted = false;
                        self.first_paint = true;
                        self.query = String::new();
                        self.output = String::new();

                    }
                }
            }

            // The actual UI
            ui.horizontal(|ui| {

                let newline_shortcut = egui::KeyboardShortcut::new(egui::Modifiers::SHIFT, egui::Key::Enter);

                if self.submitted {
                    let query_cloned = self.query.clone();
                    ui.label(query_cloned);
                }
                else{
                    let response = ui.add(egui::TextEdit::multiline(&mut self.query).hint_text("Tell me..").return_key(newline_shortcut).desired_rows(1));
                    if self.first_paint {
                        response.request_focus();
                        self.first_paint = false;
                    }
                }

            });


            let scroll_section = egui::ScrollArea::vertical().auto_shrink([false, true]).show(ui, |ui| {
                CommonMarkViewer::new("viewer")
                    .show(ui, &mut self.md_cache, &self.output);
            });

            let new_y_size = scroll_section.content_size[1]-20.0;//(scroll_section.inner_rect.max[1]-scroll_section.inner_rect.min[1]);
            if new_y_size>0.0 {
                ctx.send_viewport_cmd(ViewportCommand::InnerSize(Vec2 {x: ctx.screen_rect().width(), y: 40.0+new_y_size }));
            }

            // Submitting (pressing enter)
            if ui.input(|i| i.key_pressed(egui::Key::Enter)) && !ui.input(|i| i.modifiers.shift){
                // Clear and clean to prepate
                ctx.send_viewport_cmd(ViewportCommand::InnerSize(Vec2 {x: ctx.screen_rect().width(), y: 40.0 }));
                self.submitted = true;
                self.output = String::new();

                // Get read to send new message
                let query_cloned = self.query.clone();
                let tx_cloned = self.tx.clone();
                self.messages.push(Message{
                    role: "user".to_owned(),
                    content: "Thats a great answer and format, can you do the same for: ".to_owned() + &query_cloned,
                });
                let cloned_messages: Vec<Message> = self.messages.clone();

                thread::spawn(move || {
                    make_llm_call(cloned_messages, tx_cloned);
                });
            }

        });

    }
}


fn make_llm_call(messages: Vec<Message>, tx: mpsc::Sender<String>){

    let api_key: String = match env::var("ANTHROPIC_API_KEY") {
        Ok(value) => value,
        Err(_) => "Not Found".to_string(),
    };

    let url: &str = "https://api.anthropic.com/v1/messages";

    let mut serialized_messages: Vec<Value> = vec![];
    for message in messages {
        serialized_messages.push(serde_json::to_value(&message).unwrap());
    }

    
    let data = ureq::json!({
        "stream": true,
        "model": "claude-3-5-sonnet-20240620",
        "max_tokens": 1000,
        "temperature": 0.5,
        "system": "Provide a concise explanation. Strict Word Economy is applied: be concise and direct, avoiding introductory phrases or redundant wording. Don't use Anaphora. Begin with a clear, 1-3 sentence explanation of the topic, ensuring the content is accessible to a grade 12 level of understanding. When elaboration is requested, continue to add essential details to deepen understanding, while maintaining simplicity and brevity. Each added detail must be directly relevant and informative. Commonmark is supported. Dont include the markdown start and end tags, the entire response will be parsed automatically.",
        "messages": serialized_messages
    });

    let start = Instant::now();

    let response: Result<ureq::Response, ureq::Error> = ureq::post(url)
        .set("x-api-key", &api_key)
        .set("Content-Type", "application/json")
        .set("anthropic-version", "2023-06-01")
        .send_json(data);

    // print_headers(response);

    let mut first_read_done = false; // Flag to check if first read is done
    let mut time_to_first_token: u64 = 0;

    match response {
        Ok(res) => {
            let mut reader = res.into_reader();
            let mut buffer = [0; 8192]; // 8KB buffer


            loop {
                match reader.read(&mut buffer) {
                    Ok(0) => break, // EOF
                    Ok(n) => {
                        // Process the n bytes read into buffer
                        // println!("Read {} bytes", n);

                        if !first_read_done {
                            time_to_first_token = start.elapsed().as_secs();
                            println!("Time taken for first read: {:?}", time_to_first_token);
                            first_read_done = true; // Set the flag after first read
                        }

                        if let Ok(s) = std::str::from_utf8(&buffer[..n]) {

                            let json_objects: Vec<Value> = s
                                .lines()
                                .filter_map(|line| line.split_once("data:").map(|(_, json)| json))
                                .filter_map(|line| serde_json::from_str(line).ok())
                                .collect();

                            
                            for json_object in &json_objects {
                                if json_object.get("delta").is_some() && json_object.get("delta").unwrap().get("text").is_some() {
                                    let mut popped_string = json_object["delta"]["text"].to_string();
                                    popped_string.pop();
                                    tx.send(popped_string[1..].replace("\\n", "\n").replace("\\\"", "\"")).ok();
                                } 
                                else if json_object.get("type").is_some() && json_object["type"]=="message_stop" {
                                    tx.send("!STOP!".to_string()).ok();
                                }
                            }

                            

                        }
                    },
                    Err(e) => break,
                }
            }
        },
        Err(error) => println!("error occured: {}", error),
    }
}

fn setup_custom_fonts(ctx: &egui::Context) {
    // Start with the default fonts (we will be adding to them rather than replacing them).
    let mut fonts = egui::FontDefinitions::default();

    // Install my own font (maybe supporting non-latin characters).
    // .ttf and .otf files supported.
    fonts.font_data.insert(
        "AeonikBold".to_owned(),
        egui::FontData::from_static(include_bytes!(
            "./Aeonik-Bold.ttf"
        )),
    );

    fonts.font_data.insert(
        "GT-Sectra".to_owned(),
        egui::FontData::from_static(include_bytes!(
            "./GT-Sectra-Fine-Regular.ttf"
        )),
    );

    // Put my font first (highest priority) for proportional text:
    fonts
        .families
        .entry(egui::FontFamily::Proportional)
        .or_default()
        .insert(1, "AeonikBold".to_owned());

    fonts
        .families
        .entry(egui::FontFamily::Proportional)
        .or_default()
        .insert(0, "GT-Sectra".to_owned());

    // Put my font as last fallback for monospace:
    fonts
        .families
        .entry(egui::FontFamily::Monospace)
        .or_default()
        .push("GT-Sectra".to_owned());

    // Tell egui to use these fonts:
    ctx.set_fonts(fonts);
}