extern crate chrono;
extern crate pulldown_cmark;
extern crate serde;
extern crate tera;
extern crate walkdir;

use chrono::DateTime;
use pulldown_cmark::{Parser, html};
use serde::{Deserialize, Serialize};
use tera::{Context, Tera, Value};
use toml::value;
use walkdir::WalkDir;

use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::io::prelude::*;

#[derive(Deserialize)]
struct SiteConfig {
    baseurl: String,
    title: String,
    rsslink: String,
    author: SiteAuthor,
}

#[derive(Deserialize)]
struct SiteAuthor {
    name: String,
}

#[derive(Deserialize)]
struct FrontMatter {
    date: value::Datetime,
    description: Option<String>,
    draft: Option<bool>,
    title: String,
    rsslink: Option<String>,
    template: Option<String>,
}

#[derive(Serialize, Clone)]
struct ParsedPage {
    content: String,
    date: String,
    link: String,
    section: String,
    section_index: bool,
    template: String,
    title: String,
    vars: Value,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
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

    let mut parsed = vec![];

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
    let mut content_sections = HashMap::new();
    for entry in WalkDir::new(dir_content)
            .into_iter()
            .filter_map(Result::ok) {
        let path0 = entry.path();
        if path0 == pc0 { continue; }
        if entry.file_type().is_dir() {
            let path = path0.strip_prefix(dir_content)?;
            println!("d:d: {}", path.display());
            fs::create_dir_all(pp0.join(path))?;
            let sec_name = path.to_str().unwrap();
            let sec_pages : Vec<ParsedPage> = Vec::new();
            content_sections.insert(String::from(sec_name), sec_pages);
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
        let dtx = DateTime::parse_from_rfc3339(&value.date.to_string()).unwrap();
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
        let mut html_from_md = String::new();
        html::push_html(&mut html_from_md, parser);
        page_vars.insert("content", &html_from_md);

        // find out if a section template is needed
        for (sec, _) in content_sections.iter() {
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
        let mut skip_write = false;
        let mut section_index = false;
        if pp1.to_str().unwrap() == "./public/_index.md" {
            // special case for the /index.html
            pf = pp1.with_file_name("index.html");
            page_tpl = "index.html".to_string();
            page_section = "indexindex".to_string();
        } else if path.to_str().unwrap().ends_with("/_index.md") {
            // use _index.md for a section's /section/index.html
            pf = pp1.with_file_name("index.html");
            page_tpl = page_section.clone();
            page_tpl.push_str("_index.html");
            skip_write = true;
            section_index = true;
        } else {
            let pd = pp1.with_extension("");
            pf = pd.join("index.html");
            fs::create_dir_all(pd)?;
        }
        println!("d:f: {} {}", pf.strip_prefix(pp0).unwrap().display(), !skip_write);
        if !skip_write {
            let rv = tera.render(&page_tpl, &page_vars)?;
            let mut ofile = fs::File::create(pf.clone())?;
            ofile.write_all(&rv.trim().as_bytes())?;
        }

        let parsed_page = ParsedPage {
            title: value.title,
            date: value.date.to_string(),
            link: pf.strip_prefix(pp0).unwrap().with_file_name("").to_str().unwrap().to_string(),
            content: html_from_md,
            section: page_section,
            template: page_tpl,
            vars: page_vars.into_json(),
            section_index: section_index,
        };
        parsed.push(parsed_page);
    }

    for p in parsed {
        if p.section == "indexindex" {
            // @TODO templating
            //println!("i {} {} {}", p.date, p.title, p.link);
        } else if p.section.len() > 0 {
            //if p.section_index { continue; }
            //println!("{} _{}_ {} {}", p.date, p.title, p.link, p.template);
            content_sections.get_mut(&p.section).unwrap().push(p);
        } else {
            //println!("  {} {} {}", p.date, p.section, p.title);
        }
    }

    let mut prev_year = "0".to_string();
    for (sec, pp) in content_sections.iter_mut() {
        if pp.len() < 1 { continue; }
        let mut out = String::new();
        (*pp).sort_by(|a, b| b.date.cmp(&a.date));
        let mut idx = 0;
        let mut pi_tpl = String::new();
        let mut pi_vars = Context::new();
        // RSS/Atom
        let mut rss_vars = Context::new();
        rss_vars.insert("Site_BaseUrl", &config.baseurl);
        rss_vars.insert("Site_Title", &config.title);
        rss_vars.insert("Site_Author_Name", &config.author.name);
        rss_vars.insert("Title", &config.title);
        rss_vars.insert("content", "foo");
        let mut rss_date = String::new();
        let mut rss_link = config.baseurl.clone();
        rss_link.push_str("/");
        rss_link.push_str(sec);
        rss_link.push_str("/atom.xml");
        rss_vars.insert("rsslink", &rss_link);

        for p in pp.clone() {
            if p.section_index {
                pi_tpl = p.template.clone();
                //pi_vars = p.vars.clone();
                pi_vars = Context::from_value(p.vars.clone()).unwrap();
                continue;
            }
            if idx > 0 {
                out.push_str("</ul>\n");
            } else {
                rss_date.push_str(&p.date);
            }
            let yr = String::from(&p.date[0..4]);
            if yr != prev_year {
                out.push_str("\n<h3>");
                out.push_str(&yr);
                out.push_str("</h3>\n");
                out.push_str("<ul class=\"posts\">\n");
            }
            let dtx = DateTime::parse_from_rfc3339(&p.date.to_string()).unwrap();
            out.push_str(" <li>\n  <time class=\"pull-right post-list\">");
            out.push_str(&dtx.format("%Y-%m-%d").to_string());
            out.push_str("</time>\n  <span><a href=\"");
            out.push_str(&config.baseurl);
            out.push_str("/");
            out.push_str(&p.link);
            out.push_str("\">");
            out.push_str(&p.title);
            out.push_str("</a></span>\n </li>\n");
            prev_year = yr;
            idx += 1;
        }
        out.push_str("</ul>");

        pi_vars.insert("content", &out);
        if pi_tpl.len() < 1 || pp0.join(pi_tpl.clone()).exists() {
            println!("Skipping {}, no section template.", sec);
            continue;
        }
        let rv = tera.render(&pi_tpl, &pi_vars)?;
        let pf = pp0.join(sec).join("index.html");
        let mut ofile = fs::File::create(pf.clone())?;
        ofile.write_all(&rv.trim().as_bytes())?;
        println!("d:f: {} i", pf.strip_prefix(pp0).unwrap().display());

        // RSS/Atom
        rss_vars.insert("entries", &pp.clone());
        rss_vars.insert("Date", &rss_date);
        let rss_tpl = "rss_page.html";
        let rv = tera.render(&rss_tpl, &rss_vars)?;
        // @TODO use rsslink
        let pf = pp0.join(sec).join("atom.xml");
        let mut ofile = fs::File::create(pf.clone())?;
        ofile.write_all(&rv.trim().as_bytes())?;
        println!("d:r: {}", pf.strip_prefix(pp0).unwrap().display());
    }

    Ok(())
}
