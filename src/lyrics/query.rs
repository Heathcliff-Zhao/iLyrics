use std::time::Duration;

use log::error;
use log::info;
use log::warn;
use lrc::Lyrics;
use reqwest::blocking::Client;
use rusqlite::{params, Connection};

pub struct Query {
    client: Client,
    last_query: String,
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
            self.last_query = query;
            if name.is_empty() || artist.is_empty() {
                *lyrics = None;
                return Ok(true);
            }
            info!("{}", &self.last_query);

            // 连接到本地数据库
            let conn = Connection::open("./songs.db")?;
            let mut stmt = conn.prepare("SELECT lyrics FROM songs WHERE name=? AND artist=?")?;

            let rows = stmt.query_map(params![name, artist], |row| row.get(0))?;

            // 如果找到歌词
            if let Some(row) = rows.into_iter().next() {
                let lyrics_text: String = row?;
                let downloaded_lyrics = Lyrics::from_str(&lyrics_text).map_err(|e| {
                    error!("Failed to parse lyrics: {:?}", e);
                    e
                })?;
                info!("OK");
                let mut new_lyrics = Lyrics::new();
                let timed_lines = downloaded_lyrics.get_timed_lines();
                for (i, (time_tag, line)) in timed_lines.iter().enumerate() {
                    let line = html_escape::decode_html_entities(&line);
                    let text = line.trim();
                    if text.is_empty() {
                        if i < timed_lines.len() - 1 {
                            let duration =
                                timed_lines[i + 1].0.get_timestamp() - time_tag.get_timestamp();
                            if duration <= 3000 {
                                continue;
                            }
                        }
                    }
                    new_lyrics.add_timed_line(time_tag.clone(), line)?;
                }
                *lyrics = Some(new_lyrics);
            } else {
                *lyrics = None;
            }

            Ok(true)
        } else {
            Ok(false)
        }
    }
}
// TODO: async
// TODO: refactor
//     pub fn get_lyrics(
//         &mut self,
//         name: &str,
//         artist: &str,
//         lyrics: &mut Option<Lyrics>,
//     ) -> Result<bool, Box<dyn std::error::Error>> {
//         let query = format!("{} {}", name, artist);
//         if self.last_query != query {
//             self.last_query = query;
//             if name.is_empty() || artist.is_empty() {
//                 *lyrics = None;
//                 return Ok(true);
//             }
//             info!("{}", &self.last_query);
//             *lyrics = None;
//             let response = self
//                 .client
//                 .get("https://lyrics-api.lujjjh.com/")
//                 .query(&[("name", name), ("artist", artist)])
//                 .send()
//                 .map_err(|e| {
//                     error!("Network error: {:?}", e);
//                     e
//                 })?
//                 .error_for_status()
//                 .map_err(|e| {
//                     warn!("Bad status: {:?}", e);
//                     e
//                 })?;
//             // true url
//             info!("{}", response.url());
//             let body = response.text().map_err(|e| {
//                 error!("Failed to read response body: {:?}", e);
//                 e
//             })?;
//             let downloaded_lyrics = Lyrics::from_str(body).map_err(|e| {
//                 error!("Failed to parse lyrics: {:?}", e);
//                 e
//             })?;
//             info!("OK");
//             let mut new_lyrics = Lyrics::new();
//             let timed_lines = downloaded_lyrics.get_timed_lines();
//             for (i, (time_tag, line)) in timed_lines.iter().enumerate() {
//                 let line = html_escape::decode_html_entities(&line);
//                 let text = line.trim();
//                 // Skip empty lines that last no longer than 3s.
//                 if text.is_empty() {
//                     if i < timed_lines.len() - 1 {
//                         let duration =
//                             timed_lines[i + 1].0.get_timestamp() - time_tag.get_timestamp();
//                         if duration <= 3000 {
//                             continue;
//                         }
//                     }
//                 }
//                 new_lyrics.add_timed_line(time_tag.clone(), line)?;
//             }
//             *lyrics = Some(new_lyrics);
//             Ok(true)
//         } else {
//             Ok(false)
//         }
//     }
// }
