use anyhow::{Result, anyhow};
use base64::prelude::*;
use ha1ku::{episodes, search, sources};
use rocket::data::{Data, ToByteUnit};
use rocket::fs::FileServer;
use rocket::response::content::RawHtml;
use rocket::tokio::task::spawn_blocking;

#[macro_use]
extern crate rocket;

#[launch]
fn rocket() -> _ {
    let path = std::env::current_dir().unwrap().join("public");
    rocket::build()
        .mount("/", FileServer::from(path))
        .mount("/", routes![query_handler])
}

#[post("/query", data = "<data>")]
async fn query_handler(data: Data<'_>) -> RawHtml<String> {
    let Ok(result) = query(
        data.open(2.mebibytes())
            .into_string()
            .await
            .unwrap()
            .into_inner()
            .to_string(),
    )
    .await
    else {
        return RawHtml("ERROR".to_string());
    };
    RawHtml(result)
}
async fn query(stream: String) -> Result<String> {
    let wrapped = serde_urlencoded::from_str::<Vec<(String, String)>>(&stream)?;
    let query_type = wrapped[0].0.as_str();
    let query = wrapped[0].1.as_str();
    match query_type {
        "search" => search_format(query).await,
        "select" => episodes_format(query).await,
        "index" => sources_format(query).await,
        _ => forced_error(anyhow!("failed to match query type")),
    }
}

async fn sources_format(query: &str) -> Result<String> {
    let query: Vec<&str> = query.split(':').collect();
    let sources_result = sources(query[0], query[1], "sub").await?;
    let construct: String = format!(
        r#"<span id="iframe"><video id="video" crossorigin="anonymous" class="{}" controls><script src="player.js"></script></video></span>"#,
        sources_result[0].link
    );
    Ok(construct)
}

async fn episodes_format(query: &str) -> Result<String> {
    let query: Vec<&str> = query.split(':').collect();
    let episodes_result = episodes(query[0], "1", query[1]).await?;
    let mut construct = String::new();
    for episode in episodes_result.iter() {
        construct.push_str(
            format!(
                r#"<option value="{}:{}">{}</option>"#,
                query[0], episode.episodeIdNum, episode.episodeIdNum
            )
            .as_str(),
        );
    }
    Ok(construct)
}

async fn search_format(query: &str) -> Result<String> {
    let search_result = search(query, "sub").await?;
    let mut construct = String::new();
    for anime in search_result.iter() {
        construct.push_str(
            format!(
                r#"<option value="{}:{}">{}</option>"#,
                anime._id, anime.availableEpisodes.sub, anime.name
            )
            .as_str(),
        );
    }
    Ok(construct)
}

fn forced_error(e: anyhow::Error) -> Result<String> {
    Err(e)
}

// match request[0].0.as_str() {
//     "search" => {
//         let Ok(object) = searchscrape(request[0].1.as_str()) else {
//             panic!("{}", API_ERR)
//         };
//         RawHtml(search(object))
//     }
//     "select" => {
//         let Ok(object) = selectscrape(request[0].1.as_str()) else {
//             panic!("{}", API_ERR)
//         };
//         RawHtml(select(object))
//     }
//     "index" => {
//         let Ok(json) = searchapi(request[0].1.as_str().to_string().as_str()) else {
//             panic!("{}", API_ERR)
//         };
//         RawHtml(index(json))
//     }
//     "info" => RawHtml("ok".to_string()),
//     _ => panic!("invalid"),
// }
