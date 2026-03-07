use std::fs;

fn read_file(path: &str) -> Result<String, std::io::Error> {
    fs::read_to_string(path)
}

fn read_file_safe(path: &str) -> String {
    match read_file(path) {
        Ok(content) => content,
        Err(e) => format!("Error: {}", e),
    }
}
