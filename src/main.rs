extern crate chrono;
extern crate pulldown_cmark;
extern crate serde;
extern crate tera;
extern crate walkdir;

use pulldown_cmark::{Parser, html};
use serde::Deserialize;
use tera::{Context, Tera};
use toml::value::Datetime;
use walkdir::WalkDir;

use std::fs;
use std::path::Path;
use std::io::prelude::*;

#[derive(Deserialize)]
struct SiteConfig {
    baseurl: String,
    title: String,
}

#[derive(Deserialize)]
struct FrontMatter {
    date: Datetime,
    description: Option<String>,
    draft: Option<bool>,
    title: String,
    template: Option<String>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // @TODO RSS
    // @TODO sass and or other stuff to preprocess

    // config file
    let mut config_contents = String::new();
    let config_file = fs::File::open("./config.toml");
    match config_file {
        Err(_) => panic!("No config.toml found."),
        Ok(mut x) => {
            x.read_to_string(&mut config_contents)?;
        },
    };
    if config_contents.len() < 1 {
        panic!("No config.toml found.")
    }
    let config : SiteConfig = toml::from_str(&config_contents).unwrap();

    // initialize stuff
    let dir_static  = "./static";
    let dir_public  = "./public";
    let dir_content = "./content";

    let ps0 = Path::new(dir_static);
    let pp0 = Path::new(dir_public);
    let pc0 = Path::new(dir_content);

    let tera = match Tera::new("theme/**/*.html") {
        Ok(t) => t,
        Err(e) => {
            println!("Parsing error(s): {}", e);
            panic!();
        }
    };

    // handle static files
    for entry in WalkDir::new(dir_static)
            .into_iter()
            .filter_map(|e| e.ok()) {
        let path0 = entry.path();
        if path0 == ps0 { continue; }
        let path = path0.strip_prefix(dir_static)?;

        if entry.file_type().is_dir() {
            println!("s:d: {} ", path.display());
            fs::create_dir_all(pp0.join(path))?;
        }
        if entry.file_type().is_file() {
            println!("s:f: {} ", path.display());
            fs::copy(path0, pp0.join(path))?;
        }
        // @TODO symlinks are ignored?
    }

    // figure out which sections we have
    let mut content_sections = vec![];
    for entry in WalkDir::new(dir_content)
            .into_iter()
            .filter_map(Result::ok) {
        let path0 = entry.path();
        if path0 == pc0 { continue; }
        if entry.file_type().is_dir() {
            let path = path0.strip_prefix(dir_content)?;
            println!("d:d: {}", path.display());
            fs::create_dir_all(pp0.join(path))?;
            let x = path.to_str().unwrap();
            content_sections.push(String::from(x));
        }
    }

    // handle markdown files
    for entry in WalkDir::new(dir_content)
            .into_iter()
            .filter_map(Result::ok) {
        let path0 = entry.path();
        if path0 == pc0 { continue; }
        if entry.file_type().is_dir() {
            continue;
        }
        let fname = entry.file_name().to_str().unwrap();
        if !fname.ends_with(".md") {
            continue;
        }
        let path = path0.strip_prefix(dir_content)?;

        // initialize every time to avoid cached vars
        let mut page_vars = Context::new();
        page_vars.insert("Site_BaseUrl", &config.baseurl);
        page_vars.insert("Site_Title", &config.title);

        let mut page_section = String::new();
        let mut page_tpl = "page.html".to_string();
        page_vars.insert("Template", &page_tpl);

        //let metadata = entry.metadata()?;
        //let last_mod = metadata.modified()?.elapsed()?.as_secs();
        let mut file = fs::File::open(path0)?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;
        let parts : Vec<&str> = contents.split("+++").collect();
        if parts.len() < 3 {
            continue;
        }

        // parse front matter
        let value : FrontMatter = toml::from_str(parts[1]).unwrap();
        let is_draft: bool = value.draft.unwrap_or(false);
        if is_draft {
            continue;
        }
        let dtx = chrono::DateTime::parse_from_rfc3339(&value.date.to_string()).unwrap();
        page_vars.insert("Date", &dtx.format("%a %b %d %Y").to_string());
        match value.description {
            None => (),
            Some(x) => page_vars.insert("Description", &x),
        }
        match value.template {
            None => (),
            Some(x) => page_tpl = x,
        };
        page_vars.insert("Title", &value.title);

        // count words for reading time
        let words : Vec<&str> = parts[2].split(" ").collect();
        let wc : usize = (words.len() / 200) + 1;
        page_vars.insert("ReadingTime", &wc);

        // convert to markdown
        let parser = Parser::new(parts[2]);
        let mut html = String::new();
        html::push_html(&mut html, parser);
        page_vars.insert("content", &html);

        // find out if a section template is needed
        for sec in &content_sections {
            let mut sc = String::from(sec);
            sc.push_str("/");
            if path.starts_with(&sc) {
                sc.replace_range(sc.len()-1.., "_");
                if fname == "_index.md" {
                    sc.push_str("index.html");
                } else {
                    sc.push_str(&page_tpl);
                }
                page_tpl = sc;
                page_vars.insert("Section", &sec);
                page_section = sec.to_string();
                break;
            }
        }

        let pf;
        let pp1 = pp0.join(path);
        if pp1.to_str().unwrap() == "./public/_index.md" {
            // special case for the /index.html
            pf = pp1.with_file_name("index.html");
            page_tpl = "index.html".to_string();
        } else if path.to_str().unwrap().ends_with("/_index.md") {
            // use _index.md for a section's /section/index.html
            pf = pp1.with_file_name("index.html");
            page_tpl = page_section;
            page_tpl.push_str("_index.html");
        } else {
            let pd = pp1.with_extension("");
            pf = pd.join("index.html");
            fs::create_dir_all(pd)?;
        }
        println!("d:f: {}", pf.strip_prefix(pp0).unwrap().display());
        let rv = tera.render(&page_tpl, &page_vars)?;
        let mut ofile = fs::File::create(pf)?;
        ofile.write_all(&rv.trim().as_bytes())?;
    }
    Ok(())
}
