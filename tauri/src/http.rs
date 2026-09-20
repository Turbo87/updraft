pub fn client() -> reqwest::ClientBuilder {
    let roots = rustls::RootCertStore {
        roots: webpki_roots::TLS_SERVER_ROOTS.to_vec(),
    };
    let tls = rustls::ClientConfig::builder()
        .with_root_certificates(roots)
        .with_no_client_auth();
    reqwest::Client::builder().use_preconfigured_tls(tls)
}

pub async fn read_bounded_body(
    mut response: reqwest::Response,
    limit: usize,
    limit_message: &'static str,
) -> anyhow::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await? {
        anyhow::ensure!(bytes.len() + chunk.len() <= limit, limit_message);
        bytes.extend_from_slice(&chunk);
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use claims::{assert_err, assert_ok};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    async fn chunked_response(body: &'static [u8]) -> reqwest::Response {
        let listener = assert_ok!(tokio::net::TcpListener::bind("127.0.0.1:0").await);
        let address = assert_ok!(listener.local_addr());
        let server = tokio::spawn(async move {
            let (mut stream, _) = assert_ok!(listener.accept().await);
            let mut request = Vec::new();
            while !request.ends_with(b"\r\n\r\n") {
                request.push(assert_ok!(stream.read_u8().await));
            }
            let headers =
                b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n";
            assert_ok!(stream.write_all(headers).await);
            assert_ok!(stream.write_all(body).await);
        });
        let url = format!("http://{address}/");
        let response = assert_ok!(assert_ok!(client().build()).get(url).send().await);
        assert_ok!(server.await);
        response
    }

    #[tokio::test]
    async fn bounds_the_accumulated_body_across_chunks() {
        for limit in [3, 4, 5] {
            let response = chunked_response(b"2\r\nab\r\n2\r\ncd\r\n0\r\n\r\n").await;
            let result = read_bounded_body(response, limit, "Too many bytes").await;
            if limit < 4 {
                assert_eq!(assert_err!(result).to_string(), "Too many bytes");
            } else {
                assert_eq!(assert_ok!(result), b"abcd");
            }
        }
    }

    #[tokio::test]
    async fn rejects_incomplete_bodies_instead_of_returning_partial_bytes() {
        let response = chunked_response(b"2\r\nab\r\n2\r\nc").await;
        let error = assert_err!(read_bounded_body(response, 4, "Too many bytes").await);
        claims::assert_some!(error.downcast_ref::<reqwest::Error>());
    }
}
