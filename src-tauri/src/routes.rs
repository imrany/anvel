use actix::{Actor, StreamHandler};
use actix_files::NamedFile;
use actix_multipart::Multipart;
use actix_web::{get, post, web, HttpRequest, HttpResponse, Responder, Result};
use futures_util::stream::StreamExt;
use notify_rust::{Notification, Timeout};
use reqwest::{
    multipart::{Form, Part},
    Client,
};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    // fs::File,
    path,
};
use tokio::{
    fs::{read, File},
    io::AsyncWriteExt,
};
// use serde_json::json;
use actix_web_actors::ws;
use local_ip_address::local_ip;

#[derive(Serialize, Deserialize, Debug)]
struct DirectoryObject {
    root: String,
    name: String,
    path: path::PathBuf,
    metadata: FileMeta,
}
use dirs;
use open;

#[derive(Serialize, Deserialize, Debug)]
struct FileMeta {
    is_file: bool,
    file_extension: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
struct DirectoryContent {
    contents: Vec<DirectoryObject>,
}

#[derive(Serialize)]
struct ErrorMessage {
    message: String,
}

#[derive(Serialize)]
struct IpContent {
    internal: String,
    external: String,
}

#[derive(Deserialize, Clone)]
struct RootPath {
    root: path::PathBuf,
}

#[derive(Deserialize, Clone, Debug)]
struct SendInfo {
    file_path: path::PathBuf,
    file_name: String,
    recipient_server: String,
}

pub struct AppState {
    pub root_dir: path::PathBuf,
    pub home_dir: path::PathBuf,
    pub download_dir: path::PathBuf,
    pub shared_dir: path::PathBuf,
}

#[post("/directory_content")]
pub async fn directory_content(
    state: web::Data<AppState>,
    path: web::Json<RootPath>,
) -> HttpResponse {
    let root = &state.root_dir;
    let path_dir = &path.root;
    let shared_dir = &state.shared_dir;
    let home_dir = &state.home_dir;
    let download_dir = &state.download_dir;
    let audio_dir = &path::PathBuf::from(dirs::audio_dir().unwrap().display().to_string());
    let desktop_dir = &path::PathBuf::from(dirs::desktop_dir().unwrap().display().to_string());
    let picture_dir = &path::PathBuf::from(dirs::picture_dir().unwrap().display().to_string());
    let video_dir = &path::PathBuf::from(dirs::video_dir().unwrap().display().to_string());
    let document_dir = &path::PathBuf::from(dirs::document_dir().unwrap().display().to_string());

    let directory_path = match path_dir.to_str().unwrap() {
        "root" => root,
        "Music" => audio_dir,
        "Desktop" => desktop_dir,
        "Pictures" => picture_dir,
        "Videos" => video_dir,
        "Downloads" => download_dir,
        "Documents" => document_dir,
        "home" => home_dir,
        "Anvel shared" => shared_dir,
        _ => path_dir,
    };

    // Read the directory contents
    let contents = match fs::read_dir(directory_path) {
        Ok(entries) => {
            let mut contents = Vec::new();
            for entry in entries {
                if let Ok(entry) = entry {
                    if let Some(file_name) = entry.file_name().to_str() {
                        let metadata = FileMeta {
                            is_file: directory_path.join(file_name.to_owned()).is_file(),
                            // file_extension: Some(String::from("none"))
                            file_extension: match directory_path
                                .join(file_name.to_owned())
                                .is_file()
                            {
                                true => {
                                    let extension =
                                        match directory_path.join(file_name.to_owned()).extension()
                                        {
                                            Some(v) => format!("{}", v.to_str().unwrap()),
                                            None => String::from("no file extension"),
                                        };
                                    Some(extension)
                                }
                                false => Some(String::from("folder")),
                            },
                        };
                        let directory_object = DirectoryObject {
                            root: format!("{}", directory_path.to_str().unwrap()),
                            name: file_name.to_owned(),
                            path: directory_path.join(file_name.to_owned()),
                            metadata,
                        };
                        contents.push(directory_object);
                    }
                }
            }
            contents
        }
        Err(e) => {
            let err_message = ErrorMessage {
                message: format!("{e} with the name '{}'", directory_path.to_str().unwrap()),
            };
            return HttpResponse::InternalServerError().json(err_message);
        }
    };

    let directory_content = DirectoryContent { contents };
    HttpResponse::Ok().json(&directory_content)
}

#[get("/download/{filename:.*}")]
pub async fn download(req: HttpRequest) -> Result<NamedFile> {
    let path: path::PathBuf = req.match_info().query("filename").parse().unwrap();
    Ok(NamedFile::open(path)?)
}

// #[get("/download/{filename}")]
// pub async fn download(path: web::Path<RootPath>) -> Result<NamedFile> {
//     let file_path= format!("shared/{}",&path.root.to_str().unwrap());
//     Ok(NamedFile::open(file_path)?)
// }

#[get("/ping/{sender_ip}")]
pub async fn ping(sender_ip: web::Path<String>) -> HttpResponse {
    let resp = Client::new()
        .get(format!(
            "http://{sender_ip}:80/api/pong/{}",
            local_ip().unwrap()
        ))
        .send()
        .await;
    match resp {
        Ok(res) => {
            if res.status().is_success() {
                let res_json: String = res.json().await.unwrap();
                return HttpResponse::Ok().json(res_json);
            } else {
                let res_text = format!("Failed to ping. Status code: {}", res.status());
                println!("{res_text}");
                return HttpResponse::InternalServerError().json(res_text);
            }
        }
        Err(e) => {
            let res_text = format!("{e}");
            println!("{res_text}");
            return HttpResponse::InternalServerError().json(res_text);
        }
    }
}

#[get("/pong/{recipient_ip}")]
pub async fn pong(recipient_ip: web::Path<String>) -> HttpResponse {
    Notification::new()
        .summary("Anvel - Ping alert")
        .body(
            format!(
                "Device '{}' can now send you files.",
                &recipient_ip.as_str()
            )
            .as_str(),
        )
        .icon("thunderbird")
        .appname("Anvel")
        .timeout(Timeout::Milliseconds(10000)) //milliseconds
        .show()
        .unwrap();
    HttpResponse::Ok().json("pong")
}

// #[get("/shared_folder")]
// pub async fn get_shared_folder_contents()-> HttpResponse{
//     let shared_folder_path=path::PathBuf::from("./shared");
//     // This will POST a body of `{"root":"rust","body":"json"}`
//     let data = json!({
//         "root": &shared_folder_path
//     });

//     let resp=Client::new()
//     .post("http://localhost:8000/api/directory_content")
//     .json(&data)
//     .send()
//     .await;
//     match resp {
//         Ok(res) =>{
//             if res.status().is_success() {
//                 let res_json:DirectoryContent=res.json().await.unwrap();
//                 return HttpResponse::Ok().json(res_json);
//             } else {
//                 let res_text=format!("Failed to get Shared Folder. Status code: {}",res.status());
//                 println!("{res_text}");
//                 return HttpResponse::InternalServerError().json(res_text);
//             }
//         },
//         Err(e) => {
//             let res_text=format!("{e}");
//             println!("{res_text}");
//             return HttpResponse::InternalServerError().json(res_text);
//         }
//     }
// }

#[post("/receive")]
pub async fn receive(state: web::Data<AppState>, mut payload: Multipart) -> Result<HttpResponse> {
    while let Some(item) = payload.next().await {
        let mut field = item?;
        let content_disposition = field.content_disposition().clone();
        let filename = content_disposition.get_filename().unwrap_or_default();

        // Create a file with a unique name in the server's current directory
        let shared_dir = state.shared_dir.to_str().unwrap();
        let filepath = format!("{shared_dir}/{filename}");
        let mut file = File::create(&filepath).await?;

        // Write file content
        while let Some(chunk) = field.next().await {
            let data = chunk?;
            file.write_all(&data).await?;
        }

        //println!("Received file: {}", filename);
        //show file recieved notification
        Notification::new()
            .summary("Anvel - New file received")
            .body("You've received a new file")
            .icon("thunderbird")
            .appname("Anvel")
            .timeout(Timeout::Milliseconds(10000)) //milliseconds
            .show()
            .unwrap();
    }

    Ok(HttpResponse::Ok().json("File was received successfully"))
}

#[post("/send")]
pub async fn send(resource: web::Json<SendInfo>) -> HttpResponse {
    // Path to the media file you want to send
    // let file_path = "path/to/your/media/file.jpg";
    let file_path = &resource.file_path;
    let file_name = resource.clone().file_name;

    // URL of the server that will receive the file
    // let server_url = "https://example.com/api/receive";
    let server_url = &resource.recipient_server;

    // Read the file asynchronously
    let file_content = read(file_path).await.unwrap();

    // Create a multipart form with the file
    let form = Form::new().part(
        "file",
        Part::bytes(file_content).file_name(file_name.clone()),
    );

    // Send the multipart form to the server
    let response = Client::new().post(server_url).multipart(form).send().await;
    match response {
        Ok(res) => {
            // Check the server's response
            if res.status().is_success() {
                let res_json: String = res.json().await.unwrap();
                return HttpResponse::Ok().json(res_json);
            } else {
                let res_text = format!(
                    "Failed to send '{file_name}' to '{server_url}'.  Status code: {}",
                    res.status()
                );
                return HttpResponse::InternalServerError().json(res_text);
            }
        }
        Err(e) => {
            let res_text = format!("{e}");
            return HttpResponse::InternalServerError().json(res_text);
        }
    }
}

#[post("/open")]
pub async fn open_file(path: web::Json<RootPath>) -> impl Responder {
    let file_path = &path.root;
    let open_file = open::that(file_path);
    if let Ok(_file) = open_file {
        return HttpResponse::Ok().json("File opened");
    } else {
        return HttpResponse::InternalServerError().json("Failed to open file");
    };
}

#[get("/get_ip_address")]
pub async fn get_ip_address() -> impl Responder {
    if let Ok(internal_ip) = local_ip() {
        // Make a request to httpbin to get the external IP address
        if let Ok(response) = reqwest::get("https://httpbin.org/ip").await {
            // Parse the JSON response to extract the IP address
            let ip_address: serde_json::Value = response.json().await.unwrap();
            let ip_external = ip_address["origin"].as_str().unwrap_or("Unknown");
            let ip = IpContent {
                internal: internal_ip.to_string(),
                external: ip_external.to_string(),
            };
            return HttpResponse::Ok().json(ip);
        } else {
            let ip = IpContent {
                internal: internal_ip.to_string(),
                external: "No internet".to_string(),
            };
            return HttpResponse::Ok().json(ip);
        };
    } else {
        let err_message = ErrorMessage {
            message: "Searching for network information failed".to_string(),
        };
        return HttpResponse::InternalServerError().json(err_message);
    }
}

//websocket
struct MyWebSocket;

impl Actor for MyWebSocket {
    type Context = ws::WebsocketContext<Self>;
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for MyWebSocket {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        match msg {
            Ok(ws::Message::Text(text)) => ctx.text(text), // Echo the text back
            Ok(ws::Message::Binary(bin)) => ctx.binary(bin),
            _ => (),
        }
    }
}

#[get("/ws")]
pub async fn websocket(req: HttpRequest, stream: web::Payload) -> impl Responder {
    ws::start(MyWebSocket {}, &req, stream)
}
