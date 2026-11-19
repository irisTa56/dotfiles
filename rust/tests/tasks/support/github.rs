//! A stand-in for GitHub's REST API, served on a local port, which answers every request
//! with one fixed response and records what it was sent.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;

/// One request the stand-in received.
#[derive(Debug, Clone)]
pub(crate) struct Request {
    pub(crate) method: String,
    pub(crate) path: String,
    /// The value of the `Authorization` header, where one was sent.
    pub(crate) authorization: Option<String>,
    pub(crate) body: serde_json::Value,
}

/// The stand-in, serving until the test process ends.
pub(crate) struct GitHubApi {
    url: String,
    requests: Arc<Mutex<Vec<Request>>>,
}

impl GitHubApi {
    /// A stand-in that answers each request with `status` and the JSON `body`.
    pub(crate) fn answering(status: u16, body: &str) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("a local port can be bound");
        let url = format!(
            "http://{}",
            listener
                .local_addr()
                .expect("the bound port has an address")
        );
        let requests = Arc::new(Mutex::new(Vec::new()));
        let recorded = Arc::clone(&requests);
        let response = format!(
            "HTTP/1.1 {status} Stand-in\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(stream) = stream else { continue };
                if let Some(request) = serve(stream, &response) {
                    recorded
                        .lock()
                        .expect("no recording thread panicked")
                        .push(request);
                }
            }
        });
        GitHubApi { url, requests }
    }

    /// The base URL a run takes as `GITHUB_API_URL`.
    pub(crate) fn url(&self) -> &str {
        &self.url
    }

    /// The requests received so far, in order.
    pub(crate) fn requests(&self) -> Vec<Request> {
        self.requests
            .lock()
            .expect("no recording thread panicked")
            .clone()
    }
}

/// Reads one request from `stream`, answers it with `response`, and returns it.
fn serve(mut stream: TcpStream, response: &str) -> Option<Request> {
    let mut reader = BufReader::new(stream.try_clone().ok()?);
    let mut line = String::new();
    reader.read_line(&mut line).ok()?;
    let mut words = line.split_whitespace();
    let method = words.next()?.to_owned();
    let path = words.next()?.to_owned();
    let mut length = 0;
    let mut authorization = None;
    loop {
        let mut header = String::new();
        reader.read_line(&mut header).ok()?;
        let header = header.trim_end();
        if header.is_empty() {
            break;
        }
        let (name, value) = header.split_once(':')?;
        let value = value.trim();
        if name.eq_ignore_ascii_case("content-length") {
            length = value.parse().ok()?;
        } else if name.eq_ignore_ascii_case("authorization") {
            authorization = Some(value.to_owned());
        }
    }
    let mut body = vec![0; length];
    reader.read_exact(&mut body).ok()?;
    stream.write_all(response.as_bytes()).ok()?;
    Some(Request {
        method,
        path,
        authorization,
        body: serde_json::from_slice(&body).unwrap_or(serde_json::Value::Null),
    })
}
