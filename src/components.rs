use tide_jsx::{component, rsx};
use std::path::PathBuf;
use urlencoding::encode;

#[component]
pub fn Heading<'title>(title: &'title str) {
    rsx! {
        <h1 class={"text-pink-500 text-3xl font-extrabold p-2"}>
            {title}
        </h1>
    }
}

#[component]
pub fn SearchInput(value: String) {
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
pub fn CreateFolderInput() {
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
pub fn FileItem(value: PathBuf) {
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
    let path = encode(&path); //windows support
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
pub fn Image<'src>(src: &'src str) {
    rsx! {
        <img class={"pt-2 w-48 h-48"} src={src} alt={"server-image"}/>
    }
}

#[component]
pub fn Video<'src>(src: &'src str, size: &'src str) {
    rsx! {
    <video controls={"true"} width={size} height={size}>
        <source src={src}></source>
    </video>
    }
}

#[component]
pub fn DirItem(value: PathBuf, parent: bool, move_mode: bool) {
    let path = if !parent {
        value
            .file_stem()
            .unwrap_or_default()
            .to_str()
            .unwrap_or_default()
    } else {
        value.to_str().unwrap_or_default()
    };
    let path = encode(&path);
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
