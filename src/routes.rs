use crate::components::{CreateFolderInput, DirItem, FileItem, Heading, Image, SearchInput, Video};
use std::fs;
use std::path::Path;
use tide::StatusCode;
use tide::{prelude::*, Request, Response};
use tide_jsx::html::HTML5Doctype;
use tide_jsx::{html, view};
use urlencoding::decode;

pub async fn dirs(req: Request<()>) -> tide::Result {
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

pub async fn search(req: Request<()>) -> tide::Result {
    let home_dir = std::env!("HOME").to_string();
    let session: String = match req.session().get("dir") {
        Some(s) => s,
        None => home_dir,
    };
    view! {
        <SearchInput value={session}/>
    }
}

pub async fn get_folder_create(_req: Request<()>) -> tide::Result {
    view! {
        <CreateFolderInput/>
    }
}

#[derive(Deserialize)]
struct Location {
    destination: String,
}

pub async fn showing(mut req: Request<()>) -> tide::Result {
    let home_dir = std::env!("HOME").to_string();
    let Location { destination } = req.body_form().await?;
    let dest = decode(&destination)?; //windows support
    let session = req.session_mut();
    let base_file = dest.replace(&home_dir, "/files");
    let source = match Path::new(destination.as_str())
        .extension()
        .unwrap_or_default()
        .to_str()
        .unwrap_or_default()
    {
        "mp4" | "mov" => html! {  <Video src={base_file.as_str()} size={"400"}/> },
        _ => html! {<Image src={base_file.as_str()}/>},
    };
    session.insert("showcase", destination)?;
    Ok(Response::builder(tide::http::StatusCode::Ok)
        .content_type(tide::http::mime::HTML)
        .header("HX-Trigger-After-Settle", "fetchrename")
        .body(source)
        .build())
}

pub async fn getrenameinput(req: Request<()>) -> tide::Result {
    let session = req.session();
    let showcase: String = session.get("showcase").unwrap_or("".to_string());
    let showcase = decode(&showcase)?.to_string();
    view! {
        <input
            type={"search"}
            name={"destination"}
            class={"p-2 border-2 border-lime-500 rounded w-1/3"}
            hx-post={"/rename_file"}
            hx-trigger={"click from:#rename"}
            hx-target={"#images"}
            placeholder={"Rename current file..."}
            value={showcase}
        />
    }
}

pub async fn renamefile(mut req: Request<()>) -> tide::Result {
    let Location { destination } = req.body_form().await?;
    let destination = decode(&destination)?.to_string();
    let session = req.session_mut();
    let home_dir = std::env!("HOME").to_string();
    let showcase: String = session.get("showcase").unwrap_or("".to_string());
    let showcase = decode(&showcase)?.to_string();
    if destination.is_empty() {
        return Ok(Response::builder(StatusCode::NotFound).build());
    }
    let dir_path = match session.get::<String>("dir") {
        Some(d) => Path::new(&d)
            .join(destination)
            .to_str()
            .unwrap()
            .to_string(),
        None => home_dir.to_string(),
    };
    fs::rename(showcase, dir_path)?;
    Ok(Response::builder(tide::http::StatusCode::Ok)
        .header("HX-Trigger-After-Settle", "refetch")
        .body("")
        .build())
}

pub async fn outputdir(_req: Request<()>) -> tide::Result {
    view! {
        <output
          hx-get={"/get_rename_input"}
          id={"update_input"}
          hx-trigger={"load once, fetchrename from:body"}
          hx-swap={"innerHTML"}
        >
            {""}
        </output>

    }
}

pub async fn update_dir_state(mut req: Request<()>) -> tide::Result {
    let Location { destination } = req.body_form().await?;
    let destination = decode(&destination)?.to_string();
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

pub async fn move_file(mut req: Request<()>) -> tide::Result {
    let Location { destination } = req.body_form().await?;
    let destination = decode(&destination)?.to_string();
    let session = req.session_mut();
    let home_dir = std::env!("HOME");
    let file_path = session.get::<String>("showcase").unwrap_or("".to_string());
    let file_path = decode(&file_path)?.to_string();
    if file_path.is_empty() || home_dir.is_empty() {
        return Ok(Response::builder(StatusCode::NotFound).build());
    };
    let filename = Path::new(&file_path).file_name().unwrap().to_str().unwrap();
    let dir_path = match session.get::<String>("dir") {
        Some(d) => Path::new(&d)
            .join(destination)
            .join(filename)
            .to_str()
            .unwrap()
            .to_string(),
        None => home_dir.to_string(),
    };
    fs::rename(file_path, dir_path)?;
    session.insert("showcase", "")?;
    Ok(Response::builder(tide::http::StatusCode::Ok)
        .header("HX-Trigger-After-Settle", "refetch")
        .body("")
        .build())
}

pub async fn toggle_move(mut req: Request<()>) -> tide::Result {
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

pub async fn create_directory(mut req: Request<()>) -> tide::Result {
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

pub async fn index(mut req: Request<()>) -> tide::Result {
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
                hx-target={"#update_input"}
                >
                <output id={"update_input"}>{""}</output>
              </section>
              <section id={"images"} class={"p-1"}>{""}</section>
              <section class={"px-2 gap-2 flex"} id={"controls"}>
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
                <button
                    class={"border-2 border-sky-500 p-2 rounded text-white font-extrabold bg-sky-400"}
                    id={"rename"}
                    hx-get={"/output_dir"}
                    hx-target={"#create_folder"}
                    hx-trigger={"click once"}
                    >
                        {"Rename"}
                </button>
                <button
                  hx-post={"/togglemove"}
                  hx-swap={"none"}
                  class={"border-2 border-purple-500 p-2 rounded text-white font-extrabold bg-purple-400"}
                  >
                  {"Toggle Move"}
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
