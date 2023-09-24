use std::fs;
use std::path::{Path, PathBuf};
use tide::http::mime;
use tide::utils::After;
use tide::{log, Request, Response};
use tide::{prelude::*, Body, StatusCode};
use tide_jsx::html::HTML5Doctype;
use tide_jsx::{component, html, rsx, view};
use urlencoding::decode;

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
fn DirItem(value: PathBuf, parent: bool) {
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

async fn dirs(req: Request<()>) -> tide::Result {
    let current: String = req.session().get("dir").unwrap_or_default();
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
    let mut directories: Vec<_> = folders
        .iter()
        .filter(|d| d.as_ref().unwrap().metadata().unwrap().is_dir())
        .map(|d| DirItem {
            value: d.as_ref().unwrap().path(),
            parent: false,
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
    };
    directories.splice(0..0, vec![parent]);
    let files: Vec<_> = folders
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
                {files}
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

#[derive(Deserialize)]
struct Location {
    destination: String,
}

async fn showing(mut req: Request<()>) -> tide::Result {
    let home_dir = std::env!("HOME").to_string();
    let Location { destination } = req.body_form().await?;
    let base_file = destination.replace(&home_dir, "/files");
    match Path::new(destination.as_str())
        .extension()
        .unwrap_or_default()
        .to_str()
        .unwrap_or_default()
    {
        "mp4" | "mov" => view! {  <Video src={base_file.as_str()} size={"400"}/> },
        _ => view! {<Image src={base_file.as_str()}/>},
    }
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
            <link rel={"stylesheet"} href={"/static/output.css"} media={"all"} />
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
              <section id={"images"} class={"p-1"}>{""}</section>
              <section class={"px-2 gap-2 flex"} id={"controls"}>
                <button
                  hx-get={"/data"}
                  hx-target={"#data"}
                  class={"border-2 border-purple-500 p-2 rounded text-white font-extrabold bg-purple-400"}
                  >
                      {"Rename"} </button>
                <button
                    class={"border-2 border-red-500 p-2 rounded text-white font-extrabold bg-pink-400"}
                    hx-get={"/example"}
                    hx-target={"#images"}>
                        {"Download"}
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
    app.at("/show").post(showing);
    app.at("/dirs").get(dirs);
    app.at("/select_location").post(update_dir_state);
    app.listen("0.0.0.0:5000").await?;
    Ok(())
}
