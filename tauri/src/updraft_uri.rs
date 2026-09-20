use crate::airspace_resource::airspace_resource_response;
use crate::basemap::basemap_resource_response;
use crate::terrain::terrain_resource_response;
use std::future::Future;
use tauri::http::{HeaderValue, Request, Response, StatusCode, header};
use tauri::{UriSchemeContext, UriSchemeResponder};

/// Routes resources that use the `updraft` URI scheme.
pub fn handle_updraft_uri<R: tauri::Runtime>(
    context: UriSchemeContext<'_, R>,
    request: Request<Vec<u8>>,
    responder: UriSchemeResponder,
) {
    let app = context.app_handle().clone();
    let respond = move |mut response: Response<Vec<u8>>| {
        response.headers_mut().insert(
            header::ACCESS_CONTROL_ALLOW_ORIGIN,
            HeaderValue::from_static("*"),
        );
        responder.respond(response);
    };

    match request.uri().path() {
        path if path.starts_with("/arrivals/") && path.ends_with(".geojson") => {
            let id = path["/arrivals/".len()..path.len() - ".geojson".len()].to_owned();
            spawn_response(
                crate::waypoints::arrival_stream::arrival_resource_response(app, id),
                respond,
            );
        }
        "/waypoints.geojson" => spawn_response(
            crate::waypoints::resource::waypoint_resource_response(app),
            respond,
        ),
        "/airspace.geojson" => spawn_response(airspace_resource_response(app), respond),
        path if path.starts_with("/basemap/") => {
            let path = path["/basemap/".len()..].to_owned();
            spawn_response(basemap_resource_response(app, path), respond);
        }
        path if path.starts_with("/terrain/") => {
            let path = path["/terrain/".len()..].to_owned();
            spawn_response(terrain_resource_response(app, path), respond);
        }
        _ => respond(not_found_response()),
    }
}

fn spawn_response(
    future: impl Future<Output = Response<Vec<u8>>> + Send + 'static,
    respond: impl FnOnce(Response<Vec<u8>>) + Send + 'static,
) {
    tauri::async_runtime::spawn(async move {
        respond(future.await);
    });
}

fn not_found_response() -> Response<Vec<u8>> {
    Response::builder()
        .status(StatusCode::NOT_FOUND)
        .body(Vec::new())
        .expect("the fixed not-found response should be valid")
}

pub fn tile_coordinates(path: &str) -> Option<[u32; 3]> {
    let mut parts = path.split('/');
    let z = parts.next()?.parse().ok()?;
    let x = parts.next()?.parse().ok()?;
    let y = parts.next()?.parse().ok()?;
    let size = 1_u32.checked_shl(z)?;
    (parts.next().is_none() && x < size && y < size).then_some([z, x, y])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tile_coordinate_boundaries() {
        for (path, expected) in [
            ("0/0/0", [0, 0, 0]),
            ("31/2147483647/2147483647", [31, 2147483647, 2147483647]),
            ("+1/01/+1", [1, 1, 1]),
        ] {
            claims::assert_some_eq!(tile_coordinates(path), expected);
        }
        for path in [
            "",
            "0/0",
            "0/0/0/0",
            "/0/0/0",
            "0/0/0/",
            "32/0/0",
            "0/1/0",
            "0/0/1",
            "31/2147483648/0",
            "31/0/2147483648",
            "4294967296/0/0",
            "1/4294967296/0",
            "1/0/4294967296",
            "-1/0/0",
            "1/-1/0",
            "1/0/-1",
            "a/0/0",
            "0/ 0/0",
        ] {
            claims::assert_none!(tile_coordinates(path), "{path}");
        }
    }
}
