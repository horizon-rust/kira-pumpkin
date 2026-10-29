//! One HTTPS POST through `wasi:http` (Pumpkin grants it with the `HTTP_OUTBOUND` permission),
//! with timeouts on the connection, the first byte and every byte after.
use std::time::Duration;
use wasi::http::outgoing_handler;
use wasi::http::types::{Fields, Method, OutgoingBody, OutgoingRequest, RequestOptions, Scheme};
use wasi::io::streams::StreamError;

/// Kira's answers are small; anything bigger is not hers.
const MAX_BODY: usize = 16 * 1024;

/// POSTs `body` (JSON) to `url` (`https://host/path`) with `token` as a bearer token: the status and
/// the response body, or what went wrong.
pub fn post_json(
    url: &str,
    token: &str,
    body: &str,
    timeout: Duration,
) -> Result<(u16, Vec<u8>), String> {
    let rest = url
        .strip_prefix("https://")
        .ok_or("not an https:// address")?;
    let (authority, path) = rest.split_at(rest.find('/').unwrap_or(rest.len()));
    let path = if path.is_empty() { "/" } else { path };
    let fail = |what: &str| format!("could not {what}");

    let headers = Fields::from_list(&[
        ("content-type".into(), b"application/json".to_vec()),
        (
            "authorization".into(),
            format!("Bearer {token}").into_bytes(),
        ),
        (
            "user-agent".into(),
            concat!("kira-pumpkin/", env!("CARGO_PKG_VERSION")).into(),
        ),
    ])
    .map_err(|e| format!("headers: {e:?}"))?;
    let request = OutgoingRequest::new(headers);
    request
        .set_method(&Method::Post)
        .map_err(|()| fail("set the method"))?;
    request
        .set_scheme(Some(&Scheme::Https))
        .map_err(|()| fail("set the scheme"))?;
    request
        .set_authority(Some(authority))
        .map_err(|()| fail("set the host"))?;
    request
        .set_path_with_query(Some(path))
        .map_err(|()| fail("set the path"))?;

    let outgoing = request.body().map_err(|()| fail("open the body"))?;
    {
        let stream = outgoing.write().map_err(|()| fail("write the body"))?;
        for chunk in body.as_bytes().chunks(4096) {
            stream
                .blocking_write_and_flush(chunk)
                .map_err(|e| format!("sending: {e:?}"))?;
        }
    }
    OutgoingBody::finish(outgoing, None).map_err(|e| format!("sending: {e:?}"))?;

    let nanos = u64::try_from(timeout.as_nanos()).unwrap_or(u64::MAX);
    let options = RequestOptions::new();
    let _ = options.set_connect_timeout(Some(nanos));
    let _ = options.set_first_byte_timeout(Some(nanos));
    let _ = options.set_between_bytes_timeout(Some(nanos));
    let pending =
        outgoing_handler::handle(request, Some(options)).map_err(|e| format!("request: {e:?}"))?;
    pending.subscribe().block();
    let response = pending
        .get()
        .ok_or("no response")?
        .map_err(|()| fail("read the response"))?
        .map_err(|e| format!("request: {e:?}"))?;

    let status = response.status();
    let incoming = response
        .consume()
        .map_err(|()| fail("read the response body"))?;
    let stream = incoming
        .stream()
        .map_err(|()| fail("read the response body"))?;
    let mut out = Vec::new();
    loop {
        match stream.blocking_read(4096) {
            Ok(chunk) => out.extend_from_slice(&chunk),
            Err(StreamError::Closed) => break,
            Err(e) => return Err(format!("reading: {e:?}")),
        }
        if out.len() > MAX_BODY {
            return Err("response too large".into());
        }
    }
    Ok((status, out))
}
