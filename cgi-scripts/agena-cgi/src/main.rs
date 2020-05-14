use std::env;

// Base url of agena proxy
const BASE: &'static str = "gemini://example.com/";

fn main() {
    let query = match env::var("QUERY_STRING") {
        Ok(q) => q,
        _ => {
            println!("10\tGopher url:\r\n");
            return;
        }
    };
    println!("30\t{}{}\r\n", BASE, query);
}
