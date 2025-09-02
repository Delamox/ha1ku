use std::{collections::HashMap, process::exit};
//YYKgYqaJP2sPyYvk4

use anyhow::{Result, anyhow};
use reqwest::{ResponseBuilderExt, blocking::Client};
use serde_derive::Deserialize;

const BASEURL: &str = "https://allmanga.to";
const SEARCHGQL: &str = "query( $search: SearchInput $limit: Int $page: Int $translationType: VaildTranslationTypeEnumType $countryOrigin: VaildCountryOriginEnumType ) { shows( search: $search limit: $limit page: $page translationType: $translationType countryOrigin: $countryOrigin ) { edges { _id name availableEpisodes __typename } }}";
const EPISODELISTGQL: &str =
    "query ($showId: String!) { show( _id: $showId ) { _id availableEpisodesDetail }}";
const EPISODELINKGQL: &str = "query ($showId: String!, $translationType: VaildTranslationTypeEnumType!, $episodeString: String!) { episode( showId: $showId translationType: $translationType episodeString: $episodeString ) { episodeString sourceUrls }}";
const AGENT: &str =
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:109.0) Gecko/20100101 Firefox/121.0";
const ENCRYPT: &str = "175948514e4c4f57175b54575b5307515c050f5c0a0c0f0b0f0c0e590a0c0b5b0a0c0e0a0e0d0b5e0b5e0b0b0e0c0d010b5d0e0f0b090e0f0b0a0e0b0b0c0b0a0e0d0b5e0e0c0b0c0b0a0b0a0b090b5d0b5d0e0d0b080b080b5e0e0d0b0c0e0a0b090b0c0e080b5d0e0c0b0d0e0a0b0a0a0e0f590a0e0e0b0b5d0b0e0e0d0e0a0e080b0b0e0f0b5d0b0c0b5d0b5d0e0b0e0d0e0c0e0c0e0a0e0a0e080b0a0b0a0e0a0b0f0e0d0b0b0e0d0b0e0b5d0e0b0b080e0d0e080a0e0f590a0e0e5a0e0b0e0a0e5e0e0f0a010e0a0e0d0b5e0b5e0b0b0e0c0d010b5d0e0f0b090e0f0b0a0e0b0b0c0b0a0e0d0b5e0e0c0b0c0b0a0b0a0b090b5d0b5d0e0d0b080b080b5e0e0d0b0c0e0a0b090b0c0e080b5d0e0c0b0d0e0a0b0a0e080b0e0b0e0b0f0a000e5b0f0e0e090a0e0f590a0e0a590b0f0b0e0b5d0b0e0f0e0a590b090b0c0b0e0f0e0a590b0a0b5d0b0e0f0e0a590a0c0a590a0c0f0d0f0a0f0c0e0b0e0f0e5a0e0b0f0c0c5e0e0a0a0c0b5b0a0c0d090e5e0f5d0a0c0a590a0c0e0a0e0f0f0a0e0b0a0c0b5b0a0c0b0c0b0e0b0c0b0b0a5a0b0e0b5e0a5a0b0e0b0c0d0a0b0c0b0e0b5b0b0a0b5e0b5b0b0e0b0e0a000b0e0b0e0b0e0d5b0a0c0a590a0c0f0a0f0c0e0f0e000f0d0e590e0f0f0a0e5e0e010e000d0a0f5e0f0e0e0b0a0c0b5b0a0c0f0d0f0b0e0c0a0c0a590a0c0e5c0e0b0f5e0a0c0b5b0a0c0e0b0f0e0a5a0f0e0c0a0d0e0e090e0d0d5e0b090d5d0f080d5b0f5e0b080d0f0c000e0f0b0c0e080d010b0d0d010f0d0f0b0e0c0a0c0f5a";

pub fn main() -> Result<()> {
    // let Ok(response) = search_anime("chuunibyou", "sub") else {
    //     exit(1);
    // };
    // let Ok(response) = episodes_list("pDPgcY7XvZy6QNa2f") else {
    //     exit(1);
    // };
    let Ok(response) = episode_link("pDPgcY7XvZy6QNa2f", "3", "sub") else {
        exit(1);
    };
    // let Ok(response) = substitute_data(ENCRYPT) else {
    //     exit(1);
    // };

    // println!("{response}");
    dbg!(response);
    Ok(())
}

fn substitute_data(input: &str) -> Result<String> {
    let chunks: Vec<&str> = input
        .as_bytes()
        .chunks(2)
        .map(|e| str::from_utf8(e).map_err(|_x| anyhow!("Chunk error")))
        .collect::<Result<Vec<&str>>>()?;

    let maps = get_table();
    let mut out = String::new();
    for chunk in chunks {
        match maps.get(chunk) {
            Some(e) => out.push_str(e),
            None => (),
        }
    }
    Ok(out.replace("clock", "clock.json"))
}

fn search_anime(query: &str, translation: &str) -> Result<Search> {
    let params_raw = format!(
        r#"{{"search":{{"allowAdult":false,"allowUnknown":false,"query":"{}"}},"limit":40,"page":1,"translationType":"{}","countryOrigin":"ALL"}}"#,
        query, translation,
    );

    let params = &[("variables", params_raw.as_str()), ("query", SEARCHGQL)];
    let params_serialized = serde_urlencoded::to_string(&params)?;

    let response_raw = api_call(&params_serialized)?;
    let response_serialized: Search = serde_json::from_str(
        &response_raw
            .split_at(17)
            .1
            .split_at(response_raw.len() - 20)
            .0,
    )?;
    Ok(response_serialized)
}

fn episodes_list(id: &str) -> Result<()> {
    let params_raw = format!(r#"{{"showId":"{}"}}"#, id);
    let params = &[
        ("variables", params_raw.as_str()),
        ("query", EPISODELISTGQL),
    ];
    let params_serialized = serde_urlencoded::to_string(&params)?;
    let response_raw = api_call(&params_serialized)?;
    dbg!(response_raw);
    Ok(())
}

fn episode_link(id: &str, episode_string: &str, translation: &str) -> Result<Vec<EpisodeSource>> {
    let params_raw = format!(
        r#"{{"showId":"{}","translationType":"{}","episodeString":"{}"}}"#,
        id, translation, episode_string
    );
    let params = &[
        ("variables", params_raw.as_str()),
        ("query", EPISODELINKGQL),
    ];
    let params_serialized = serde_urlencoded::to_string(&params)?;
    let response_raw = api_call(&params_serialized)?;
    let response_serialized: EpisodeWrapper = serde_json::from_str(
        &response_raw
            .split_at(8)
            .1
            .split_at(response_raw.len() - 10)
            .0,
    )?;
    Ok(response_serialized.episode.sourceUrls)
}

fn api_call(params_serialized: &str) -> Result<String> {
    let response_raw = Client::new()
        .get(format!(
            "https://api.allanime.day/api?{}",
            params_serialized
        ))
        .header("Referer", BASEURL)
        .header("Agent", AGENT)
        .send()?
        .text()?;
    Ok(response_raw)
}

#[derive(Deserialize, Debug)]
#[allow(non_snake_case, dead_code)]
struct EpisodeWrapper {
    episode: Episode,
}
#[derive(Deserialize, Debug)]
#[allow(non_snake_case, dead_code)]
struct Episode {
    episodeString: String,
    sourceUrls: Vec<EpisodeSource>,
}
#[derive(Deserialize, Debug)]
#[allow(non_snake_case, dead_code)]
struct EpisodeSource {
    sourceUrl: String,
    sourceName: String,
}

// search entry structs
#[derive(Deserialize, Debug)]
#[allow(non_snake_case, dead_code)]
struct Search {
    edges: Vec<SearchField>,
}
#[derive(Deserialize, Debug)]
#[allow(non_snake_case, dead_code)]
struct SearchField {
    _id: String,
    name: String,
    availableEpisodes: AvailableEpisodes,
    __typename: String,
}
#[derive(Deserialize, Debug)]
#[allow(non_snake_case, dead_code)]
struct AvailableEpisodes {
    sub: u128,
    dub: u128,
    raw: u128,
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
