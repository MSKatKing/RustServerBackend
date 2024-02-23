use std::{fs, io, thread};
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use thread_helper::ThreadPool;
use lazy_static::lazy_static;

lazy_static!{
    static ref ERR_PAGE: Option<String> = {
        let page = fs::read_to_string("website/404.html").ok()?;
        Some(format!("HTTP/1.1 404 NOT FOUND\r\nContent-Len: {}\r\n\r\n{page}", page.len()))
    };
}

enum ConnectionError {
    TCPReadFailed,
    HTMLNotFound
}

impl ConnectionError {
    fn get_html_err_msg(&self) -> &[u8] {
        match self {
            ConnectionError::TCPReadFailed => "HTTP/1.1 400 BAD REQUEST".as_bytes(),
            ConnectionError::HTMLNotFound => ERR_PAGE.as_ref().map_or_else(|| "HTTP/1.1 404 NOT FOUND".as_bytes(), |s| s.as_bytes()),
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
            std::process::exit(404);
        }
    }

    let listener = TcpListener::bind("127.0.0.1:8080").unwrap();
    let pool = ThreadPool::new(9);

    let input_thread = thread::spawn(move || {
        let mut input = String::new();
        loop {
            io::stdin().read_line(&mut input).expect("Failed to read line from stdin");
            if input.trim() == "stop" {
                println!("Stopping the web server...");
                break;
            }
            input.clear();
        }
        finish_wait();
        std::process::exit(0);
    });

    println!("Successfully started! Listening on: 127.0.0.1:8080...");

    for stream in listener.incoming() {
        let mut stream = match stream {
            Ok(r) => r,
            Err(_) => continue,
        };

        //println!("Connection established! ({})", stream.peer_addr().unwrap().to_string());

        pool.execute(move || {
            let result = handle_connection(&mut stream);
            match result {
                Ok(response) => stream.write_all(response.as_bytes()).unwrap_or(()),
                Err(e) => stream.write_all(e.get_html_err_msg()).unwrap_or(()),
            }
            stream.flush().unwrap();
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
        let css_contents = fs::read_to_string(format!("website{path}")).ok().ok_or(ConnectionError::HTMLNotFound)?;
        return Ok(format!("HTTP/1.1 200 OK\r\nContent-Type: text/css\r\nContent-Length: {}\r\n\r\n{css_contents}", css_contents.len()))
    }
    if path.contains(".png") {
        let img_contents = fs::read_to_string(format!("website{path}")).ok().ok_or(ConnectionError::HTMLNotFound)?;
        return Ok(format!("HTTP/1.1 200 OK\r\nContent-Type: image/png\r\nContent-Length: {}\r\n\r\n{img_contents}", img_contents.len()))
    }

    if path == "/" {
        path = "/home".to_string();
    }

    let status_line = "HTTP/1.1 200 OK";
    let html_contents = fs::read_to_string(format!("website{path}.html")).ok().ok_or(ConnectionError::HTMLNotFound)?;

    Ok(format!("{status_line}\r\nContent-Type: text/html\r\nContent-Length: {}\r\n\r\n{html_contents}", html_contents.len()))
}

fn finish_wait() {
    println!("Press enter to continue...");
    let mut temp = String::new();
    io::stdin().read_line(&mut temp).unwrap();
}