// use std::sync::Arc;

use tide::http::mime;
use tide::utils::After;
use tide::{log, Request, Response};
use tide_jsx::html::HTML5Doctype;
use tide_jsx::{component, html, rsx, view};

// #[derive(Clone, Debug)]
// struct AppState {
//     current_dir: Arc<String>,
// }

// impl AppState {
//     fn new() -> Self {
//         let home_dir = std::env!("HOME").to_string();
//         Self {
//             current_dir: Arc::new(home_dir),
//         }
//     }
//     fn update(mut self, new_dir: String) {
//         self.current_dir = Arc::new(new_dir);
//     }
// }

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
    // let home_dir = std::env!("HOME");
    rsx! {
        <input
            type={"search"}
            class={"p-2 border-2 border-pink-500 rounded w-1/4"}
            value={value}
        />
    }
}

#[component]
fn Image<'src>(src: &'src str) {
    rsx! {
        <img class={"pt-2"} src={src} alt={"server-image"}/>
    }
}

async fn search(req: Request<()>) -> tide::Result {
    let session: String = match req.session().get("dir") {
        Some(s) => s,
        None => "...".to_string()
    };
    view! {
        <SearchInput value={session}/>
    }
}

async fn example(mut req: Request<()>) -> tide::Result {
    let actual = html! {
        <Image src={"/files/Pictures/7b7.png"}/>
    };
    let session = req.session_mut();
    session.insert("dir", "/home".to_string())?;
    Ok(Response::builder(tide::http::StatusCode::Ok)
        .content_type(tide::http::mime::HTML)
        .header("x-api-key", "pandamonium")
        .body(actual)
        .build())
}

async fn index(_req: Request<()>) -> tide::Result {
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
                hx-trigger={"load"}
                hx-target={"#search_input"}
                >
                <output id={"search_input"}></output>
              </section>
              <section class={"px-2 gap-2 flex"} id={"controls"}>
                <button
                  class={"border-2 border-purple-500 p-2 rounded text-white font-extrabold bg-purple-400"}
                  hx-get={"/test"}
                  hx-target={"#test"}>
                      {"Toggle Move"}
                </button>
                <button
                    class={"border-2 border-red-500 p-2 rounded text-white font-extrabold bg-pink-400"}
                    hx-get={"/example"}
                    hx-target={"#images"}>
                        {"Download"}
                </button>
                <button
                    class={"border-2 border-lime-500 p-2 rounded text-white font-extrabold bg-lime-400"}
                    hx-get={"/data"}
                    hx-target={"#data"}>
                        {"Rename"}
                </button>
              </section>
              <section id={"images"} class={"p-3"}>
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
    let home_dir = std::env!("HOME");
    // let state = AppState::new();
    // let mut app = tide::with_state(state);
    let mut app = tide::new();
    // let dir = &app.state().current_dir.clone();
    app.with(tide::log::LogMiddleware::new());
    app.with(tide::sessions::SessionMiddleware::new(
        tide::sessions::MemoryStore::new(), 
        b"47f9a496e1eedfdd72ecd3d16d0d0744"
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
    app.at("/files").serve_dir(home_dir)?;
    app.at("/static").serve_dir("./static")?;
    app.at("/example").get(example);
    app.listen("0.0.0.0:5000").await?;
    Ok(())
}
