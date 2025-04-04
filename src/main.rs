use aspasia::{AssSubtitle, SubRipSubtitle, Subtitle, WebVttSubtitle};
use chardetng::EncodingDetector;
use clap::Parser;
use lazy_static::lazy_static;
use lrc::Lyrics;
use regex::Regex;
use std::{fs::File, io::Read, path::PathBuf, str::FromStr};
use walkdir::WalkDir;
use whichlang::{Lang, detect_language};

lazy_static! {
    static ref ASS_TAGS: Regex = Regex::new(r"\{[^}]*\}").unwrap();
    static ref HTML_TAGS: Regex = Regex::new(r"<[^>]+>").unwrap();
}

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Input {
    path: PathBuf,
}

fn main() {
    let input = Input::parse();
    let supported_extensions = ["ass", "srt", "vtt", "lrc"];
    let mut chinese_count = 0;
    let mut japanese_count = 0;

    for entry in WalkDir::new(input.path) {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };

        if !entry.file_type().is_file() {
            continue;
        }

        let path = entry.path();
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_lowercase())
            .unwrap_or_default();

        if !supported_extensions.contains(&&*ext) {
            continue;
        }

        let mut file = match File::open(path) {
            Ok(f) => f,
            Err(e) => {
                eprintln!("Error opening {}: {}", path.display(), e);
                continue;
            }
        };

        let mut bytes = Vec::new();
        if let Err(e) = file.read_to_end(&mut bytes) {
            eprintln!("Error reading {}: {}", path.display(), e);
            continue;
        };

        let mut detector = EncodingDetector::new();
        detector.feed(&bytes, true);
        let encoding = detector.guess(None, true);
        let (cow, _, had_errors) = encoding.decode(&bytes);
        if had_errors {
            eprintln!("Warning: decoding errors in {}", path.display());
        }
        let mut contents = cow.into_owned();

        if let Err(e) = file.read_to_string(&mut contents) {
            eprintln!("Error reading {}: {}", path.display(), e);
            continue;
        }

        let texts = match ext.as_str() {
            "ass" => parse_ass(&contents, path),
            "srt" => parse_srt(&contents, path),
            "vtt" => parse_vtt(&contents, path),
            "lrc" => parse_lrc(&contents, path),
            _ => continue,
        };

        // println!("{:#?}", texts);

        for text in texts {
            let cleaned = clean_text(&text);
            match detect_language(&cleaned) {
                Lang::Cmn => chinese_count += 1,
                Lang::Jpn => japanese_count += 1,
                _ => {}
            }
        }
    }

    println!(
        r#"{{"progress": {}}}"#,
        chinese_count as f32 / (japanese_count + chinese_count) as f32
    );
}

fn clean_text(text: &str) -> String {
    let text = ASS_TAGS.replace_all(text, "");
    let text = HTML_TAGS.replace_all(&text, "");
    text.trim().to_string()
}

fn parse_ass(contents: &str, path: &std::path::Path) -> Vec<String> {
    let ass = match AssSubtitle::from_str(contents) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("Failed to parse ASS file {}: {}", path.display(), e);
            return Vec::new();
        }
    };
    ass.events().iter().map(|e| e.text.clone()).collect()
}

fn parse_srt(contents: &str, path: &std::path::Path) -> Vec<String> {
    let srt = match SubRipSubtitle::from_str(contents) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Failed to parse SRT file {}: {}", path.display(), e);
            return Vec::new();
        }
    };
    srt.events().iter().map(|e| e.text.clone()).collect()
}

fn parse_vtt(contents: &str, path: &std::path::Path) -> Vec<String> {
    let vtt = match WebVttSubtitle::from_str(contents) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("Failed to parse VTT file {}: {}", path.display(), e);
            return Vec::new();
        }
    };
    vtt.events().iter().map(|e| e.text.clone()).collect()
}

fn parse_lrc(contents: &str, path: &std::path::Path) -> Vec<String> {
    let lyrics = match Lyrics::from_str(contents) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("Failed to parse LRC file {}: {}", path.display(), e);
            return Vec::new();
        }
    };

    lyrics
        .get_timed_lines()
        .iter()
        .map(|(_, text)| text.to_string())
        .filter(|text| !text.trim().is_empty())
        .collect()
}

#[cfg(test)]
mod test {
    use whichlang::{Lang, detect_language};

    #[test]
    fn test() {
        let texts = vec!["碧蓝档案", "ブルーアーカイブ"];
        let mut chinese_count = 0;
        let mut japanese_count = 0;

        for text in texts {
            match detect_language(&text) {
                Lang::Cmn => chinese_count += 1,
                Lang::Jpn => japanese_count += 1,
                _ => {}
            }
        }
        assert_eq!(chinese_count, 1);
        assert_eq!(japanese_count, 1);
    }
}
