use anyhow::{Result, anyhow};
use ha1ku::{episodes, info, search, sources};
use rocket::data::{Data, ToByteUnit};
use rocket::fs::FileServer;
use rocket::response::content::RawHtml;

#[macro_use]
extern crate rocket;

#[launch]
fn rocket() -> _ {
    let path = std::env::current_dir().unwrap().join("public");
    rocket::build()
        .mount("/", FileServer::from(path))
        .mount("/", routes![query_handler, info_handler])
}

#[post("/query", data = "<data>")]
async fn query_handler(data: Data<'_>) -> RawHtml<String> {
    let result = query(
        data.open(2.mebibytes())
            .into_string()
            .await
            .unwrap()
            .into_inner()
            .to_string(),
    )
    .await;
    match result.is_ok() {
        true => {
            dbg!(&result);
            RawHtml(result.unwrap())
        }
        false => {
            dbg!(result.unwrap_err());
            RawHtml(
                "<script>if(!alert(\"A critical error has occured, the page will now reload.\")){window.location.reload();}</script>".to_string(),
            )
            //catchall error for shit that goes wrong in the query function, refreshes the page because error notifications are dependent on request type.
        }
    }
}

async fn query(stream: String) -> Result<String> {
    let wrapped = serde_urlencoded::from_str::<Vec<(String, String)>>(&stream)?;
    let query_type = wrapped[0].0.as_str();
    let query = wrapped[0].1.as_str();
    let mut media_type = "sub";
    if wrapped.len() == 2 {
        media_type = wrapped[1].1.as_str();
    }
    match query_type {
        "search" => Ok(search_format(query)
            .await
            .unwrap_or("<option>error: no results!</option>".to_string())),
        "episodes" => Ok(episodes_format(query)
            .await
            .unwrap_or("<option>error: no episodes found!</option>".to_string())),
        "sources" => Ok(sources_format(query, media_type)
            .await
            .unwrap_or("<a id=\"iframe\">error: no sources found!</a>".to_string())),
        _ => Err(anyhow!("failed to match query type")),
    }
}

async fn sources_format(query: &str, media_type: &str) -> Result<String> {
    let query: Vec<&str> = query.split(':').collect();
    let is_download = media_type.len() > 3;
    let sources_result = sources(query[0], query[1], media_type, is_download).await?;

    let construct = match is_download {
        true => {
            format!(
                r#"<span id="iframe"><a id="video" href="{}" download>download</a></span>"#,
                sources_result[0].link
            )
        }
        false => {
            format!(
                r#"<span id="iframe"><video id="video" crossorigin="anonymous" class="{}" controls><script src="player.js"></script></video></span>"#,
                sources_result[0].link
            )
        }
    };
    Ok(construct)
}

async fn episodes_format(query: &str) -> Result<String> {
    let query: Vec<&str> = query.split(':').collect();
    let episodes_result = episodes(query[0], "1", query[1]).await?;
    let mut construct = String::new();
    for episode in episodes_result.iter() {
        let title = if episode.notes.is_some() {
            format!(
                "{}: {}",
                episode.episodeIdNum,
                episode
                    .notes
                    .as_ref()
                    .unwrap()
                    .split("<note-split>")
                    .collect::<Vec<&str>>()[0]
            )
        } else {
            episode.episodeIdNum.to_string()
        };
        construct.push_str(
            format!(
                r#"<option value="{}:{}">{}</option>"#,
                query[0], episode.episodeIdNum, title
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

#[post("/info", data = "<data>")]
async fn info_handler(data: Data<'_>) -> RawHtml<String> {
    let result = info_query(
        data.open(2.mebibytes())
            .into_string()
            .await
            .unwrap()
            .into_inner()
            .to_string(),
    )
    .await;
    match result.is_ok() {
        true => RawHtml(result.unwrap()),
        false => {
            dbg!(result.unwrap_err());
            RawHtml("ERROR".to_string())
        }
    }
}

async fn info_query(stream: String) -> Result<String> {
    let wrapped = serde_urlencoded::from_str::<Vec<(String, String)>>(&stream)?;
    let query = wrapped[0].1.split(":").collect::<Vec<&str>>()[0];
    info_format(query).await
}

async fn info_format(query: &str) -> Result<String> {
    let info = info(query).await?;
    let mut construct: String = format!(
        r#"<span id=info><img src="{}" class="poster"/><span><h1>{}</h1><span><i>"#,
        info.thumbnail.unwrap_or("".to_string()),
        info.name.unwrap_or("".to_string()),
    );
    if info.score.is_some() {
        construct.push_str(format!("score: {}<br>", info.score.unwrap()).as_str())
    }
    if info.episodeCount.is_some() {
        construct.push_str(format!("episodes: {}<br>", info.episodeCount.unwrap()).as_str())
    }
    if info.status.is_some() {
        construct.push_str(format!("status: {}<br>", info.status.unwrap()).as_str())
    }
    if info.studios.is_some() {
        construct.push_str(format!("studio: {}<br>", info.studios.unwrap()[0]).as_str())
    }
    if info.nativeName.is_some() {
        construct.push_str(format!("original title: {}<br>", info.nativeName.unwrap()).as_str())
    }
    construct.push_str("</i>");
    if info.description.is_some() {
        construct.push_str(format!("<p>{}</p>", info.description.unwrap()).as_str());
    }
    construct.push_str("</span></span></span>");
    Ok(construct)
}
