use std::env::current_dir;

pub fn get_current_dir() -> String {
    match current_dir() {
        Ok(x) => return String::from(x.to_str().unwrap()),
        Err(e) => eprintln!("Error getting current directory: {}", e),
    }
    String::new()
}