use std::error::Error;
use std::fs::File;
use std::io::prelude::*;
use std::net::TcpListener;
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::result::Result;

extern crate base64;

const PATH_PREFIX: &str = "/storage/emulated/0/"; // Directory where to start server

fn mime_type(path: &Path) -> &str {
    let extension = path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("");
    match extension {
        "html" => "text/html",
        "css" => "text/css",
        "js" => "application/javascript",
        "json" => "application/json",
        "png" => "image/png",
        "jpeg" => "image/jpeg",
        "jpg" => "image/jpeg",
        "mp4" => "video/mp4",
        "mkv" => "video/x-matroska",
        "pdf" => "application/pdf",
        "doc" => "application/msword",
        "docx" => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        "xls" => "application/vnd.ms-excel",
        "xlsx" => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        "ppt" => "application/vnd.ms-powerpoint",
        "pptx" => "application/vnd.openxmlformats-officedocument.presentationml.presentation",
        "zip" => "application/zip",
        "rar" => "application/x-rar-compressed",
        "tar" => "application/x-tar",
        "gz" => "application/gzip",
        "gif" => "image/gif",
        "txt" => "text/plain",
        "py" => "text/x-python",
        _ => "application/octet-stream",
    }
}

fn build_response(status_line: &str, filename: &str) -> Result<String, Box<dyn Error>> {
    let mime_type = mime_type(Path::new(filename)).to_string();
    
    if mime_type.starts_with("image") {
        let mut file = match File::open(filename) {
            Ok(file) => file,
            Err(_) => return Ok("".to_string()),
        };
        let mut binary_data = Vec::new();
        file.read_to_end(&mut binary_data)?;
        let response = format!("{}Content-Type: text/html\r\n\r\n<img src='data:{};base64,{}' />", status_line, mime_type, base64::encode(&binary_data));
        Ok(response)
        
    } else {
        let mut file = match File::open(filename) {
            Ok(file) => file,
            Err(_) => return Ok("".to_string()),
        };
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;
        let response = format!(
            "{}Content-Type: {}\r\n\r\n{}",
            status_line, mime_type, contents
        );
        Ok(response)
    }
}

fn handle_connection(mut stream: TcpStream) -> Result<(), Box<dyn Error>> {
    let mut buffer = [0; 1024];
    stream.read(&mut buffer).unwrap();

    let request = String::from_utf8_lossy(&buffer[..]);
    let request_lines: Vec<&str> = request.split("\r\n").collect();
    let request_line = request_lines[0];
    let request_path = &request_line.split_whitespace().nth(1).unwrap()[1..];
    let full_path = PathBuf::from(PATH_PREFIX).join(request_path);
    let response = if full_path.strip_prefix(PATH_PREFIX).is_ok() && full_path.is_file() {
        let status_line = "HTTP/1.1 200 OK\r\n";
        build_response(status_line, full_path.to_str().unwrap())
    } else if full_path.strip_prefix(PATH_PREFIX).is_ok() && full_path.is_dir() {
        let mut entries = vec![];
        for entry in full_path.read_dir().unwrap() {
            let entry = entry.unwrap();
            let entry_path = entry.path();
            let entry_path = entry_path.strip_prefix(PATH_PREFIX).unwrap();
            entries.push(entry_path.display().to_string());
        }
        let entries = entries
            .iter()
            .map(|s| format!("<li><a href='/{}'>{}</a></li>", s, s))
            .collect::<String>();
        let body = format!("<html><body><h1>Directory Listing</h1><ul>{}</ul></body></html>", entries);
        let status_line = "HTTP/1.1 200 OK\r\n";
        let response = format!("{}Content-Type: text/html\r\n\r\n{}", status_line, body);
        Ok(response)
    } else {
        Ok("HTTP/1.1 404 Not Found\r\n\r\n".to_string())
    };

    match response {
        Ok(response) => {
            stream.write(response.as_bytes()).unwrap();
            stream.flush().unwrap();
        }
        Err(_) => {}
    };
    
    Ok(())
}

fn main() {
    let listener = TcpListener::bind("127.0.0.1:8080").unwrap();
    println!("Server listening at http://127.0.0.1:8080");
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                match handle_connection(stream) {
                    Ok(_) => {}
                    Err(e) => println!("Error: {}", e),
                };
            }
            Err(e) => {
                println!("Error: {}", e);
                break;
            }
        }
    }
}