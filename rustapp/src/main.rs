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



fn main() -> eframe::Result<()> {
    dotenv().ok();
    let (tx, rx) = mpsc::channel::<String>();


    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([320.0, 240.0]).with_always_on_top().with_position(Pos2{x: 1200.0, y: 10.0}),
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
        }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {

        ctx.request_repaint_after(Duration::from_millis(16));

        if let Ok(s) = self.rx.try_recv() {
            self.output.push_str(&s);
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ctx.set_pixels_per_point(2.5);

            ui.horizontal(|ui| {

                let response = ui.add(egui::TextEdit::singleline(&mut self.query).hint_text("Tell me.."));
                if self.first_paint {
                    response.request_focus();
                    self.first_paint = false;
                }

                if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)){
                    self.output = String::new();
                    ctx.send_viewport_cmd(ViewportCommand::InnerSize(Vec2 {x: 280.0 , y: 240.0 }));
                    let query_cloned = self.query.clone();
                    let tx_cloned = self.tx.clone();
                    thread::spawn(move || {
                        make_llm_call(query_cloned, tx_cloned);
                    });
                }
            });


            let scroll_section = egui::ScrollArea::vertical().auto_shrink([false, true]).show(ui, |ui| {
                CommonMarkViewer::new("viewer")
                    .show(ui, &mut self.md_cache, &self.output);
            });

            let new_y_size = scroll_section.content_size[1]-200.0;//(scroll_section.inner_rect.max[1]-scroll_section.inner_rect.min[1]);
            if new_y_size>0.0 {
                ctx.send_viewport_cmd(ViewportCommand::InnerSize(Vec2 {x: 280.0 , y: 240.0+new_y_size }));
            }

        });
    }
}


fn make_llm_call(query: String, tx: mpsc::Sender<String>){

    let api_key: String = match env::var("ANTHROPIC_API_KEY") {
        Ok(value) => value,
        Err(_) => "Not Found".to_string(),
    };

    let url: &str = "https://api.anthropic.com/v1/messages";
    
    let data = ureq::json!({
        "stream": true,
        "model": "claude-3-5-sonnet-20240620",
        "max_tokens": 1000,
        "temperature": 0.5,
        "system": "The user is an engineer is working on a project. It could be electrical, mechanical, or software. They dont like using lots of words, so they will ask questions with few keywords so they can ask quickly. Provide a concise explanation. Strict Word Economy is applied: be concise and direct, avoiding introductory phrases or redundant wording. Don't use Anaphora. Use markdown features (not just lists) to produce a clean, formatted answer to me. Dont include the markdown start and end tags, the entire response will be parsed automatically.",
        "messages": [
            {
                "role": "user",
                "content": query 
            }
        ]
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
                                    tx.send(json_object["delta"]["text"].to_string().replace("\\n", "\n").replace("\"", "")).ok();
                                } 
                                // else {
                                    // println!("Data object does not exist.");
                                // }
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
        "my_font".to_owned(),
        egui::FontData::from_static(include_bytes!(
            "./InterVariable.ttf"
        )),
    );

    // Put my font first (highest priority) for proportional text:
    fonts
        .families
        .entry(egui::FontFamily::Proportional)
        .or_default()
        .insert(0, "my_font".to_owned());

    // Put my font as last fallback for monospace:
    fonts
        .families
        .entry(egui::FontFamily::Monospace)
        .or_default()
        .push("my_font".to_owned());

    // Tell egui to use these fonts:
    ctx.set_fonts(fonts);
}