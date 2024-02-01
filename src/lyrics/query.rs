use std::error::Error;
use std::io::Read;
use std::time::Duration;

use log::error;
use log::info;
use log::warn;
use lrc::Lyrics;
use lrc::TimeTag;
use regex::Regex;
use reqwest::blocking::Client;
use serde::Deserialize;
use serde_json::Value;

pub struct Query {
    client: Client,
    last_query: String,
}

#[derive(Deserialize)]
struct SearchResult {
    result: SearchResultInner,
}

#[derive(Deserialize)]
struct SearchResultInner {
    songs: Vec<Song>,
}

#[derive(Deserialize)]
struct Song {
    id: u32,
    name: String,
    artists: Vec<Artist>,
}

#[derive(Deserialize)]
struct Artist {
    name: String,
}

#[derive(Deserialize)]
struct LyricsResponse {
    lrc: Lrc,
}

#[derive(Deserialize)]
struct Lrc {
    lyric: String,
}

fn convert_lyrics(lyrics: String) -> Result<String, Box<dyn Error>> {
    // 创建正则表达式
    let re = Regex::new(r"\[(\d{2}:\d{2}\.\d{2})\d\]")?;
    // 替换匹配的时间戳
    let result = re.replace_all(&lyrics, "[$1]");
    Ok(result.to_string())
}

fn check_exist(song_id: u32) -> bool {
    let path = format!(
        "E:\\Study\\Projectlearning\\amll-ttml-db\\lrclib\\{}.lrc",
        song_id
    );
    std::path::Path::new(&path).exists()
}

fn read_lrclib_file(song_id: u32) -> Result<String, Box<dyn Error>> {
    // check
    if !check_exist(song_id) {
        return Ok(String::new());
    }
    let path = format!(
        "E:\\Study\\Projectlearning\\amll-ttml-db\\lrclib\\{}.lrc",
        song_id
    );
    let mut file = std::fs::File::open(path)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    Ok(contents)
}

impl Query {
    pub fn new() -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .unwrap();
        Self {
            client,
            last_query: String::from(""),
        }
    }
    pub fn get_lyrics(
        &mut self,
        name: &str,
        artist: &str,
        lyrics: &mut Option<Lyrics>,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        let query = format!("{} {}", name, artist);
        if self.last_query != query {
            self.last_query = query.clone();
            if name.is_empty() || artist.is_empty() {
                *lyrics = None;
                return Ok(true);
            }
            info!("{}", &self.last_query);
            *lyrics = None;

            // 第一步：调用搜索接口获取歌曲 ID
            let search_response = self
                .client
                .get(&format!("http://localhost:3000/search?keywords={}", query))
                .send()?
                .error_for_status()?;
            let search_body = search_response.text()?;
            let search_result: SearchResult = serde_json::from_str(&search_body)?;

            let target_artist_name = String::from(artist);

            let song_id = search_result
                .result
                .songs
                .iter()
                .find(|song| {
                    song.artists
                        .iter()
                        .any(|artist| artist.name == target_artist_name)
                })
                .map(|song| song.id)
                .ok_or("Song not found")?;

            // 第二步：使用歌曲 ID 获取歌词
            // first use manual lyrics in E:\Study\Projectlearning\amll-ttml-db\lrclib\
            // "E:\Study\Projectlearning\amll-ttml-db\lrclib\72394.lrc"
            let mut actual_lyrics = read_lrclib_file(song_id)?;
            if actual_lyrics.is_empty() {
                // if not exist, use api
                let lyrics_response = self
                    .client
                    .get(&format!("http://localhost:3000/lyric?id={}", song_id))
                    .send()?
                    .error_for_status()?;
                let lyrics_body = lyrics_response.text()?;

                // 解析 JSON 并提取歌词
                let lyrics_json: LyricsResponse = serde_json::from_str(&lyrics_body)?;
                actual_lyrics = lyrics_json.lrc.lyric;

                // 处理歌词中的每个时间标记
                actual_lyrics = convert_lyrics(actual_lyrics)?;
            }

            // 处理歌词中的每个时间标记
            actual_lyrics = convert_lyrics(actual_lyrics)?;

            // 使用原有逻辑处理歌词
            let downloaded_lyrics = Lyrics::from_str(&actual_lyrics).map_err(|e| {
                error!("Failed to parse lyrics: {:?}", e);
                e
            })?;
            info!("OK");
            let mut new_lyrics = Lyrics::new();
            let timed_lines = downloaded_lyrics.get_timed_lines();
            for (i, (time_tag, line)) in timed_lines.iter().enumerate() {
                let line = html_escape::decode_html_entities(&line);
                let text = line.trim();
                // Skip empty lines that last no longer than 3s.
                if text.is_empty() {
                    if i < timed_lines.len() - 1 {
                        let duration =
                            timed_lines[i + 1].0.get_timestamp() - time_tag.get_timestamp();
                        if duration <= 3000 {
                            continue;
                        }
                    }
                }
                if name.to_lowercase() == "shanghai breezes" {
                    let new_start = std::cmp::max(0, time_tag.get_timestamp() - 5000);
                    let new_time_tag = TimeTag::new(new_start);

                    new_lyrics.add_timed_line(new_time_tag.clone(), text)?;
                } else {
                    new_lyrics.add_timed_line(time_tag.clone(), text)?;
                }
            }
            *lyrics = Some(new_lyrics);
            Ok(true)
        } else {
            Ok(false)
        }
    }
}
