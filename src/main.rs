extern crate tera;
extern crate pulldown_cmark;
extern crate regex;
extern crate walkdir;

use pulldown_cmark::{Parser, html};
use regex::Regex;
use tera::{Context, Tera};
use walkdir::WalkDir;

use std::{env, fs};
use std::path::Path;
use std::io::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let dir_static = "./static";
    let dir_public = "./public";
    let dir_content = "./content";

    let ps0 = Path::new(dir_static);
    let pp0 = Path::new(dir_public);
    let pc0 = Path::new(dir_content);

    let tera = match Tera::new("theme/*.html") {
        Ok(t) => t,
        Err(e) => {
            println!("Parsing error(s): {}", e);
            panic!();
        }
    };


    // static files
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


    // markdown files
    for entry in WalkDir::new(dir_content)
            .into_iter()
            .filter_map(Result::ok) {
        let path0 = entry.path();
        if path0 == pc0 { continue; }
        let path = path0.strip_prefix(dir_content)?;
        if entry.file_type().is_dir() {
            println!("d:d: {} ", path.display());
            fs::create_dir_all(pp0.join(path))?;
            continue;
        }
        let fname = entry.file_name();
        if !fname.to_string_lossy().ends_with(".md") {
            continue;
        }

        // @TODO read from config
        let mut globals = Context::new();
        globals.insert("Site_BaseUrl", "https://f5n.org");
        globals.insert("Site_Title", "f5n dot org");
        globals.insert("Site_Params_defaultDescription", "a website");

        let mut tpl = "page.html";
        globals.insert("Template", tpl);

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
        let args : Vec<&str> = parts[1].split("\n").collect();
        for line in args {
            let pos : usize = match line.find('=') {
                Some(x) => x,
                None => 0,
            };
            if pos < 1 {
                continue;
            }
            let mut arg = String::from(&line[0..1]);
            arg = arg.to_uppercase();
            arg.push_str(&line[1..pos].trim());
            let val = &line[(pos+1)..].trim();
            if arg == "Draft" && val == &"true" {
                continue;
            }
            if arg == "Template" {
                tpl = val;
            }
            globals.insert(arg, val);
        }

        let parser = Parser::new(parts[2]);
        let mut html = String::new();
        html::push_html(&mut html, parser);
        globals.insert("content", &html);

        let rv = tera.render(tpl, &globals)?;

        let p1 = pp0.join(path).with_extension("html");
        println!("d:f: {}",p1.strip_prefix(pp0).unwrap().display());
        let mut ofile = fs::File::create(p1)?;
        ofile.write_all(&rv.trim().as_bytes())?;
    }
    Ok(())
}
