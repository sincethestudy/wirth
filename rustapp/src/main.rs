use std::env;
use dotenv::dotenv;
use ureq;

fn main() {
    println!("anthropic request test");
    dotenv().ok();

    println!("check for anthropic key");
    match env::var("ANTHROPIC_API_KEY") {
        Ok(value) => println!("MY_VARIABLE: {}", value),
        Err(_) => println!("MY_VARIABLE not set"),
    };

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

    // try making the request with ureq
    let response: Result<ureq::Response, ureq::Error> = ureq::post(url)
        .set("x-api-key", &api_key)
        .set("Content-Type", "application/json")
        .set("anthropic-version", "2023-06-01")
        .send_json(data);

    let body = match response {
        Ok(res) => res.into_string().unwrap_or_else(|_| "Error converting response to string".to_string()),
        Err(error) => format!("Error Occurred: {}", error),
    };
        
        
    println!("Response body: {}", body);


}