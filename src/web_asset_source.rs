use bevy::{
    asset::io::PathStream,
    prelude::*,
    utils::{ConditionalSendFuture, HashMap},
};
use std::{
    path::{Path, PathBuf},
    sync::{Arc, RwLock},
};

use bevy::asset::io::{AssetReader, AssetReaderError, Reader};

/// Which protocol to use
#[derive(Debug)]
pub enum Protocol {
    /// Use TLS for setting up connections.
    Https,
    /// Unencrypted connections.
    Http,
}

/// Resource which stores headers, query and other settings for [`WebAssetReader`].
#[derive(Resource, Debug, Default)]
pub struct WebAssetReaderData {
    pub(crate) data: Arc<RwLock<WebAssetReaderDataInner>>,
}

impl WebAssetReaderData {
    /// When true, this feature turns "test/example..png" into "test/example" while sending the request, but leaves individual dots alone.
    pub fn set_fake_extensions(&self, state: bool) {
        let mut w = self.data.write().unwrap();
        w.fake_extensions = state;
    }

    /// Push a new header to be sent along every asset load. The same key can be pushed multiple times.
    pub fn push_header(&self, key: impl ToString, value: impl ToString) {
        let mut w = self.data.write().unwrap();
        w.headers
            .entry(key.to_string())
            .or_insert_with(Vec::new)
            .push(value.to_string());
    }

    /// Remove all headers
    pub fn clear_headers(&self) {
        let mut w = self.data.write().unwrap();
        w.headers.clear();
    }

    /// Push a query parameter, which will be appended to the reqeust before its sent
    pub fn push_query(&self, key: impl ToString, value: impl ToString) {
        let mut w = self.data.write().unwrap();
        w.query.insert(key.to_string(), value.to_string());
    }

    /// Remove all query params
    pub fn clear_query(&self) {
        let mut w = self.data.write().unwrap();
        w.query.clear();
    }
}

/// Struct which stores headers, query and other settings for [`WebAssetReader`].
///
/// The asset reader stores this using a `Weak<RwLock<>>` so it may access changes done to the [`WebAssetReaderData`] Resource owning this.
#[derive(Debug, Default, Clone)]
pub struct WebAssetReaderDataInner {
    /// Headers will be passed along with each request
    pub headers: HashMap<String, Vec<String>>,
    /// Query parameters will be passed along with each request
    pub query: HashMap<String, String>,
    /// Fake extensions are those with 2 dots. They will be removed before sending the request.
    pub fake_extensions: bool,
}

/// Treats paths as urls to load assets from.
pub struct WebAssetReader {
    /// The protocol whith which the request is sent.
    /// This is usually set by the plugin during [`bevy::app::App::register_asset_source`]
    /// to correspond with the the [`bevy::asset::io::AssetSourceId`] set for this
    pub protocol: Protocol,
    /// Shared
    pub shared: Arc<RwLock<WebAssetReaderDataInner>>,
}

fn strip_double_extension(path: &mut PathBuf) -> Option<()> {
    let fname = path.file_name()?.to_str()?;
    let ext_start = fname.len() - path.extension()?.len();

    if &fname[ext_start - 2..ext_start] == ".." {
        path.set_extension("");
        path.set_extension("");
        Some(())
    } else {
        Some(())
    }
}

impl WebAssetReader {
    fn make_uri(&self, path: &Path) -> PathBuf {
        let mut buf = PathBuf::from(match self.protocol {
            Protocol::Http => "http://",
            Protocol::Https => "https://",
        })
        .join(path);
        if self.shared.read().unwrap().fake_extensions {
            strip_double_extension(&mut buf);
        }
        buf
    }

    fn make_uri_query(&self, path: &Path) -> PathBuf {
        let mut buf = self.make_uri(path);
        let shared_guard = self.shared.read().unwrap();
        let mut query = shared_guard.query.iter();
        let mut query_string = String::new();
        if let Some((query_k, val)) = query.next() {
            query_string += &format!("?{query_k}={val}");
        }

        for (query_k, val) in query {
            query_string += &format!("&{query_k}={val}");
        }
        if query_string.len() > 0 {
            buf.push(query_string);
        }
        buf
    }

    /// See [bevy::asset::io::get_meta_path]
    fn make_meta_uri(&self, path: &Path) -> Option<PathBuf> {
        path.extension()?;
        let mut uri = self.make_uri(path);
        let mut fname = uri.file_name()?.to_os_string();
        fname.push(".meta");
        uri.set_file_name(fname);
        Some(uri)
    }
}

#[cfg(target_arch = "wasm32")]
async fn get<'a>(
    path: PathBuf,
    shared: Arc<RwLock<WebAssetReaderDataInner>>,
) -> Result<Box<Reader<'a>>, AssetReaderError> {
    use bevy::asset::io::VecReader;
    use js_sys::Uint8Array;
    use wasm_bindgen::JsCast;
    use wasm_bindgen_futures::JsFuture;
    use web_sys::{Request, RequestInit, RequestMode, Response};

    fn js_value_to_err<'a>(
        context: &'a str,
    ) -> impl FnOnce(wasm_bindgen::JsValue) -> std::io::Error + 'a {
        move |value| {
            let message = match js_sys::JSON::stringify(&value) {
                Ok(js_str) => format!("Failed to {context}: {js_str}"),
                Err(_) => {
                    format!(
                        "Failed to {context} and also failed to stringify the JSValue of the error"
                    )
                }
            };

            std::io::Error::new(std::io::ErrorKind::Other, message)
        }
    }

    let window = web_sys::window().unwrap();
    let mut init = RequestInit::new();
    init.set_mode(RequestMode::Cors);
    let request = Request::new_with_str_and_init(path.to_str().unwrap(), &init).unwrap();
    let request_headers = request.headers();
    for (header_name, header_values) in shared
        .read()
        .unwrap()
        .headers
        .iter()
        .map(|(k, v)| (k.as_str(), v.as_slice()))
    {
        for header_value in header_values {
            request_headers
                .append(header_name, header_value.as_str())
                .map_err(js_value_to_err("append header"))?;
        }
    }
    let resp_value = JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(js_value_to_err("fetch path"))?;
    let resp = resp_value
        .dyn_into::<Response>()
        .map_err(js_value_to_err("convert fetch to Response"))?;
    match resp.status() {
        200 => {
            let data = JsFuture::from(resp.array_buffer().unwrap()).await.unwrap();
            let bytes = Uint8Array::new(&data).to_vec();
            let reader: Box<Reader> = Box::new(VecReader::new(bytes));
            Ok(reader)
        }
        404 => Err(AssetReaderError::NotFound(path)),
        status => Err(AssetReaderError::Io(
            std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Encountered unexpected HTTP status {status}"),
            )
            .into(),
        )),
    }
}

#[cfg(not(target_arch = "wasm32"))]
async fn get<'a>(
    path: PathBuf,
    shared: Arc<RwLock<WebAssetReaderDataInner>>,
) -> Result<Box<Reader<'a>>, AssetReaderError> {
    use std::io;

    use bevy::asset::io::VecReader;
    use bevy::tasks::AsyncComputeTaskPool;

    let str_path = path.to_str().ok_or_else(|| {
        AssetReaderError::Io(
            io::Error::new(
                io::ErrorKind::Other,
                format!("non-utf8 path: {}", path.display()),
            )
            .into(),
        )
    })?;

    let mut request = ureq::get(str_path);

    // From headers iter to surf headers
    for (header_name, header_values) in shared
        .read()
        .unwrap()
        .headers
        .iter()
        .map(|(k, v)| (k.as_str(), v.as_slice()))
    {
        for val in header_values {
            request = request.set(header_name, val);
        }
    }

    let pool = AsyncComputeTaskPool::get();
    let blocking_task: bevy_tasks::Task<Result<_, AssetReaderError>> = pool.spawn(async move {
        let response = request.call().map_err(|err| {
            if let ureq::Error::Status(404, _) = err {
                AssetReaderError::NotFound(path.clone())
            } else {
                AssetReaderError::Io(
                    io::Error::new(
                        io::ErrorKind::Other,
                        format!("unexpected error while loading {}: {}", path.display(), err),
                    )
                    .into(),
                )
            }
        })?;

        let mut buf = vec![];
        let mut reader = response.into_reader();
        reader.read_to_end(&mut buf).map_err(|err| {
            AssetReaderError::Io(
                io::Error::new(
                    io::ErrorKind::Other,
                    format!("unexpected error while loading {}: {}", path.display(), err,),
                )
                .into(),
            )
        })?;
        Ok(buf)
    });

    let result = blocking_task.await?;
    Ok(Box::new(VecReader::new(result)))
}

impl AssetReader for WebAssetReader {
    fn read<'a>(
        &'a self,
        path: &'a Path,
    ) -> impl ConditionalSendFuture<Output = Result<Box<Reader<'a>>, AssetReaderError>> {
        get(self.make_uri_query(path), self.shared.clone())
    }

    async fn read_meta<'a>(&'a self, path: &'a Path) -> Result<Box<Reader<'a>>, AssetReaderError> {
        match self.make_meta_uri(path) {
            Some(uri) => get(uri, self.shared.clone()).await,
            None => Err(AssetReaderError::NotFound(
                "source path has no extension".into(),
            )),
        }
    }

    async fn is_directory<'a>(&'a self, _path: &'a Path) -> Result<bool, AssetReaderError> {
        Ok(false)
    }

    async fn read_directory<'a>(
        &'a self,
        path: &'a Path,
    ) -> Result<Box<PathStream>, AssetReaderError> {
        Err(AssetReaderError::NotFound(self.make_uri(path)))
    }
}

#[cfg(test)]
mod tests {
    use bevy::utils::default;

    use super::*;

    fn new_reader(protocol: Protocol, fake_extensions: bool) -> WebAssetReader {
        WebAssetReader {
            protocol,
            shared: Arc::new(RwLock::new(WebAssetReaderDataInner {
                fake_extensions,
                ..default()
            })),
        }
    }

    #[test]
    fn make_http_uri_no_fake() {
        assert_eq!(
            new_reader(Protocol::Http, false)
                .make_uri(Path::new("s3.johanhelsing.studio/dump/favicon..png"))
                .to_str()
                .unwrap(),
            "http://s3.johanhelsing.studio/dump/favicon..png"
        );
    }

    #[test]
    fn make_http_uri() {
        assert_eq!(
            new_reader(Protocol::Http, true)
                .make_uri(Path::new("s3.johanhelsing.studio/dump/favicon.png"))
                .to_str()
                .unwrap(),
            "http://s3.johanhelsing.studio/dump/favicon.png"
        );
    }

    #[test]
    fn make_http_uri_strip_fake() {
        assert_eq!(
            new_reader(Protocol::Http, true)
                .make_uri(Path::new("s3.johanhelsing.studio/dump/favicon..png"))
                .to_str()
                .unwrap(),
            "http://s3.johanhelsing.studio/dump/favicon"
        );
    }

    #[test]
    fn make_https_uri() {
        assert_eq!(
            new_reader(Protocol::Https, true)
                .make_uri(Path::new("s3.johanhelsing.studio/dump/favicon.png"))
                .to_str()
                .unwrap(),
            "https://s3.johanhelsing.studio/dump/favicon.png"
        );
    }

    #[test]
    fn make_http_meta_uri() {
        assert_eq!(
            new_reader(Protocol::Http, true)
                .make_meta_uri(Path::new("s3.johanhelsing.studio/dump/favicon.png"))
                .expect("cannot create meta uri")
                .to_str()
                .unwrap(),
            "http://s3.johanhelsing.studio/dump/favicon.png.meta"
        );
    }

    #[test]
    fn make_http_meta_uri_strip_fake() {
        assert_eq!(
            new_reader(Protocol::Http, true)
                .make_meta_uri(Path::new("s3.johanhelsing.studio/dump/favicon..png"))
                .expect("cannot create meta uri")
                .to_str()
                .unwrap(),
            "http://s3.johanhelsing.studio/dump/favicon.meta"
        );
    }

    #[test]
    fn make_https_meta_uri() {
        assert_eq!(
            new_reader(Protocol::Https, true)
                .make_meta_uri(Path::new("s3.johanhelsing.studio/dump/favicon.png"))
                .expect("cannot create meta uri")
                .to_str()
                .unwrap(),
            "https://s3.johanhelsing.studio/dump/favicon.png.meta"
        );
    }

    #[test]
    fn make_https_without_extension_meta_uri() {
        assert_eq!(
            new_reader(Protocol::Https, true)
                .make_meta_uri(Path::new("s3.johanhelsing.studio/dump/favicon")),
            None
        );
    }
}
