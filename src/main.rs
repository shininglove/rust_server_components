use std::fs;
use std::path::{Path, PathBuf};
use tide::http::mime;
use tide::utils::After;
use tide::{log, Request, Response};
use tide::{prelude::*, Body, StatusCode};
use tide_jsx::html::HTML5Doctype;
use tide_jsx::{component, html, rsx, view};
use urlencoding::{encode,decode};

#[component]
fn Heading<'title>(title: &'title str) {
    rsx! {
        <h1 class={"text-pink-500 text-3xl font-extrabold p-2"}>
            {title}
        </h1>
    }
}

#[component]
fn SearchInput(value: String) {
    rsx! {
        <input
            type={"search"}
            name={"destination"}
            class={"p-2 border-2 border-pink-500 rounded w-1/3"}
            hx-post={"/select_location"}
            hx-trigger={"click from:#update"}
            value={value}
        />
    }
}

#[component]
fn CreateFolderInput() {
    rsx! {
        <input
            type={"search"}
            name={"folder_name"}
            class={"p-2 border-2 border-purple-500 rounded w-1/3"}
            placeholder={"Create new folder"}
            hx-post={"/create_folder"}
            hx-trigger={"click from:#create"}
            hx-swap={"none"}
        />
    }
}

#[component]
fn FileItem(value: PathBuf) {
    // bug: everything after # symbol gets removed (state can solve this)
    let filename = value
        .file_stem()
        .unwrap_or_default()
        .to_str()
        .unwrap_or_default();
    let extension = value
        .extension()
        .unwrap_or_default()
        .to_str()
        .unwrap_or_default();
    let path = value.as_path().to_string_lossy();
    let value = format!("{}.{}", filename, extension);
    let emoji = match extension.to_lowercase().as_str() {
        "mp4" | "mov" => "📀",
        "png" | "jpg" | "jpeg" => "📷",
        _ => "❓",
    };
    let path = encode(&path);
    let vals = format!(r#"{{"destination":"{}"}}"#, path);
    rsx! {
        <div
        class={"text-2xl cursor-pointer"}
        hx-post={"/show"}
        hx-vals={vals}
        hx-trigger={"click"}
        hx-target={"#images"}
        hx-swap={"innerHTML show:bottom"}
        >
            {emoji} {value}
        </div>
    }
}

#[component]
fn Image<'src>(src: &'src str) {
    rsx! {
        <img class={"pt-2 w-48 h-48"} src={src} alt={"server-image"}/>
    }
}

#[component]
fn Video<'src>(src: &'src str, size: &'src str) {
    rsx! {
    <video controls={"true"} width={size} height={size}>
        <source src={src}></source>
    </video>
    }
}

#[component]
fn DirItem(value: PathBuf, parent: bool, move_mode: bool) {
    let path = if !parent {
        value
            .file_stem()
            .unwrap_or_default()
            .to_str()
            .unwrap_or_default()
    } else {
        value.to_str().unwrap_or_default()
    };
    let vals = format!(r#"{{"destination":"{}"}}"#, path);
    let destination = match parent {
        true => "..".to_string(),
        false => path.to_string(),
    };
    if move_mode {
        rsx! {
            <div
              class={"text-2xl cursor-pointer"}
              hx-post={"/movefile"}
              hx-vals={vals}
              hx-trigger={"click"}
              hx-target={"#images"}
              hx-swap={"innerHTML"}
            >
            {"↪️ "} {destination}
            </div>
        }
    } else {
        rsx! {
            <div
              class={"text-2xl cursor-pointer"}
              hx-post={"/select_location"}
              hx-vals={vals}
              hx-trigger={"click"}
              hx-swap={"none"}
            >
            {"📂"} {destination}
            </div>
        }
    }
}

async fn dirs(req: Request<()>) -> tide::Result {
    let current: String = req.session().get("dir").unwrap_or_default();
    let move_mode: bool = req.session().get("movemode").unwrap_or(false);
    let dir_names = fs::read_dir(&current).unwrap();
    let folders: Vec<_> = dir_names
        .into_iter()
        .filter(|d| {
            let point = d.as_ref().unwrap();
            let dotfiles = point
                .path()
                .file_stem()
                .unwrap()
                .to_str()
                .unwrap()
                .starts_with('.');
            !dotfiles
        })
        .collect();
    let mut dirs: Vec<_> = folders
        .iter()
        .filter(|d| d.as_ref().unwrap().metadata().unwrap().is_dir())
        .collect();
    dirs.sort_by_key(|x| x.as_ref().unwrap().metadata().unwrap().modified().unwrap());
    dirs.reverse();
    let mut directories: Vec<_> = dirs
        .iter()
        .map(|d| DirItem {
            value: d.as_ref().unwrap().path(),
            parent: false,
            move_mode,
        })
        .collect();
    let path = Path::new(current.as_str());
    let base_path = match path.parent() {
        Some(p) => p,
        None => path,
    };
    let parent = DirItem {
        value: base_path.to_path_buf(),
        parent: true,
        move_mode: false,
    };
    directories.splice(0..0, vec![parent]);
    let mut files: Vec<_> = folders
        .iter()
        .filter(|d| {
            let item = d.as_ref().unwrap();
            let metadata = item.metadata().unwrap();
            let extension = match item.path().extension() {
                Some(e) => matches!(
                    e.to_ascii_lowercase().to_str().unwrap(),
                    "mp4" | "mov" | "png" | "jpg" | "jpeg"
                ),
                None => false,
            };
            let is_file = metadata.is_file();
            is_file && extension
        })
        .collect();
    files.sort_by_key(|f| f.as_ref().unwrap().metadata().unwrap().len());
    let file_items: Vec<_> = files
        .iter()
        .map(|d| FileItem {
            value: d.as_ref().unwrap().path(),
        })
        .collect();
    view! {
        <>
            <section class={"p-2 inline-flex flex-wrap gap-3 w-screen"}>
                {directories}
            </section>
            <section class={"p-2 inline-flex flex-wrap gap-3"}>
                {file_items}
            </section>
        </>
    }
}

async fn search(req: Request<()>) -> tide::Result {
    let home_dir = std::env!("HOME").to_string();
    let session: String = match req.session().get("dir") {
        Some(s) => s,
        None => home_dir,
    };
    view! {
        <SearchInput value={session}/>
    }
}

async fn get_folder_create(_req: Request<()>) -> tide::Result {
    view! {
        <CreateFolderInput/>
    }
}

#[derive(Deserialize)]
struct Location {
    destination: String,
}

async fn showing(mut req: Request<()>) -> tide::Result {
    let home_dir = std::env!("HOME").to_string();
    let Location { destination } = req.body_form().await?;
    let dest = decode(&destination)?;
    let session = req.session_mut();
    let base_file = dest.to_string().replace(&home_dir, "/files");
    let source = match Path::new(destination.as_str())
        .extension()
        .unwrap_or_default()
        .to_str()
        .unwrap_or_default()
    {
        "mp4" | "mov" => view! {  <Video src={base_file.as_str()} size={"400"}/> },
        _ => view! {<Image src={base_file.as_str()}/>},
    };
    session.insert("showcase", destination)?;
    source
}

async fn example(_req: Request<()>) -> tide::Result {
    let actual = html! {
        <Image src={"/files/Pictures/7b7.png"}/>
    };
    Ok(Response::builder(tide::http::StatusCode::Ok)
        .content_type(tide::http::mime::HTML)
        .header("x-api-key", "pandamonium")
        .body(actual)
        .build())
}

async fn update_dir_state(mut req: Request<()>) -> tide::Result {
    let Location { destination } = req.body_form().await?;
    let session = req.session_mut();
    let home_dir = std::env!("HOME").to_string();
    let base_dir = match session.get::<String>("dir") {
        Some(d) => d,
        None => home_dir,
    };
    let new_home = Path::new(base_dir.as_str()).join(destination);
    session.insert("dir", new_home).unwrap();
    Ok(Response::builder(tide::http::StatusCode::Ok)
        .header("HX-Trigger-After-Settle", "refetch")
        .build())
}

async fn move_file(mut req: Request<()>) -> tide::Result {
    let Location { destination } = req.body_form().await?;
    let destination = decode(&destination)?.to_string();
    let session = req.session_mut();
    let home_dir = std::env!("HOME");
    let file_path = session.get::<String>("showcase").unwrap_or("".to_string());
    if file_path.is_empty() || home_dir.is_empty() {
        return Ok(Response::builder(StatusCode::NotFound).build());
    };
    let file_path = decode(&file_path)?.to_string();
    let filename = Path::new(&file_path).file_name().unwrap().to_str().unwrap();
    let dir_path = match session.get::<String>("dir") {
        Some(d) => {
            let d = decode(&d).unwrap().to_string();
            Path::new(&d)
            .join(destination)
            .join(filename)
            .to_str()
            .unwrap()
            .to_string()},
        None => home_dir.to_string(),
    };
    fs::rename(file_path, dir_path)?;
    session.insert("showcase", "")?;
    Ok(Response::builder(tide::http::StatusCode::Ok)
        .header("HX-Trigger-After-Settle", "refetch")
        .body("")
        .build())
}

async fn toggle_move(mut req: Request<()>) -> tide::Result {
    let session = req.session_mut();
    let move_mode = session.get::<bool>("movemode").unwrap_or(false);
    session.insert("movemode", !move_mode)?;
    Ok(Response::builder(tide::http::StatusCode::Ok)
        .header("HX-Trigger-After-Settle", "refetch")
        .build())
}

#[derive(Deserialize)]
struct Directory {
    folder_name: String,
}

async fn create_directory(mut req: Request<()>) -> tide::Result {
    let Directory { folder_name } = req.body_form().await?;
    let session = req.session();
    let home_dir = std::env!("HOME").to_string();
    let base_dir = match session.get::<String>("dir") {
        Some(d) => d,
        None => home_dir,
    };
    let new_directory = Path::new(base_dir.as_str()).join(folder_name);
    if !new_directory.exists() {
        fs::create_dir_all(new_directory)?;
    }
    Ok(Response::builder(tide::http::StatusCode::Ok)
        .header("HX-Trigger-After-Settle", "refetch")
        .build())
}

async fn index(mut req: Request<()>) -> tide::Result {
    let session = req.session_mut();
    if session.get::<String>("dir").is_none() {
        let home_dir = std::env!("HOME").to_string();
        session.insert("dir", home_dir)?;
    }
    view! {
      <>
       <HTML5Doctype />
       <html>
         <head>
            <title>{"Tide JSX"}</title>
             <script src={"https://unpkg.com/htmx.org@1.9.5"}>{""}</script>
            <link rel={"stylesheet"} href={"/static/output.css"} />
            <link rel={"stylesheet"} href={"/static/video-js.css"} />
            <link rel={"icon"} href={"/static/favicon.ico"} />
        </head>
         <body>
            <nav>
                <Heading title={"FileSystem"} />
            </nav>
            <main>
              <section
                id={"search"}
                class={"px-2 pb-2"}
                hx-get={"/search"}
                hx-trigger={"load, refetch from:body"}
                hx-target={"#search_input"}
                >
                <output id={"search_input"}>{""}</output>
              </section>
              <section
                id={"create_folder"}
                class={"px-2 pb-2"}
                hx-get={"/get_create_input"}
                hx-trigger={"click from:#create once"}
                hx-target={"#create_input"}
                >
                <output id={"create_input"}>{""}</output>
              </section>
              <section id={"images"} class={"p-1"}>{""}</section>
              <section class={"px-2 gap-2 flex"} id={"controls"}>
                <button
                  hx-post={"/togglemove"}
                  hx-swap={"none"}
                  class={"border-2 border-purple-500 p-2 rounded text-white font-extrabold bg-purple-400"}
                  >
                      {"Toggle Move"} </button>
                <button
                    id={"create"}
                    class={"border-2 border-red-500 p-2 rounded text-white font-extrabold bg-pink-400"}
                    >
                        {"Create Folder"}
                </button>
                <button
                    class={"border-2 border-lime-500 p-2 rounded text-white font-extrabold bg-lime-400"}
                    id={"update"}
                    >
                        {"Update"}
                </button>
              </section>
            <section
                id={"files"}
                hx-get={"/dirs"}
                hx-swap={"innerHTML"}
                hx-trigger={"load, refetch from:body"}
                >
                {""}
            </section>
         </main>
        </body>
       </html>
     </>
    }
}

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
    app.at("/example").get(example);
    app.at("/get_create_input").get(get_folder_create);
    app.at("/create_folder").post(create_directory);
    app.at("/show").post(showing);
    app.at("/dirs").get(dirs);
    app.at("/select_location").post(update_dir_state);
    app.listen("0.0.0.0:5000").await?;
    Ok(())
}
