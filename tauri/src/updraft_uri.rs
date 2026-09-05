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
