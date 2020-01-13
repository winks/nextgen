extern crate pulldown_cmark;
extern crate regex;
extern crate walkdir;

use pulldown_cmark::{Parser, html};
use regex::Regex;
use walkdir::WalkDir;

use std::{env, fs};
use std::path::Path;
use std::io::prelude::*;
use std::collections::HashMap;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let dir_static = "./static";
    let dir_public = "./public";
    let dir_theme  = "./theme";
    let dir_content = "./content";

    let mut templates = HashMap::new();
    let mut partials  = HashMap::new();
    let mut blocks    = HashMap::new();

    let mut vars      = HashMap::new();
    let mut vars_root = HashMap::new();
    vars_root.insert("TestVar".to_string(), "test_var".to_string());

    let mut vars_site = HashMap::new();
    vars_site.insert("BaseUrl".to_string(), "https://f5n.org".to_string());
    vars.insert("Site".to_string(), vars_site);

    let mut vars_site_params = HashMap::new();
    vars_site_params.insert("Foo".to_string(), "foo".to_string());
    //vars_site_params.insert("highlight".to_string(), "foo");

    let ps0 = Path::new(dir_static);
    let pp0 = Path::new(dir_public);
    let pt0 = Path::new(dir_theme);

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

    // theme files
    for entry in WalkDir::new(dir_theme)
            .into_iter()
            .filter_map(Result::ok) {
        let path0 = entry.path();
        if path0 == pt0 { continue; }
        let path = path0.strip_prefix(dir_theme)?;
        let fname = entry.file_name();

        if entry.file_type().is_dir() {
            continue;
        }
        println!("t:f: {} {}", path.display(), fname.to_string_lossy().to_string());
        let mut file = fs::File::open(path0)?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;
        if entry.path().to_string_lossy().starts_with("./theme/partials/") {
            partials.insert(fname.to_string_lossy().to_string(), contents);
        } else if fname == "base.html" {
            templates.insert("base.html".to_string(), contents);
        }
    }

    let re_partial = Regex::new(r#"\{\{[\s*]?partial "([\w\.]+)" \.[\s*]?\}\}"#).unwrap();
    let re_block   = Regex::new(r#"\{\{[\s*]?block "([\w\.]+)" \.[\s*]?\}\}[\s*]?\{\{[\s*]?end[\s*]?\}\}"#).unwrap();
    let re_with    = Regex::new(r#"\{\{[\s*]?with \.([\w\.]+)[\s*]?\}\}((?sU).*)\{\{[\s*]?end[\s*]?\}\}"#).unwrap();
    let re_isset   = Regex::new(r#"\{\{[\s*]?if[\s*]?isset \.([\w\.]+)[\s*]?"([\w\.]+)"[\s*]?\}\}((?sU).*)\{\{[\s*]?end[\s*]?\}\}"#).unwrap();
    let re_eqne    = Regex::new(r#"\{\{[\s*]?if[\s*]?(eq|ne) \.([\w\.]+)\s[\s*]?"([\w\.]+)"[\s*]?\}\}((?sU).*)\{\{[\s*]?end[\s*]?\}\}"#).unwrap();
    //let t = "{{ partial \"foo.html\" .}}  ";
    //println!("{:?} {}", re_partial.is_match(t), t);

    // markdown files
    for entry in WalkDir::new(dir_content)
            .into_iter()
            .filter_map(Result::ok) {
        let path0 = entry.path();
        let path = path0.strip_prefix(dir_content)?;
        if entry.file_type().is_dir() {
            //println!("s:d: {} ", path.display());
            fs::create_dir_all(pp0.join(path))?;
            continue;
        }
        let fname = entry.file_name();
        if !fname.to_string_lossy().ends_with(".md") {
            continue;
        }

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
            vars_root.insert(arg, val.to_string());
        }

        let parser = Parser::new(parts[2]);
        let mut html = String::new();
        html::push_html(&mut html, parser);
        blocks.insert("main".to_string(), html);

        let rv = &templates["base.html"].to_string();
        //println!("{:?}", re_partial.is_match(&rv));
        let rv2 = re_partial.replace_all(rv, |caps: &regex::Captures| {
            let m = &caps[1];
            &partials[m]
        }).into_owned();
        //println!("{:?}", re_block.is_match(&rv));
        let rv3 = re_block.replace_all(&rv2, |caps: &regex::Captures| {
            let m = &caps[1];
            &blocks[m]
        }).into_owned();
        let rv4 = re_with.replace_all(&rv3, |caps: &regex::Captures| {
            let m = &caps[1];
            let p : Vec<&str> = m.split('.').collect();
            if !vars.contains_key(p[0]) {
                "".to_string()
            } else if p.len() == 2 {
                if !vars[p[0]].contains_key(p[1]) {
                  "".to_string()
                } else {
                  let x = &caps[2];
                  x.replace("{{ . }}", &vars[p[0]][p[1]])
                }
            } else if p.len() == 3 {
                if p[0] == "Site" && p[1] == "Params" && vars_site_params.contains_key(p[2]) {
                  let x = &caps[2];
                  x.replace("{{ . }}", &vars_site_params[p[2]])
                } else {
                  "".to_string()
                }
            } else {
                println!("error {:?}", p);
                "".to_string()
            }
        }).into_owned();
        let rv5 = re_isset.replace_all(&rv4, |caps: &regex::Captures| {
            let m = &caps[1];
            let mut p : Vec<&str> = m.split('.').collect();
            let n = &caps[2];
            p.push(n);
            if !vars.contains_key(p[0]) {
                "".to_string()
            } else if p.len() == 2 {
                if !vars[p[0]].contains_key(p[1]) {
                  "".to_string()
                } else {
                  let x = &caps[3];
                  x.replace("{{ . }}", &vars[p[0]][p[1]])
                }
            } else if p.len() == 3 {
                if p[0] == "Site" && p[1] == "Params" && vars_site_params.contains_key(p[2]) {
                  let x = &caps[3];
                  x.replace("{{ . }}", &vars_site_params[p[2]])
                } else {
                  "".to_string()
                }
            } else {
                println!("error {:?}", p);
                "".to_string()
            }
        }).into_owned();
        println!("{:?}", re_eqne.is_match(&rv5));
        let rv6 = re_eqne.replace_all(&rv4, |caps: &regex::Captures| {
            let m = &caps[1];
            let mut p : Vec<&str> = m.split('.').collect();
            let n = &caps[2];
            p.push(n);
        println!("{:?}", caps);
            if !vars.contains_key(p[0]) {
                "".to_string()
            } else if p.len() == 2 {
                if !vars[p[0]].contains_key(p[1]) {
                  "".to_string()
                } else {
                  let x = &caps[3];
                  x.replace("{{ . }}", &vars[p[0]][p[1]])
                }
            } else if p.len() == 3 {
                if p[0] == "Site" && p[1] == "Params" && vars_site_params.contains_key(p[2]) {
                  let x = &caps[3];
                  x.replace("{{ . }}", &vars_site_params[p[2]])
                } else {
                  "".to_string()
                }
            } else {
                println!("error {:?}", p);
                "".to_string()
            }
        }).into_owned();

        let mut ofile = fs::File::create(pp0.join(path))?;
        ofile.write_all(&rv5.trim().as_bytes())?;
    }
    Ok(())
}
