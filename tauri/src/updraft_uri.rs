use crate::airspace_resource;
use crate::basemap::basemap_resource_response;
use tauri::http::{HeaderValue, Request, Response, StatusCode, header};
use tauri::{AppHandle, UriSchemeContext, UriSchemeResponder};

const UPDRAFT_URI_AUTHORITY: &str = "localhost";

/// Routes resources that use the `updraft` URI scheme.
pub fn handle_updraft_uri<R: tauri::Runtime>(
    context: UriSchemeContext<'_, R>,
    request: Request<Vec<u8>>,
    responder: UriSchemeResponder,
) {
    let app = context.app_handle().clone();
    tauri::async_runtime::spawn(async move {
        responder.respond(updraft_uri_response(request, app).await);
    });
}

async fn updraft_uri_response<R: tauri::Runtime>(
    request: Request<Vec<u8>>,
    app: AppHandle<R>,
) -> Response<Vec<u8>> {
    let mut response = if request.uri().scheme_str() != Some("updraft")
        || request.uri().authority().map(|value| value.as_str()) != Some(UPDRAFT_URI_AUTHORITY)
    {
        not_found_response()
    } else {
        match request.uri().path() {
            "/airspace.geojson" => airspace_resource::airspace_resource_response(app).await,
            path if path.starts_with("/basemap/") => {
                let path = path["/basemap/".len()..].to_owned();
                basemap_resource_response(app, path).await
            }
            _ => not_found_response(),
        }
    };

    response.headers_mut().insert(
        header::ACCESS_CONTROL_ALLOW_ORIGIN,
        HeaderValue::from_static("*"),
    );
    response
}

fn not_found_response() -> Response<Vec<u8>> {
    Response::builder()
        .status(StatusCode::NOT_FOUND)
        .body(Vec::new())
        .expect("the fixed not-found response should be valid")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::basemap::Basemaps;
    use crate::driver::{Driver, DriverHandle};
    use std::sync::{Arc, Mutex};
    use std::time::Duration;
    use tauri::http::{Request, StatusCode, header};
    use tauri::{Manager, test::mock_app};
    use updraft_core::{AirspaceState, SettingsSnapshot};

    fn driver() -> DriverHandle {
        Driver::spawn(
            SettingsSnapshot::default(),
            AirspaceState::none_at_startup(),
            Box::new(|_, _, _| Box::new(|| {})),
            Box::new(|_| {}),
            Duration::from_millis(100),
        )
    }

    fn request(uri: &str) -> Request<Vec<u8>> {
        Request::builder()
            .uri(uri)
            .body(Vec::new())
            .expect("the resource URI should be valid")
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn routes_airspace_by_path_under_the_localhost_authority() {
        let app = mock_app();
        app.manage(driver());

        let request = request("updraft://localhost/airspace.geojson?v=0");
        let response = updraft_uri_response(request, app.handle().clone()).await;

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers()[header::ACCESS_CONTROL_ALLOW_ORIGIN], "*");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn rejects_airspace_in_the_authority() {
        let app = mock_app();

        let request = request("updraft://airspace.geojson?v=0");
        let response = updraft_uri_response(request, app.handle().clone()).await;

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn routes_missing_basemap_tiles() {
        let app = mock_app();
        app.manage(Arc::new(Mutex::new(Basemaps::default())));

        let request = request("updraft://localhost/basemap/6/33/20.pbf");
        let response = updraft_uri_response(request, app.handle().clone()).await;
        assert_eq!(response.status(), StatusCode::NO_CONTENT);
        assert_eq!(response.headers()[header::ACCESS_CONTROL_ALLOW_ORIGIN], "*");
    }
}
