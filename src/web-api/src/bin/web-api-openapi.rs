use utoipa::OpenApi;
use web_api::ApiDoc;

fn main() {
    let doc = ApiDoc::openapi().to_pretty_json().unwrap();
    print!("{doc}");
}
