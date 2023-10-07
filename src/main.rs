extern crate chrono;
extern crate pulldown_cmark;
extern crate serde;
extern crate tera;
extern crate walkdir;

use chrono::{DateTime, SecondsFormat};
use pulldown_cmark::{Parser, html};
use serde::{Deserialize, Serialize};
use tera::{Context, Tera};
use toml::value;
use walkdir::WalkDir;

use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::time::SystemTime;
use std::io::prelude::*;

#[derive(Deserialize, Serialize)]
struct SiteConfig {
    baseurl: String,
    title: String,
    rsslink: String,
    blueprint: String,
    author: SiteAuthor,
    verbose: Option<bool>,
}

#[derive(Deserialize, Serialize)]
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
    tags: Option<value::Array>,
}

#[derive(Serialize, Clone)]
struct ParsedPage {
    content: String,
    description: String,
    date: String,
    datefull: String,
    dateshort: String,
    year: String,
    link: String,
    readingtime: String,
    rsslink: String,
    section: String,
    section_index: bool,
    template: String,
    title: String,
    tags: Vec<String>,
}

impl ParsedPage {
    fn new() -> ParsedPage {
        ParsedPage {
            content: String::new(),
            description: String::new(),
            date: String::new(),
            datefull: String::new(),
            dateshort: String::new(),
            year: String::new(),
            link: String::new(),
            rsslink: String::new(),
            readingtime: String::new(),
            section: String::new(),
            section_index: false,
            template: String::new(),
            title: String::new(),
            tags: Vec::new(),
        }
    }
}

struct Stats {
    pages: u32,
    files: u32,
    templates: usize,
    sections: u32,
    rss: u32,
}

impl Stats {
    fn new() -> Stats {
        Stats {
            pages: 0,
            files: 0,
            templates: 0,
            sections: 0,
            rss: 0,
        }
    }
    fn print(&self) {
        println!("+-------------+-----+");
        println!("Pages         | {}", self.pages);
        println!("Section files | {}", self.sections);
        println!("RSS files     | {}", self.rss);
        println!("Static files  | {}", self.files);
        println!("Templates     | {}", self.templates);
        println!();
    }
}

fn write(tera: &Tera, tpl: &str, vars: Context, msg: &str, pf: &Path, pp0: &Path, verb: bool) {
    if verb {
        println!("{}: {:?} {}", msg, pp0, pf.strip_prefix(pp0).unwrap().display());
    }
    let rv = tera.render(tpl, &vars).unwrap();
    let mut ofile = fs::File::create(pf).unwrap();
    ofile.write_all(rv.trim().as_bytes()).unwrap();
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let time_start = SystemTime::now();
    let mut stats = Stats::new();

    // config file
    let mut config_contents = String::new();
    let config_file = fs::File::open("./nextgen.toml");
    match config_file {
        Err(_) => panic!("No nextgen.toml found."),
        Ok(mut x) => {
            x.read_to_string(&mut config_contents)?;
        },
    };
    if config_contents.is_empty() {
        panic!("Empty nextgen.toml found.")
    }
    let config : SiteConfig = toml::from_str(&config_contents).unwrap();
    let verbose = config.verbose.unwrap_or(false);

    // initialize stuff
    let dir_static  = "./static";
    let dir_content = "./content";
    let dir_public  = "./public";
    let dir_bprint  = config.blueprint.clone();
    let dir_bp_static = dir_bprint.to_owned() + "/static";

    let ps0 = Path::new(dir_static);
    let pc0 = Path::new(dir_content);
    let t0x = &(dir_bp_static.clone());
    let ps01 = Path::new(t0x);
    let pp0 = Path::new(dir_public);
    match fs::create_dir_all(&pp0) {
        Ok(x) => x,
        Err(e) => {
            println!("Could not create output directory {}: {}", pp0.display(), e)
        }
    };

    let tera = match Tera::new(&(dir_bprint.to_owned() + "/templates/**/*.html")) {
        Ok(t) => t,
        Err(e) => {
            println!("Parsing error(s): {}", e);
            panic!();
        }
    };
    let mut tera_macro = match Tera::new(&(dir_bprint.to_owned() + "/macros/**/*.html")) {
        Ok(t) => t,
        Err(e) => {
            println!("Parsing error(s): {}", e);
            panic!();
        }
    };
    stats.templates = tera.templates.len();

    // handle static files
    for entry in WalkDir::new(dir_static)
            .into_iter()
            .filter_map(|e| e.ok()) {
        let path0 = entry.path();
        if path0 == ps0 { continue; }
        let path = path0.strip_prefix(dir_static)?;

        if entry.file_type().is_dir() {
            if verbose {
                println!("s:d: {} ", path.display());
            }
            fs::create_dir_all(pp0.join(path))?;
        }
        if entry.file_type().is_file() {
            if verbose {
                println!("s:f: {} ", path.display());
            }
            fs::copy(path0, pp0.join(path))?;
            stats.files += 1;
        }
        // @TODO symlinks are ignored?
    }
    for entry in WalkDir::new(dir_bp_static.clone())
            .into_iter()
            .filter_map(|e| e.ok()) {
        let path01 = entry.path();
        if path01 == ps01 { continue; }
        let path = path01.strip_prefix(dir_bp_static.clone())?;

        if entry.file_type().is_dir() {
            if verbose {
                println!("s:d: {} ", path.display());
            }
            fs::create_dir_all(pp0.join(path))?;
        }
        if entry.file_type().is_file() {
            if verbose {
                println!("s:f: {} ", path.display());
            }
            fs::copy(path01, pp0.join(path))?;
            stats.files += 1;
        }
        // @TODO symlinks are ignored?
    }

    // figure out which sections we have
    let mut content_sections = HashMap::new();
    let sec_pages : Vec<ParsedPage> = Vec::new();
    content_sections.insert(String::from("_index"), sec_pages.clone());
    content_sections.insert(String::from("_default"), sec_pages.clone());
    content_sections.insert(String::from("_pages"), sec_pages.clone());
    for entry in WalkDir::new(dir_content)
            .into_iter()
            .filter_map(Result::ok) {
        let path0 = entry.path();
        if path0 == pc0 { continue; }
        if entry.file_type().is_dir() {
            let path = path0.strip_prefix(dir_content)?;
            if verbose {
                println!("d:d: {}", path.display());
            }
            fs::create_dir_all(pp0.join(path))?;
            let sec_name = path.parent().unwrap().to_str().unwrap();
            content_sections.insert(String::from(sec_name), sec_pages.clone());
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
        let mut page = ParsedPage::new();
        let mut page_vars = Context::new();
        page_vars.insert("Site", &config);

        //let metadata = entry.metadata()?;
        //let last_mod = metadata.modified()?.elapsed()?.as_secs();
        let mut file = fs::File::open(path0)?;
        file.read_to_string(&mut page.content)?;
        let parts : Vec<&str> = page.content.split("+++").collect();
        if parts.len() < 3 {
            continue;
        }

        // parse front matter
        let value_fm : FrontMatter = toml::from_str(parts[1]).unwrap();
        if value_fm.draft.unwrap_or(false) {
            continue;
        }
        let dtx = DateTime::parse_from_rfc3339(&value_fm.date.to_string()).unwrap();
        page.date = dtx.to_rfc3339_opts(SecondsFormat::Secs, true).to_string();
        page.datefull = dtx.format("%a %b %d %Y").to_string();
        page.dateshort = dtx.format("%Y-%m-%d").to_string();
        page.year = dtx.format("%Y").to_string();
        page.title = value_fm.title;
        page.description = value_fm.description.unwrap_or_default();
        page.template = "page.html".to_string();
        let tags = value_fm.tags.unwrap_or_default();
        if !tags.is_empty() {
            page.tags = tags.iter().map(|v| v.as_str().unwrap_or("").to_string()).collect();
        }

        // count words for reading time
        let wc : usize = (parts[2].split(' ').count() / 200) + 1;
        page.readingtime = wc.to_string();

        // convert to markdown
        let tpl_path_macro = path0.to_str().unwrap();
        let body_tmp = String::new() + "{% import \"macros.html\" as macros %}" + parts[2];
        tera_macro.add_raw_template(tpl_path_macro, &body_tmp).unwrap();
        let tmp_md = tera_macro.render(tpl_path_macro, &Context::new()).unwrap();
        let parser = Parser::new(&tmp_md);
        let mut html_from_md = String::new();
        html::push_html(&mut html_from_md, parser);
        page.content = html_from_md;

        // find out if a section template is needed
        for (sec, _) in content_sections.iter() {
            let mut sc = String::from(sec);
            sc.push('/');
            if path.starts_with(&sc) {
                sc.replace_range(sc.len()-1.., "_");
                if fname == "_index.md" {
                    sc.push_str("index.html");
                } else {
                    sc.push_str(&page.template);
                }
                page.template = sc;
                page.section = sec.to_string();
                break;
            }
        }

        let pf;
        let pp1 = pp0.join(path);
        let mut skip_write = false;
        if pp1.to_str().unwrap() == "./public/_index.md" {
            // special case for the /index.html
            pf = pp1.with_file_name("index.html");
            page.template = "index.html".to_string();
            page.section = "_index".to_string();
            skip_write = true;
        } else if path.to_str().unwrap().ends_with("/_index.md") {
            // use _index.md for a section's /section/index.html
            pf = pp1.with_file_name("index.html");
            page.template = page.section.clone();
            page.template.push_str("_index.html");
            skip_write = true;
            page.section_index = true;
            page.rsslink = value_fm.rsslink.unwrap_or_default();
            // println!("PR1 {} {}", page.title, page.rsslink);
        } else {
            let pd = pp1.with_extension("");
            pf = pd.join("index.html");
            fs::create_dir_all(pd)?;
            if page.section.is_empty() {
                page.section = "_default".to_string();
            }
        }

        page.link = pf.strip_prefix(pp0).unwrap().with_file_name("").to_str().unwrap().to_string();
        page_vars.insert("Page", &page);
        if !skip_write {
            write(&tera, &page.template, page_vars, "d:f", &pf, pp0, verbose);
            stats.pages += 1;
        }
        let psx = page.section.clone();
        content_sections.get_mut(&psx).unwrap().push(page.clone());
        content_sections.get_mut("_pages").unwrap().push(page);
    }

    for (sec, pp) in content_sections.iter_mut() {
        if pp.is_empty() || &sec[0..1] == "_" { continue; }
        (*pp).sort_by(|a, b| b.date.cmp(&a.date));
        // section index page
        let mut pi_tpl = String::new();
        let mut pi_vars = Context::new();
        // RSS/Atom
        let mut rss_vars = Context::new();
        let mut rss_date = String::new();
        let mut rss_link = config.baseurl.clone();
        let mut rss_title = config.title.clone();

        for p in pp.clone() {
            if p.section_index {
                pi_tpl = p.template.clone();
                pi_vars = Context::new();
                pi_vars.insert("Page", &p);
                pi_vars.insert("Site", &config);
                rss_title.push_str(" - ");
                rss_title.push_str(&p.title);
                if !p.rsslink.is_empty() {
                    rss_link.push_str(&p.rsslink);
                } else {
                    rss_link.push_str(&config.rsslink);
                }
                // println!("PR {} {} {}", p.title, p.rsslink, rss_link);
                continue;
            } else if rss_date.is_empty() {
                rss_date.push_str(&p.date);
            }
        }

        pi_vars.insert("entries", &pp.clone());
        pi_vars.insert("rsslink", &rss_link);
        if pi_tpl.is_empty() || pp0.join(pi_tpl.clone()).exists() {
            println!("Skipping index for '{}', no section template.", sec);
            continue;
        }
        write(&tera, &pi_tpl, pi_vars, "d:s", &pp0.join(sec).join("index.html"), pp0, verbose);
        stats.sections += 1;

        // RSS/Atom
        rss_vars.insert("Site", &config);
        rss_vars.insert("entries", &pp.clone());
        rss_vars.insert("rsslink", &rss_link);
        rss_vars.insert("Date", &rss_date);
        rss_vars.insert("Title", &rss_title);
        // println!("RR {}", rss_link);
        write(&tera, "rss_page.html", rss_vars, "d:r", &pp0.join(&rss_link[config.baseurl.len()+1..]), pp0, verbose);
        stats.rss += 1;
    }

    let mut index_vars = Context::new();
    index_vars.insert("Site", &config);

    if content_sections.contains_key("_index") {
        match &content_sections.get("_index") {
            None => {},
            Some(csi) => {
                if !csi.is_empty() {
                    index_vars.insert("Page", &csi[0].clone());
                }
            }
        }
    }
    let pages = content_sections.get_mut("_pages").unwrap();
    pages.sort_by(|a, b| b.date.cmp(&a.date));

    index_vars.insert("entries", &pages.clone());
    index_vars.insert("rsslink", &config.rsslink);
    index_vars.insert("Title", &config.title);
    write(&tera, "index.html", index_vars, "d:i", &pp0.join("index.html"), pp0, verbose);

    let mut rss_index_vars = Context::new();
    rss_index_vars.insert("Site", &config);
    rss_index_vars.insert("entries", &pages.clone());
    rss_index_vars.insert("rsslink", &config.rsslink);
    rss_index_vars.insert("Title", &config.title);
    rss_index_vars.insert("Date", &pages[0].date);
    write(&tera, "rss_page.html", rss_index_vars, "d:r", &pp0.join(&config.rsslink[1..]), pp0, verbose);

    stats.print();
    match time_start.elapsed() {
        Ok(elapsed) => println!("Took {} msecs.", elapsed.as_millis()),
        Err(e) => println!("Err: {:?}", e),
    }

    Ok(())
}
