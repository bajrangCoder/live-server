use std::error::Error;
use std::fs::File;
use std::io::prelude::*;
use std::net::TcpListener;
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::result::Result;
use std::env;

extern crate base64;


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
    let path_prefix: PathBuf = env::current_dir().unwrap(); // Directory in which the program runs
    let request = String::from_utf8_lossy(&buffer[..]);
    let request_lines: Vec<&str> = request.split("\r\n").collect();
    let request_line = request_lines[0];
    let request_path = &request_line.split_whitespace().nth(1).unwrap()[1..];
    let full_path = PathBuf::from(&path_prefix).join(request_path);
    let response = if full_path.strip_prefix(&path_prefix).is_ok() && full_path.is_file() {
        let status_line = "HTTP/1.1 200 OK\r\n";
        build_response(status_line, full_path.to_str().unwrap())
    } else if full_path.strip_prefix(&path_prefix).is_ok() && full_path.is_dir() {
        let mut entries = vec![];
        for entry in full_path.read_dir().unwrap() {
            let entry = entry.unwrap();
            let entry_path = entry.path();
            let entry_path = entry_path.strip_prefix(&path_prefix).unwrap();
            entries.push(entry_path.display().to_string());
        }
        let entries = entries
            .iter()
            .map(|s| format!("<a class='listItem' href='/{}'>{}</a>", s, s))
            .collect::<String>();
        let body = format!("
        <!DOCTYPE html>
        <html>
            <head>
                <meta http-equiv='content-type' content='text/html; charset=utf-8' />
                <meta name='viewport' content='width=device-width, initial-scale=1'>
                <title>Directory Listing</title>
                <style type='text/css' media='all'>
                    *{{
                        margin: 0;
                        padding: 0;
                    }}
                    body {{
                      background-color: #595260;
                      color: #ffffff;
                    }}
                    header{{
                        background: #2C2E43;
                        padding: 10px;
                        font-size: 1em;
                        font-weight: 600;
                        color: #FFD523;
                        box-shadow: 0 3px 10px rgba(0,0,0,0.2);
                        position: sticky;
                        top: 0;
                        text-align: center;
                    }}
                    .list{{
                        list-style: none;
                        height: 100%;
                        overflow-y: auto;
                        scroll-behavior: smooth;
                        display: flex;
                        flex-direction: column;
                    }}
                    .listItem{{
                        height: 40px;
                        background: #595260;
                        border: none;
                        border-bottom: 1px solid #B2B1B9;
                        padding: 10px;
                        display: flex;
                        align-items: center;
                        transition: all 0.3s ease-in-out;
                        cursor: pointer;
                        text-decoration: none;
                        color: #ffffff;
                    }}
                    .listItem:hover{{
                        background: #FFD523;
                    }}
                </style>
            </head>
            <body>
                <header>
                    <h1>Directory Listing</h1>
                </header>
                <div class='list'>
                {}
                </div>
            </body>
        </html>
        ", entries);
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
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        println!("Error: No port argument provided");
        return;
    }
    
    let port = &args[1];
    if port.len() < 4 {
        println!("Error: Port must be at least 4 characters long");
        return;
    }
    match port.parse::<u16>() {
        Ok(port_num) => port_num,
        Err(_) => {
            println!("Error: Port argument is not a valid integer");
            return;
        }
    };
    let host = "127.0.0.1";
    let url = format!("{}:{}",host,port);
    let listener = TcpListener::bind(&url).unwrap();
    println!("Server listening at http://{} \n\tPress Ctrl + C to exit",&url);
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