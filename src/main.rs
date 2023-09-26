use rusthtmx::routes::{
    create_directory, dirs, example, get_folder_create, index, move_file, search, showing,
    toggle_move, update_dir_state,
};
use tide::http::mime;
use tide::utils::After;
use tide::{log, Request, Response};
use tide::{Body, StatusCode};
use urlencoding::decode;

#[tokio::main]
async fn main() -> tide::Result<()> {
    log::start();
    let mut app = tide::new();
    app.with(tide::log::LogMiddleware::new());
    app.with(tide::sessions::SessionMiddleware::new(
        tide::sessions::MemoryStore::new(),
        b"47f9a496e1eedfdd72ecd3d16d0d0744",
    ));
    app.with(After(|mut res: Response| async {
        if let Some(err) = res.error() {
            let msg = format!("<h1>Error: {:?}</h1>", err);
            res.set_status(err.status());
            res.set_content_type(mime::HTML);
            res.set_body(msg);
        }
        Ok(res)
    }));
    app.at("/").get(index);
    app.at("/search").get(search);
    app.at("/files/*").get(|req: Request<()>| async move {
        let home_dir = std::env!("HOME");
        let path = req.url().path();
        let file_location = decode(path)?;
        let dir = file_location.to_string();
        let file_dir = dir.replace("/files", home_dir);
        if let Ok(body) = Body::from_file(file_dir).await {
            Ok(body.into())
        } else {
            Ok(Response::new(StatusCode::NotFound))
        }
    });
    app.at("/static").serve_dir("./static")?;
    app.at("/example").get(example);
    app.at("/togglemove").post(toggle_move);
    app.at("/movefile").post(move_file);
    app.at("/get_create_input").get(get_folder_create);
    app.at("/create_folder").post(create_directory);
    app.at("/show").post(showing);
    app.at("/dirs").get(dirs);
    app.at("/select_location").post(update_dir_state);
    app.listen("0.0.0.0:5000").await?;
    Ok(())
}
