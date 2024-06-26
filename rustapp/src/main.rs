use std::env;
use dotenv::dotenv;
use ureq;
use std::io::Read;
use std::time::Instant;


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

fn main() {
    println!("anthropic request test");
    dotenv().ok();

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
        "system": "You are a world-class poet. Respond only with short poems.",
        "messages": [
            {
                "role": "user",
                "content": "Why is the ocean salty?"
            }
        ]
    });

    let start = Instant::now();

    // try making the request with ureq
    let response: Result<ureq::Response, ureq::Error> = ureq::post(url)
        .set("x-api-key", &api_key)
        .set("Content-Type", "application/json")
        .set("anthropic-version", "2023-06-01")
        .send_json(data);

    // print_headers(response);

    let mut first_read_done = false; // Flag to check if first read is done


    match response {
        Ok(res) => {
            let mut reader = res.into_reader();
            let mut buffer = [0; 8192]; // 8KB buffer


            loop {
                match reader.read(&mut buffer) {
                    Ok(0) => break, // EOF
                    Ok(n) => {
                        // Process the n bytes read into buffer
                        println!("Read {} bytes", n);

                        if !first_read_done {
                            let duration = start.elapsed();
                            println!("Time taken for first read: {:?}", duration);
                            first_read_done = true; // Set the flag after first read
                        }

                        if let Ok(s) = std::str::from_utf8(&buffer[..n]) {
                            println!("As string: {}", s);
                        }
                    },
                    Err(e) => break,
                }
            }
        },
        Err(error) => println!("error occured: {}", error),
    }



    // let body = match response {
    //     Ok(res) => res.into_string().unwrap_or_else(|_| "Error converting response to string".to_string()),
    //     Err(error) => format!("Error Occurred: {}", error),
    // };
        
        
    // println!("Response body: {}", body);


}