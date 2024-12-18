use poise::serenity_prelude::{Context, CreateMessage, Message, MESSAGE_CODE_LIMIT};
use regex::{Captures, Regex};
use reqwest::get;
use std::collections::{HashMap, HashSet};
use std::option::Option;

static COMMENT_TEMPLATES: HashMap<&'static str, &'static str> = HashMap::from([
    ("c", "// {}"),
    ("cpp", "// {}"),
    ("cs", "// {}"),
    ("java", "// {}"),
    ("js", "// {}"),
    ("go", "// {}"),
    ("kt", "// {}"),
    ("swift", "// {}"),
    ("rs", "// {}"),
    ("scala", "// {}"),
    ("py", "# {}"),
    ("sh", "# {}"),
    ("pl", "# {}"),
    ("rb", "# {}"),
    ("r", "# {}"),
    ("ps1", "# {}"),
    ("php", "// {}"),
    ("sql", "-- {}"),
    ("html", "<!-- {} -->"),
    ("xml", "<!-- {} -->"),
    ("css", "/* {} */"),
    ("lisp", "; {}"),
    ("scm", "; {}"),
    ("hs", "-- {}"),
    ("m", "% {}"),
    ("asm", "; {}"),
    ("pro", "% {}"),
    ("vim", "\" {}"),
    ("ini", "; {}"),
    ("jl", "# {}"),
    ("erl", "% {}"),
    ("ex", "# {}"),
    ("lua", "-- {}"),
    ("tcl", "# {}"),
    ("yml", "# {}"),
    ("md", "[comment]: # ({})"),
    ("lhs", "-- {}"),
]);

pub enum RangeOrIndex {
    Language(String),
    Index(String, i32),
    Range(String, i32, i32),
}

fn parse_url(url: &str) -> Option<RangeOrIndex> {
    let extension_regex = Regex::new(r"\.([^./?#]+)(#|$)").unwrap();

    let range_regex =
        Regex::new(r"(?:\.(?<language>[^#]+))?(?:#L(?<start>\d+)?(?:-L(?<end>\d+))?)?$").unwrap();

    let language = extension_regex
        .captures(url)
        .and_then(|caps| caps.get(1))
        .map(|m| m.as_str().to_string())
        .unwrap_or_default();

    if let Some(caps) = range_regex.captures(url) {
        let start = caps
            .name("start")
            .and_then(|m| m.as_str().parse::<i32>().ok());
        let end = caps
            .name("end")
            .and_then(|m| m.as_str().parse::<i32>().ok());

        if end < start {
            return None;
        }

        match (start, end) {
            (Some(start), Some(end)) => Some(RangeOrIndex::Range(language, start - 1, end)),
            (Some(start), None) => Some(RangeOrIndex::Index(language, start - 1)),
            (None, None) => Some(RangeOrIndex::Language(language)),
            _ => None,
        }
    } else {
        None
    }
}

fn trim_message(lang: String, content: String) -> String {
    if content.len() > MESSAGE_CODE_LIMIT {
        content[0..MESSAGE_CODE_LIMIT - 200].to_string()
            + &*format!(
                "\n{}",
                COMMENT_TEMPLATES
                    .get(&*lang)
                    .unwrap_or(&"// {}")
                    .replace("{}", "El mensaje fue cortado por limite de caracteres.")
            )
    } else {
        content
    }
}

async fn read_message(link: String) -> Option<String> {
    if let Ok(result) = get(&link).await {
        if result.status() == 200 {
            if let Ok(text) = result.text().await {
                let parsed = parse_url(&link)?;

                let subtext: Vec<&str> = text.split('\n').collect();

                return match parsed {
                    RangeOrIndex::Language(language) => Some(format!(
                        "Mostrando <{link}>\n```{}\n{}\n```",
                        language.to_owned(),
                        trim_message(language, text)
                    )),
                    RangeOrIndex::Index(language, index) => {
                        if index < subtext.len() as i32 {
                            Some(format!(
                                "Mostrando linea {} de <{link}>\n```{}\n{}\n```",
                                index + 1,
                                language.to_owned(),
                                trim_message(language, subtext[index as usize].to_string())
                            ))
                        } else {
                            None
                        }
                    }
                    RangeOrIndex::Range(language, start, end) => {
                        if start < subtext.len() as i32 && end <= subtext.len() as i32 {
                            Some(format!(
                                "Mostrando desde la linea {} hasta la linea {end} de <{link}>\n```{}\n{}\n```",
                                start + 1,
                                language.to_owned(),
                                trim_message(language, subtext[start as usize..end as usize].join("\n"))
                            ))
                        } else {
                            None
                        }
                    }
                };
            }
        }
    }
    None
}

pub async fn message(ctx: &Context, msg: &Message) -> bool {
    if msg.author.bot || msg.content.starts_with("noembed") {
        return false;
    }

    let repo_regex = Regex::new("(https://github\\.com/(?:[^/]+/){2})blob/(.*)").unwrap();
    let hidden_link_regex = Regex::new(r"[<>]").unwrap();

    let replaced = if repo_regex.is_match(&msg.content) {
        repo_regex.replace_all(&msg.content, |captures: &Captures| {
            captures[1].to_string() + &captures[2]
        })
    } else {
        return false;
    }
    .replace("https://github.com/", "https://raw.githubusercontent.com/");

    let without_hidden = hidden_link_regex.replace_all(&replaced, "");

    let without_spaces = without_hidden.split('\n');
    // let without_spaces = split_message_regex.split(&without_hidden);

    let links = without_spaces
        .filter(|s| !s.starts_with('!') && s.starts_with("https://raw.githubusercontent.com/"));

    let dup = HashSet::<&str>::from_iter(links);
    for link in dup {
        if let Some(content) = read_message(link.to_string()).await {
            if let Some(reference) = &msg.message_reference {
                msg.channel_id
                    .send_message(
                        &ctx,
                        CreateMessage::new()
                            .content(content)
                            .reference_message(reference.clone()),
                    )
                    .await
                    .unwrap();
            } else {
                msg.reply(&ctx, content).await.unwrap();
            }
        }
    }

    true
}
