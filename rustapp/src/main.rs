use std::env;
use dotenv::dotenv;
use ureq;
use std::io::Read;
use std::time::Instant;
use eframe::egui;
use std::thread;
use std::sync::mpsc;
use serde_json::Value;
use std::time::Duration;
use egui_commonmark::*;



fn main() -> eframe::Result<()> {
    dotenv().ok();
    let (tx, rx) = mpsc::channel::<String>();


    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([320.0, 240.0]),
        ..Default::default()
    };
    eframe::run_native(
        "My egui App",
        options,
        Box::new(move |cc| {
            Box::new(MyApp::new(tx, rx))
        }),
    )
}


struct MyApp {
    query: String,
    output: String,
    tx: mpsc::Sender<String>,
    rx: mpsc::Receiver<String>,
    first_paint: bool,
    mdCache: CommonMarkCache,
}

impl MyApp {
    fn new(tx: mpsc::Sender<String>, rx: mpsc::Receiver<String>) -> Self {
        Self {
            query: "".to_owned(),
            output: "".to_owned(),
            tx,
            rx,
            first_paint: true,
            mdCache: CommonMarkCache::default(),
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
            // ui.heading("Wirth");
            ui.horizontal(|ui| {
                let response = ui.add(egui::TextEdit::singleline(&mut self.query).hint_text("I'm trying to figure out..."));
                if self.first_paint {
                    response.request_focus();
                    self.first_paint = false;
                }

                if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)){
                    self.output = String::new();
                    let query_cloned = self.query.clone();
                    let tx_cloned = self.tx.clone();
                    thread::spawn(move || {
                        make_llm_call(query_cloned, tx_cloned);
                    });
                }
            });

            egui::ScrollArea::vertical().show(ui, |ui| {
                CommonMarkViewer::new("viewer")
                    .max_image_width(Some(512))
                    .show(ui, &mut self.mdCache, &self.output);
            });

        });
    }
}



fn print_headers(res: Result<ureq::Response, ureq::Error>){
    // Access and print response headers
    if let Ok(res) = &res {
        for header in res.headers_names() {
            if let Some(value) = res.header(&header) {
                println!("Header: {} - Value: {}", header, value);
            }
        }
    }
}

fn print_env_var_found(env_var: &str){
    println!("check for anthropic key");
    match env::var(env_var) {
        Ok(mut value) => {
            let len = value.len();
            if len > 40 {
                value.replace_range(len-40.., "**********")
            }
            println!("MY_VARIABLE: {}", value);
        },
        Err(_) => println!("MY_VARIABLE not set"),
    };
}

fn make_llm_call(query: String, tx: mpsc::Sender<String>){
    println!("anthropic request test");
    print_env_var_found("ANTHROPIC_API_KEY");

    let api_key: String = match env::var("ANTHROPIC_API_KEY") {
        Ok(value) => value,
        Err(_) => "Not Found".to_string(),
    };

    let url: &str = "https://api.anthropic.com/v1/messages";
    
    let data = ureq::json!({
        "stream": true,
        "model": "claude-3-5-sonnet-20240620",
        "max_tokens": 1000,
        "temperature": 0,
        "system": "You are a world-class poet. Respond only with short poems. Markdown sugar baby. No need to put the code tags, just return markdown syntaxed, ill parse it.",
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
