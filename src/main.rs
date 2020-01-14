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
    // @TODO DateFormat
    // @TODO RSS
    // @TODO Section pages + template
    // @TODO index
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

    let default_site_baseurl = &config.baseurl;
    let default_site_title = &config.title;

    // initialize stuff
    let dir_static = "./static";
    let dir_public = "./public";
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
    // @TODO sass
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
            println!("d:d: {} ", path.display());
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
        let path = path0.strip_prefix(dir_content)?;
        if entry.file_type().is_dir() {
            continue;
        }
        let fname = entry.file_name().to_str().unwrap();
        if !fname.ends_with(".md") {
            continue;
        }

        // initialize new to avoid cached stuff
        let mut globals = Context::new();
        globals.insert("Site_BaseUrl", default_site_baseurl);
        globals.insert("Site_Title", default_site_title);

        let mut tpl = "page.html".to_string();
        globals.insert("Template", &tpl);

        //let metadata = entry.metadata()?;
        //let last_mod = metadata.modified()?.elapsed()?.as_secs();
        //println!(": {} {:?}", path.display(), fname);
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
        globals.insert("Date", &value.date);
        match value.description {
            None => (),
            Some(x) => globals.insert("Description", &x),
        }
        globals.insert("Title", &value.title);
        match value.template {
            None => (),
            Some(x) => tpl = x,
        };

        // count words for reading time
        let words : Vec<&str> = parts[2].split(" ").collect();
        let wc : usize = (words.len() / 200) + 1;
        globals.insert("ReadingTime", &wc);

        // convert to markdown
        let parser = Parser::new(parts[2]);
        let mut html = String::new();
        html::push_html(&mut html, parser);
        globals.insert("content", &html);

        // find out if a section template is needed
        for sec in &content_sections {
            let mut sc = String::from(sec);
            sc.push_str("/");
            if path.starts_with(&sc) {
                sc.replace_range(sc.len()-1.., "_");
                sc.push_str(&tpl);
                tpl = sc;
                globals.insert("Section", &sec);
                break;
            }
        }

        let rv = tera.render(&tpl, &globals)?;
        let pd = pp0.join(path).with_extension("");
        let pf = pd.join("index.html");
        println!("d:f: {}",pf.strip_prefix(pp0).unwrap().display());
        fs::create_dir_all(pd)?;
        let mut ofile = fs::File::create(pf)?;
        ofile.write_all(&rv.trim().as_bytes())?;
    }
    Ok(())
}
