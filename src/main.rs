use std::{fs, io, thread};
use std::fs::File;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use thread_helper::ThreadPool;
use lazy_static::lazy_static;

lazy_static!{
    static ref ERR_PAGE: Option<String> = {
        let page = fs::read_to_string("website/404.html").ok()?;
        Some(format!("HTTP/1.1 404 NOT FOUND\r\nContent-Len: {}\r\n\r\n{page}", page.len()))
    };

    static ref SERVER_ERR_PAGE: Option<String> = {
        let page = fs::read_to_string("website/500.html").ok()?;
        Some(format!("HTTP/1.1 500 Internal Server Error\r\nContent-Len: {}\r\n\r\n{page}", page.len()))
    };
}

enum ConnectionError {
    TCPReadFailed,
    SourceNotFound,
    InternalServerErr,
}

impl ConnectionError {
    fn get_html_err_msg(&self) -> &[u8] {
        match self {
            ConnectionError::TCPReadFailed => "HTTP/1.1 400 BAD REQUEST".as_bytes(),
            ConnectionError::SourceNotFound => ERR_PAGE.as_ref().map_or_else(|| "HTTP/1.1 404 NOT FOUND".as_bytes(), |s| s.as_bytes()),
            ConnectionError::InternalServerErr => SERVER_ERR_PAGE.as_ref().map_or_else(|| "HTTP/1.1 500 Internal Server Error".as_bytes(), |s| s.as_bytes()),
        }
    }
}

fn main() {
    println!("Starting web server...");

    match fs::read_dir("website") {
        Ok(_) => {}
        Err(_) => {
            println!("Error! Unable to find the website! (It should be in a folder \"/website\" in the same folder this file is)");
            finish_wait();
        }
    }

    let listener = TcpListener::bind("127.0.0.1:8080").map_err(|_| {
        println!("Error! Unable to bind to port 127.0.0.1:8080!");
        finish_wait();
    }).unwrap();
    let pool = ThreadPool::new(9);

    let input_thread = thread::spawn(move || {
        let mut input = String::new();
        loop {
            io::stdin().read_line(&mut input).map_or_else(|_| 0, |s| s);
            if input.trim() == "stop" {
                println!("Stopping the web server...");
                break;
            }
            input.clear();
        }
        finish_wait();
    });

    println!("Successfully started! Listening on: 127.0.0.1:8080...");

    for stream in listener.incoming() {
        let mut stream = match stream {
            Ok(r) => r,
            Err(_) => continue,
        };

        pool.execute(move || {
            let result = handle_connection(&mut stream);
            match result {
                Ok(response) => stream.write(response.as_bytes()).unwrap_or(0),
                Err(e) => stream.write(e.get_html_err_msg()).unwrap_or(0),
            };
            stream.flush().unwrap_or(());
        });
    }

    input_thread.join().expect("Input thread panicked");

    finish_wait();
}

fn handle_connection(mut stream: &mut TcpStream) -> Result<String, ConnectionError> {
    let buf_reader = BufReader::new(&mut stream);
    let mut http_request = buf_reader
        .lines()
        .map(|result| result.map_err(|_| ConnectionError::TCPReadFailed))
        .map(|result| result.unwrap_or("".to_string()))
        .take_while(|line| !line.is_empty());

    let mut path: String = {
        http_request.next()
            .ok_or(ConnectionError::TCPReadFailed)?
            .split(" ")
            .nth(1)
            .map(|s| s.to_string())
    }.ok_or(ConnectionError::TCPReadFailed)?;

    if path.contains(".css") {
        let css_contents = fs::read_to_string(format!("website{path}")).ok().ok_or(ConnectionError::SourceNotFound)?;
        return Ok(format!("HTTP/1.1 200 OK\r\nContent-Type: text/css\r\nContent-Length: {}\r\n\r\n{css_contents}", css_contents.len()))
    }
    if path.contains(".png") {
        let mut img_contents = vec![];
        File::open(format!("website{}", path)).ok().ok_or(ConnectionError::SourceNotFound)?.read_to_end(&mut img_contents).ok().ok_or(ConnectionError::SourceNotFound)?;

        stream.write_all(format!("HTTP/1.1 200 OK\r\nContent-Type: image/png\r\nContent-Length: {}\r\n\r\n", img_contents.len()).as_bytes()).unwrap_or(());
        stream.write(&img_contents).unwrap_or(0);
        return Ok("".to_string());
    }
    if path.contains(".ico") {
        let mut img_contents = vec![];
        File::open(format!("website{}", path)).ok().ok_or(ConnectionError::SourceNotFound)?.read_to_end(&mut img_contents).ok().ok_or(ConnectionError::SourceNotFound)?;

        stream.write_all(format!("HTTP/1.1 200 OK\r\nContent-Type: image/vnd.microsoft.icon\r\nContent-Length: {}\r\n\r\n", img_contents.len()).as_bytes()).unwrap_or(());
        stream.write(&img_contents).unwrap_or(0);
        return Ok("".to_string());
    }

    if path == "/" {
        path = "/home".to_string();
    }

    let status_line = "HTTP/1.1 200 OK";
    let html_contents = fs::read_to_string(format!("website{path}.html")).ok().ok_or(ConnectionError::SourceNotFound)?;

    Ok(format!("{status_line}\r\nContent-Type: text/html\r\nContent-Length: {}\r\n\r\n{html_contents}", html_contents.len()))
}

fn finish_wait() {
    println!("Press enter to continue...");
    let mut temp = String::new();
    io::stdin().read_line(&mut temp).unwrap();
    std::process::exit(0);
}