#![cfg(feature = "http")]

use fc_ui::http::SimpleHttpClient;
use futures::{future::Either, AsyncRead};
use gpui::http_client::{AsyncBody, HttpClient, HttpRequestExt, Request};
use std::{
    io::{Read, Write},
    net::TcpListener,
    pin::Pin,
    task::{Context, Poll},
    time::{Duration, Instant},
};

// Watchdogs only bound a failing run; generous values keep loaded CI hosts
// from turning scheduling delays into false failures.
const WATCHDOG: Duration = Duration::from_secs(5);

struct PendingBody;

impl AsyncRead for PendingBody {
    fn poll_read(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        _buffer: &mut [u8],
    ) -> Poll<std::io::Result<usize>> {
        Poll::Pending
    }
}

#[test]
fn timeout_includes_reading_the_request_body() {
    let client = SimpleHttpClient::new("fc-ui-timeout-test").unwrap();
    let request = Request::post("http://127.0.0.1:9/")
        .timeout(Duration::from_millis(30))
        .body(AsyncBody::from_reader(PendingBody))
        .unwrap();

    smol::block_on(async {
        match futures::future::select(client.send(request), Box::pin(smol::Timer::after(WATCHDOG)))
            .await
        {
            Either::Left((result, _)) => assert_eq!(
                result
                    .err()
                    .expect("the complete request must time out")
                    .downcast_ref::<std::io::Error>()
                    .unwrap()
                    .kind(),
                std::io::ErrorKind::TimedOut,
            ),
            Either::Right(_) => panic!("the request body outlived the complete-request timeout"),
        }
    });
}

#[test]
fn timeout_includes_reading_the_response_body() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    listener.set_nonblocking(true).unwrap();
    let (release, hold_response) = std::sync::mpsc::channel();
    let server = std::thread::spawn(move || -> Result<(), String> {
        let accept_deadline = Instant::now() + WATCHDOG;
        let mut stream = loop {
            match listener.accept() {
                Ok((stream, _)) => break stream,
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    if Instant::now() >= accept_deadline || hold_response.try_recv().is_ok() {
                        return Err("no connection was accepted".into());
                    }
                    std::thread::sleep(Duration::from_millis(5));
                }
                Err(error) => return Err(format!("accept failed: {error}")),
            }
        };
        stream
            .set_nonblocking(false)
            .and_then(|()| stream.set_read_timeout(Some(WATCHDOG)))
            .map_err(|error| format!("configuring the stream failed: {error}"))?;
        let mut request = Vec::new();
        let mut byte = [0];
        while !request.ends_with(b"\r\n\r\n") {
            stream
                .read_exact(&mut byte)
                .map_err(|error| format!("reading the request head failed: {error}"))?;
            request.push(byte[0]);
        }
        stream
            .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nx")
            .map_err(|error| format!("writing the partial response failed: {error}"))?;
        let _ = hold_response.recv_timeout(WATCHDOG);
        Ok(())
    });
    let client = SimpleHttpClient::new("fc-ui-timeout-test").unwrap();
    // Long enough for a loaded host to deliver the request head before the
    // deadline, so the timeout is observed while reading the response body.
    let request = Request::get(format!("http://{address}/"))
        .timeout(Duration::from_millis(500))
        .body(AsyncBody::empty())
        .unwrap();
    let result = smol::block_on(async {
        match futures::future::select(client.send(request), Box::pin(smol::Timer::after(WATCHDOG)))
            .await
        {
            Either::Left((result, _)) => Some(result),
            Either::Right(_) => None,
        }
    });
    let _ = release.send(());
    if let Err(error) = server.join().unwrap() {
        panic!("the server must answer with a partial response: {error}");
    }
    assert!(
        result
            .expect("response body exceeded the timeout watchdog")
            .is_err(),
        "an incomplete response must time out"
    );
}
