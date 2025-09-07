use anyhow::{Context, Result, anyhow};
use reqwest::Client;
use serde_derive::Deserialize;
use serde_json::Number;
use std::{cmp::Ordering, collections::HashMap};

const BASEURL: &str = "https://allmanga.to";
const SEARCHGQL: &str = "query( $search: SearchInput $limit: Int $page: Int $translationType: VaildTranslationTypeEnumType $countryOrigin: VaildCountryOriginEnumType ) { shows( search: $search limit: $limit page: $page translationType: $translationType countryOrigin: $countryOrigin ) { edges { _id name availableEpisodes __typename } }}";
const EPISODEGQL: &str = "query ($showId: String!, $episodeNumStart: Float!, $episodeNumEnd: Float!) { episodeInfos( showId: $showId, episodeNumStart: $episodeNumStart, episodeNumEnd: $episodeNumEnd ) { episodeIdNum, notes, vidInforssub, vidInforsdub, vidInforsraw }}";
const SOURCEGQL: &str = "query ($showId: String!, $translationType: VaildTranslationTypeEnumType!, $episodeString: String!) { episode( showId: $showId translationType: $translationType episodeString: $episodeString ) { sourceUrls }}";
const INFOGQL: &str = "query ($_id: String!) { show ( _id: $_id ) { name, englishName, nativeName, thumbnail, score, episodeCount, description, status, studios }}";
const AGENT: &str =
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:109.0) Gecko/20100101 Firefox/121.0";

pub async fn search(query: &str, translation: &str) -> Result<Vec<Search>> {
    let params_raw = format!(
        r#"{{"search":{{"allowAdult":false,"allowUnknown":false,"query":"{}"}},"limit":40,"page":1,"translationType":"{}","countryOrigin":"ALL"}}"#,
        query, translation,
    );

    let params = &[("variables", params_raw.as_str()), ("query", SEARCHGQL)];
    let params_serialized = serde_urlencoded::to_string(params)?;

    let response_raw = api_call(&params_serialized).await?;
    let response_serialized: SearchWrapper = serde_json::from_str(
        response_raw
            .split_at(17)
            .1
            .split_at(response_raw.len() - 20)
            .0,
    )?;
    Ok(response_serialized.edges)
}

pub async fn episodes(id: &str, min: &str, max: &str) -> Result<Vec<Episode>> {
    let params_raw = format!(
        r#"{{"showId":"{}","episodeNumStart":{},"episodeNumEnd":{}}}"#,
        id, min, max,
    );
    let params = &[("variables", params_raw.as_str()), ("query", EPISODEGQL)];
    let params_serialized = serde_urlencoded::to_string(params)?;
    let response_raw = api_call(&params_serialized).await?;
    let mut response_serialized: EpisodeWrapper = serde_json::from_str(
        response_raw
            .split_at(8)
            .1
            .split_at(response_raw.len() - 10)
            .0,
    )?;
    response_serialized
        .episodeInfos
        .retain(|x| x.vidInforssub.is_some());
    let mut failed = false;
    response_serialized.episodeInfos.sort_by(|x, y| {
        match x
            .episodeIdNum
            .as_f64()
            .partial_cmp(&y.episodeIdNum.as_f64())
        {
            Some(o) => o,
            None => {
                failed = true;
                Ordering::Equal
            }
        }
    });
    match failed {
        true => Err(anyhow!("failed ordering episode list")),
        false => Ok(response_serialized.episodeInfos),
    }
}

pub async fn sources(
    id: &str,
    episode_string: &str,
    media_type: &str,
    is_download: bool,
) -> Result<Vec<Link>> {
    let params_raw = format!(
        r#"{{"showId":"{}","translationType":"{}","episodeString":"{}"}}"#,
        id,
        &media_type[0..3],
        episode_string
    );
    let params = &[("variables", params_raw.as_str()), ("query", SOURCEGQL)];
    let params_serialized = serde_urlencoded::to_string(params)?;
    let response_raw = api_call(&params_serialized).await?;
    let mut response_serialized: SourceWrapper = serde_json::from_str(
        response_raw
            .split_at(19)
            .1
            .split_at(response_raw.len() - 22)
            .0,
    )?;
    match is_download {
        true => {
            response_serialized.sourceUrls.retain(|x| {
                x.r#type == "iframe" && matches!(x.sourceName.as_str(), "S-mp4" | "Yt-mp4")
            });
        }
        false => {
            response_serialized.sourceUrls.retain(|x| {
                x.r#type == "iframe"
                    && matches!(x.sourceName.as_str(), "Default" | "S-mp4" | "Yt-mp4")
            });
        }
    }
    response_serialized
        .sourceUrls
        .sort_by(|x, y| y.priority.total_cmp(&x.priority));
    let response_decrypted = substitute_data(response_serialized.sourceUrls[0].sourceUrl.as_str())?;

    let response_raw = Client::new()
        .get(format!("https://allanime.day{}", response_decrypted))
        .header("Referer", BASEURL)
        .header("Agent", AGENT)
        .send()
        .await?
        .text()
        .await?;
    let response_serialized: LinkWrapper = serde_json::from_str(response_raw.as_str())?;
    Ok(response_serialized.links)
}

pub async fn info(query: &str) -> Result<Info> {
    let params_raw = format!(r#"{{"_id":"{}"}}"#, query);
    let params = &[("variables", params_raw.as_str()), ("query", INFOGQL)];
    let params_serialized = serde_urlencoded::to_string(params)?;

    let response_raw = api_call(&params_serialized).await?;
    let response_serialized: Info = serde_json::from_str(
        response_raw
            .split_at(16)
            .1
            .split_at(response_raw.len() - 19)
            .0,
    )
    .unwrap();
    Ok(response_serialized)
}

fn substitute_data(input: &str) -> Result<String> {
    let chunks: Vec<&str> = input.as_bytes()[2..]
        .chunks(2)
        .map(|e| str::from_utf8(e).map_err(|_x| anyhow!("Chunk error")))
        .collect::<Result<Vec<&str>>>()
        .context("this crashed lmao")?;

    let maps = get_table();
    let mut out = String::new();
    for chunk in chunks {
        if let Some(e) = maps.get(chunk) {
            out.push_str(e)
        }
    }
    Ok(out.replace("clock", "clock.json"))
}

async fn api_call(params_serialized: &str) -> Result<String> {
    let response_raw = Client::new()
        .get(format!(
            "https://api.allanime.day/api?{}",
            params_serialized
        ))
        .header("Referer", BASEURL)
        .header("Agent", AGENT)
        .send()
        .await?
        .text()
        .await?;
    Ok(response_raw)
}

#[derive(Deserialize, Debug)]
#[allow(non_snake_case, dead_code)]
pub struct Info {
    pub name: Option<String>,
    pub englishName: Option<String>,
    pub nativeName: Option<String>,
    pub thumbnail: Option<String>,
    pub score: Option<Number>,
    pub episodeCount: Option<String>,
    pub description: Option<String>,
    pub status: Option<String>,
    pub studios: Option<Vec<String>>,
}

#[derive(Deserialize, Debug)]
#[allow(non_snake_case, dead_code)]
struct LinkWrapper {
    links: Vec<Link>,
}
#[derive(Deserialize, Debug)]
#[allow(non_snake_case, dead_code)]
pub struct Link {
    pub link: String,
}

#[derive(Deserialize, Debug)]
#[allow(non_snake_case, dead_code)]
struct EpisodeWrapper {
    episodeInfos: Vec<Episode>,
}
#[derive(Deserialize, Debug)]
#[allow(non_snake_case, dead_code)]
pub struct Episode {
    pub episodeIdNum: Number,
    pub notes: Option<String>,
    vidInforssub: Option<Null>,
    vidInforsdub: Option<Null>,
    vidInforsraw: Option<Null>,
}
#[derive(Deserialize, Debug)]
#[allow(non_snake_case, dead_code)]
pub struct Null {
    vidResolution: u64,
}

#[derive(Deserialize, Debug)]
#[allow(non_snake_case, dead_code)]
struct SourceWrapper {
    sourceUrls: Vec<Source>,
}
#[derive(Deserialize, Debug)]
#[allow(non_snake_case, dead_code)]
pub struct Source {
    pub sourceUrl: String,
    pub sourceName: String,
    r#type: String,
    priority: f32,
}

#[derive(Deserialize, Debug)]
#[allow(non_snake_case, dead_code)]
struct SearchWrapper {
    edges: Vec<Search>,
}
#[derive(Deserialize, Debug)]
#[allow(non_snake_case, dead_code)]
pub struct Search {
    pub _id: String,
    pub name: String,
    pub availableEpisodes: AvailableEpisodes,
    __typename: String,
}
#[derive(Deserialize, Debug)]
#[allow(non_snake_case, dead_code)]
pub struct AvailableEpisodes {
    pub sub: u128,
    pub dub: u128,
    pub raw: u128,
}

fn get_table() -> HashMap<&'static str, &'static str> {
    HashMap::from([
        ("79", "A"),
        ("7a", "B"),
        ("7b", "C"),
        ("7c", "D"),
        ("7d", "E"),
        ("7e", "F"),
        ("7f", "G"),
        ("70", "H"),
        ("71", "I"),
        ("72", "J"),
        ("73", "K"),
        ("74", "L"),
        ("75", "M"),
        ("76", "N"),
        ("77", "O"),
        ("68", "P"),
        ("69", "Q"),
        ("6a", "R"),
        ("6b", "S"),
        ("6c", "T"),
        ("6d", "U"),
        ("6e", "V"),
        ("6f", "W"),
        ("60", "X"),
        ("61", "Y"),
        ("62", "Z"),
        ("59", "a"),
        ("5a", "b"),
        ("5b", "c"),
        ("5c", "d"),
        ("5d", "e"),
        ("5e", "f"),
        ("5f", "g"),
        ("50", "h"),
        ("51", "i"),
        ("52", "j"),
        ("53", "k"),
        ("54", "l"),
        ("55", "m"),
        ("56", "n"),
        ("57", "o"),
        ("48", "p"),
        ("49", "q"),
        ("4a", "r"),
        ("4b", "s"),
        ("4c", "t"),
        ("4d", "u"),
        ("4e", "v"),
        ("4f", "w"),
        ("40", "x"),
        ("41", "y"),
        ("42", "z"),
        ("08", "0"),
        ("09", "1"),
        ("0a", "2"),
        ("0b", "3"),
        ("0c", "4"),
        ("0d", "5"),
        ("0e", "6"),
        ("0f", "7"),
        ("00", "8"),
        ("01", "9"),
        ("15", "-"),
        ("16", "."),
        ("67", "_"),
        ("46", "~"),
        ("02", ":"),
        ("17", "/"),
        ("07", "?"),
        ("1b", "#"),
        ("63", "["),
        ("65", "]"),
        ("78", "@"),
        ("19", "!"),
        ("1c", "$"),
        ("1e", "&"),
        ("10", "("),
        ("11", ")"),
        ("12", "*"),
        ("13", "+"),
        ("14", ","),
        ("03", ";"),
        ("05", "="),
        ("1d", "%"),
    ])
}
