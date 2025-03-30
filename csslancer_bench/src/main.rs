use std::{collections::HashSet, fs::{copy, DirEntry, ReadDir}, path::{Path, PathBuf}, str};

use regex::Regex;


fn main() {
    println!("csslancer_bench::main()");

    update_parsers();
}

fn rmdir(dir: &str) {
    std::process::Command::new("rm")
        .args(["-r", dir])
        .output()
        .expect(&format!("failed to remove {dir} repo"));
}

fn cpdir(from_dir: &str, to_dir: &str) {
    std::process::Command::new("cp")
        .args(["-r", from_dir, to_dir])
        .output()
        .expect(&format!("failed copy {from_dir} to {to_dir}"))
        .status.success().then_some(())
        .expect(&format!("failed copy {from_dir} to {to_dir}"));
}



fn clone_repo(url: &str) {
    std::process::Command::new("git")
        .args(["clone", "--depth", "1" , url])
        .output()
        .expect(&format!("failed to clone repo {url}"));
}

fn exec_python_file(args: &[&str]) {
    let out = std::process::Command::new("python3")
        .args(args)
        .output()
        .expect(&format!("failed to execute python file `{}`", args[0]));
    if !out.status.success() {
        panic!("error executing file {}:\nSTDOUT:{}\nSTDERR:{}", 
            args[0],
            std::string::String::from_utf8_lossy(&out.stdout), 
            std::string::String::from_utf8_lossy(&out.stderr));
    }
}

const PATCHES: &'static [(&'static str, &'static str, &'static str)] = &[
    ("./blink/Source/build/scripts/in_generator.py", r#"print "USAGE: %s INPUT_FILES" % script_name"#, r#"print("USAGE: %s INPUT_FILES" % script_name)"#),
    ("./blink/Source/build/scripts/in_file.py", "print message", r#"print(message)"#),
    ("./blink/Source/build/scripts/in_generator.py", "basestring", "str"),
    ("./blink/Source/build/scripts/in_file.py", "args[arg_name].append(arg_value)", "args[arg_name] = args[arg_name] + arg_value"),
    ("./blink/Source/core/css/parser/CSSParser.cpp", r#"#include "core/layout/LayoutTheme.h""#, ""),
    ("./blink/Source/core/css/parser/CSSParser.cpp", "LayoutTheme::theme().systemColor(id)", "0xFFFFFFFF"),
    ("./blink/Source/core/css/parser/CSSParser.h", r#"#include "platform/graphics/Color.h"#, "namespace blink{typedef unsigned RGBA32;}")
];

fn update_parsers() {
    println!("Updating parsers");

    // rmdir("./blink");
    rmdir("./blink-css");
    rmdir("./depot_tools");
    rmdir("./chromium");

    // clone_repo("https://chromium.googlesource.com/chromium/blink");
    // clone_repo("https://chromium.googlesource.com/chromium/tools/depot_tools");
    // clone_repo("https://chromium.googlesource.com/chromium");

    cpdir("./blink/Source/core/css", "./blink-css");
    copy("./blink/Source/config.h", "./blink-css/config.h").unwrap();

    for patch in PATCHES {
        let prev = std::fs::read_to_string(Path::new(patch.0)).unwrap();
        std::fs::write(Path::new(patch.0), prev.replace(patch.1, patch.2)).unwrap();
    }

    exec_python_file(&["./blink/Source/build/scripts/make_css_property_names.py", "./blink/Source/core/css/CSSProperties.in"]);
    copy("./CSSPropertyNames.cpp", "./blink/Source/core/CSSPropertyNames.cpp").unwrap();
    copy("./CSSPropertyNames.h",   "./blink/Source/core/CSSPropertyNames.h").unwrap();
    std::fs::remove_file("./CSSPropertyNames.cpp").unwrap();
    std::fs::remove_file("./CSSPropertyNames.h").unwrap();

    let mut dir = std::fs::read_dir(Path::new("./blink/Source/core/css/parser")).unwrap();

    let parser_files = gather_files_readdir(dir);


    let mut deps = HashSet::new();
    for parser_file in parser_files.clone() {
        let file_deps = gather_deps(&parser_file);
        for file_dep in file_deps {
            deps.insert(file_dep);
        }
    }

    println!("DEPS COUNT = {}", deps.iter().count());
    for dep in deps {
        println!("DEP: {}", dep);
    }

    let mut trans_deps = HashSet::new();
    for parser_file in parser_files {
        let file_deps = gather_trans_deps(&parser_file);
        for file_dep in file_deps {
            trans_deps.insert(file_dep);
        }
    }

    println!("TRANSITIVE DEPS COUNT = {}", trans_deps.iter().count());
    for dep in trans_deps {
        println!("DEP: {}", dep);
    }


    dir = std::fs::read_dir(Path::new("./blink/Source/core/css/parser")).unwrap();
    let mut blink_css_build = cc::Build::new();
    blink_css_build.cpp(true);
    for entry in gather_files_readdir(dir) {
        blink_css_build.file(entry.to_str().unwrap());
    }
    // println!(" HOST {}", std::env::var("HOST").unwrap());
    // println!("HHOST {}", std::env::var("HHOST").unwrap());
    blink_css_build.target(std::env::var("TTARGET").unwrap().as_str());
    blink_css_build.host(std::env::var("HHOST").unwrap().as_str());
    blink_css_build.opt_level(2);
    blink_css_build.out_dir("./blink_css_out/");
    blink_css_build.compile("blink_css");
}

fn gather_trans_deps(file: &Path) -> Vec<String> {
    let mut res = Vec::new();
    gather_deps_rec(file, &mut Vec::new(),&mut res, 0);
    res
}

fn gather_deps_rec(file: &Path, handled_paths: &mut Vec<String>, trans_deps: &mut Vec<String>, lvl: usize) {
    println!("{}gather_deps_rec {}", "| ".repeat(lvl), file.to_string_lossy());
    if handled_paths.contains(&file.to_string_lossy().to_string()) {
        return;
    }
    handled_paths.push(file.to_string_lossy().to_string());
    let direct_deps = gather_deps(file);
    for dep in direct_deps.into_iter() {
        let proj_rel_path = Path::new("./blink/Source/").join(&dep);
        let file_rel_path = file.parent().unwrap().join(&dep);

        let mut found_path = None;
        if file_rel_path.exists() {
            found_path = Some(file_rel_path);
        } else if proj_rel_path.exists() {
            found_path = Some(proj_rel_path);
        }
        let dep_path = found_path.expect(&format!("Could not find include {}", dep));

        if !trans_deps.contains(&dep) {
            trans_deps.push(dep.clone());
        }
        if !handled_paths.contains(&dep) {
            gather_deps_rec(&dep_path, handled_paths, trans_deps, lvl+1);
        } 
    }
}

fn gather_deps(file: &Path) -> Vec<String> {
    let mut res = Vec::new();
    let mut contents = std::fs::read_to_string(file).unwrap();

    // patch contents
    // for patch in PATCHES {
    //     if patch.0 == file.to_string_lossy() {
    //         println!("applying patch");
    //         contents = contents.replace(patch.1, patch.2);
    //     }        
    // }
        
    let deps_rgx = Regex::new(r#"#include "(?<dep>[^"]*)"#).unwrap();

    let deps = deps_rgx.captures_iter(contents.as_str());

    for dep in deps.into_iter() {
        for dep in dep.iter() {
            let dep = dep.unwrap();
            if dep.as_str().contains("#") || dep.as_str().contains("core/css") {
                continue;
            }
            res.push(dep.as_str().to_owned());
        }
    }
    res
}

fn gather_files_readdir(in_dir: ReadDir) -> Vec<PathBuf> {
    let mut res = Vec::new();
    for entry in in_dir.into_iter() {
        let e = entry.unwrap();
        if e.file_type().unwrap().is_dir() {
            gather_files(&e, &mut res);
        } else {
            res.push(e.path());
        }
    }
    res
}

fn gather_files(in_dir: &DirEntry, paths: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(in_dir.path()).expect(&format!("{} was not a dir", in_dir.file_name().to_str().unwrap())) {
        let e = entry.unwrap();
        if e.file_type().unwrap().is_dir() {
            gather_files(&e, paths);
        } else {
            paths.push(e.path());
        }
    }
}

