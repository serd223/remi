#![allow(dead_code)]
use std::str::FromStr;

#[derive(Debug)]
pub struct Gemtext {
    pub data: Vec<GemtextEntry>,
}

#[derive(Debug)]
pub struct GemtextParseError {
    pub line: String,
    pub msg: String,
}

#[allow(dead_code)]
impl GemtextParseError {
    pub fn new(line: &str, msg: &str) -> Self {
        Self {
            line: line.to_string(),
            msg: msg.to_string(),
        }
    }
}

impl std::fmt::Display for GemtextParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "'{}' in line:\n    {}", self.msg, self.line)
    }
}

impl std::error::Error for GemtextParseError {}

impl FromStr for Gemtext {
    type Err = GemtextParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut res: Vec<GemtextEntry> = vec![];
        let mut preformatted_mode = false;
        let mut preformatted_buffer = String::new();
        let mut preformatted_alt_text = String::new();
        for l in s.lines() {
            let l1 = l.trim_start();
            if preformatted_mode {
                if l1.starts_with("```") {
                    preformatted_mode = false;
                    res.push(GemtextEntry::Preformatted {
                        alt_text: preformatted_alt_text.clone(),
                        body: preformatted_buffer.clone(),
                    });
                    preformatted_alt_text.clear();
                    preformatted_buffer.clear();
                } else {
                    if preformatted_buffer.len() > 0 {
                        preformatted_buffer.push('\n');
                    }
                    preformatted_buffer.push_str(l);
                }
            } else {
                if l1.starts_with("=>") {
                    let mut byte_counter = 0;
                    let mut word_start = 0;
                    let mut word_counter = 0;
                    let mut url = String::new();
                    let mut label = String::new();
                    for c in l1.chars() {
                        if c.is_whitespace() {
                            if word_counter == 1 {
                                // url
                                url = l1[word_start..byte_counter].to_string();
                            } else if word_counter > 1 {
                                // rest is label
                                label = l1[word_start..].to_string();
                                break;
                            }
                            word_counter += 1;
                            byte_counter += c.len_utf8();
                            word_start = byte_counter;
                        } else {
                            byte_counter += c.len_utf8();
                        }
                    }
                    if url.is_empty() && word_counter == 1 && l1.len() > word_start {
                        url = l1[word_start..].to_string();
                    }
                    res.push(GemtextEntry::Link { url, label });
                } else if l1.starts_with("### ") {
                    res.push(GemtextEntry::MinorHeading(if l1.len() > 4 {
                        l1[4..].to_string()
                    } else {
                        String::new()
                    }));
                } else if l1.starts_with("## ") {
                    res.push(GemtextEntry::MediumHeading(if l1.len() > 3 {
                        l1[3..].to_string()
                    } else {
                        String::new()
                    }));
                } else if l1.starts_with("# ") {
                    res.push(GemtextEntry::MajorHeading(if l1.len() > 2 {
                        l1[2..].to_string()
                    } else {
                        String::new()
                    }));
                } else if l1.starts_with("* ") {
                    let new_entry = if l1.len() > 2 {
                        l1[2..].to_string()
                    } else {
                        String::new()
                    };
                    let mut new_list = true;
                    if let Some(e) = res.last_mut() {
                        match e {
                            GemtextEntry::List(vec) => {
                                new_list = false;
                                vec.push(new_entry.clone());
                            }
                            _ => (),
                        }
                    }
                    if new_list {
                        res.push(GemtextEntry::List(vec![new_entry]));
                    }
                } else if l1.starts_with(">") {
                    res.push(GemtextEntry::Quote(if l1.len() > 1 {
                        l1[1..].to_string()
                    } else {
                        String::new()
                    }));
                } else if l1.starts_with("```") {
                    preformatted_mode = true;
                    if l1.len() > 3 {
                        preformatted_alt_text.push_str(&s[3..]);
                    }
                } else {
                    res.push(GemtextEntry::Text(l.to_string()));
                }
            }
        }
        Ok(Gemtext { data: res })
    }
}

#[derive(Debug)]
pub enum GemtextEntry {
    Text(String),
    Link { url: String, label: String },
    MinorHeading(String),
    MediumHeading(String),
    MajorHeading(String),
    List(Vec<String>),
    Quote(String),
    Preformatted { alt_text: String, body: String },
}
