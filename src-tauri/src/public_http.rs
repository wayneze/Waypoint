//! 公开资料的连接恢复。只重建尚未发送 HTTP 请求的中断连接，不重试 HTTP 拒绝或证书错误。
use reqwest::{RequestBuilder, Response};

fn interrupted_connection(error: &reqwest::Error) -> bool {
    if !error.is_connect() || error.is_timeout() { return false; }
    let mut cause: Option<&(dyn std::error::Error + 'static)> = Some(error);
    while let Some(value) = cause {
        if let Some(io) = value.downcast_ref::<std::io::Error>() {
            if matches!(io.kind(), std::io::ErrorKind::UnexpectedEof | std::io::ErrorKind::ConnectionReset
                | std::io::ErrorKind::ConnectionAborted | std::io::ErrorKind::BrokenPipe) { return true; }
        }
        if value.to_string() == "tls handshake eof" { return true; }
        cause = value.source();
    }
    false
}

pub(crate) async fn send(request: RequestBuilder) -> Result<Response, reqwest::Error> {
    let mut current = request;
    for attempt in 0..3 {
        let next = current.try_clone();
        match current.send().await {
            Err(error) if attempt < 2 && interrupted_connection(&error) && next.is_some() => {
                tokio::time::sleep(std::time::Duration::from_millis(250 * (attempt + 1))).await;
                current = next.unwrap();
            }
            result => return result,
        }
    }
    unreachable!()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};

    #[tokio::test]
    async fn interrupted_tls_is_bounded_and_http_rejections_are_not_retried() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let closed = std::thread::spawn(move || {
            for _ in 0..3 {
                let (mut socket, _) = listener.accept().unwrap();
                let mut hello = [0; 4096];
                socket.read(&mut hello).unwrap();
                socket.shutdown(std::net::Shutdown::Both).unwrap();
            }
        });
        let client = reqwest::Client::builder().no_proxy().timeout(std::time::Duration::from_secs(3)).build().unwrap();
        let error = send(client.get(format!("https://{address}/"))).await.unwrap_err();
        assert!(interrupted_connection(&error));
        closed.join().unwrap();
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let rejected = std::thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            let mut request = [0; 4096];
            socket.read(&mut request).unwrap();
            socket.write_all(b"HTTP/1.1 403 Forbidden\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").unwrap();
        });
        assert_eq!(send(client.get(format!("http://{address}/"))).await.unwrap().status(), 403);
        rejected.join().unwrap();
    }
}
