use std::{collections::HashSet, fs::{copy, remove_dir_all, rename, DirEntry, ReadDir}, path::{Path, PathBuf}, str};

use regex::Regex;
use std::io;

mod github_release;

fn main() {
    println!("csslancer_bench::main()");

    update_parsers();
}


fn rmdir(dir: &str) -> io::Result<()> {
    std::fs::remove_dir_all(Path::new(dir))
}

fn copy_dir_all(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> io::Result<()> {
    std::fs::create_dir_all(&dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        if ty.is_dir() {
            copy_dir_all(entry.path(), dst.as_ref().join(entry.file_name()))?;
        } else {
            std::fs::copy(entry.path(), dst.as_ref().join(entry.file_name()))?;
        }
    }
    Ok(())
}

fn cpdir(from_dir: &str, to_dir: &str) {
    copy_dir_all(from_dir, to_dir)
        .expect(&format!("could not copy dir {from_dir} to {to_dir}"));
}

fn copy_from_overlay(rel_path: &str) {
    copy(&format!("./blink-overlay/{rel_path}"), &format!("./blink/{rel_path}"));
}


fn clone_repo(url: &str) {
    println!("Cloning repo {url}");
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
    ("./blink/Source/build/scripts/in_file.py", "self._is_sequence(args[arg_name])", "type(args[arg_name]) == type([])"),
    ("./blink/Source/build/scripts/template_expander.py", "func_name", "__name__"),
    ("./blink/Source/build/scripts/make_css_value_keywords.py", "len(enum_enties)", "len(list(enum_enties))"),
    ("./blink/Source/build/scripts/make_css_value_keywords.py", "import sys", "import sys\nfrom functools import reduce"),
    ("./blink/Source/core/css/parser/CSSParser.cpp", r#"#include "core/layout/LayoutTheme.h""#, ""),
    ("./blink/Source/core/css/parser/CSSParser.cpp", "LayoutTheme::theme().systemColor(id)", "0xFFFFFFFF"),
    ("./blink/Source/core/css/parser/CSSParser.h", r#"#include "platform/graphics/Color.h"#, "namespace blink{typedef unsigned RGBA32;}"),
    ("./blink/Source/core/dom/Document.h", "#include \"bindings/core/v8/ExceptionStatePlaceholder.h\"", ""),
    ("./blink/Source/core/dom/Document.h", "#include \"bindings/core/v8/ScriptValue.h\"", ""),
    ("./blink/Source/core/svg/SVGPathSeg.h", "#include \"bindings/core/v8/ScriptWrappable.h\"", ""),
    ("./blink/Source/core/svg/SVGPathSeg.h", ", public ScriptWrappable", ""),
    ("./blink/Source/core/svg/SVGPathSeg.h", "    DEFINE_WRAPPERTYPEINFO();", ""),
    ("./blink/Source/build/scripts/hasher.py", "1L", "1"), // L in python2 denotes long integer literal, in python3 int handles integers of arbitrary size
    ("./blink/Source/build/scripts/hasher.py", "0x9E3779B9L", "0x9E3779B9"),
    ("./blink/Source/build/scripts/hasher.py", "long", "int"),
    ("./blink/Source/build/scripts/templates/MakeNames.h.tmpl", "{% for entry in entries|sort %}", "{% for entry in entries|sort(attribute='name') %}"), // jinja needs key to sort dicts on
];

fn update_parsers() {
    println!("Updating parsers");

    // let _ = rmdir("./blink");
    let _ = rmdir("./blink-css");
    let _ = rmdir("./depot_tools");
    let _ = rmdir("./chromium");

    // clone_repo("https://chromium.googlesource.com/chromium/blink");
    // clone_repo("https://chromium.googlesource.com/chromium/tools/depot_tools");
    // clone_repo("https://chromium.googlesource.com/chromium");

    // let icu_release_asset = github_release::get_release_asset_url("unicode-org", "icu", github_release::ReleaseVersion::Latest, "Win64-MSVC2022.zip").unwrap();
    // github_release::download_file(&icu_release_asset, Path::new("./blink/icu4c-Win64-MSVC2022.zip")).unwrap();
    // std::fs::create_dir_all("./blink/icu/").unwrap();
    // std::process::Command::new("tar")
    //     .args(["-xf", "./blink/icu4c-Win64-MSVC2022.zip", "-C", "./blink/icu/"])
    //     .output()
    //     .expect(&format!("failed unzip unicode-org/icu release"));

    // https://github.com/unicode-org/icu/
    if cfg!(windows) {
        // take Github release icu4c-XX.X-Win64-MSVC2022.zip and unzip at ./blink/icu/
        remove_dir_all("./blink/icu/bin64").unwrap();
        remove_dir_all("./blink/icu/lib64").unwrap();
        cpdir("./blink/icu/include/unicode/", "./blink/unicode/");
    } else if cfg!(target_os = "linux") {
        // take Github release icu4c-XX.X-Fedora_linux40-x64.tgz and untar at ./blink/icu/
        // remove_dir_all("./blink/icu/usr/local/bin/").unwrap();
        // remove_dir_all("./blink/icu/usr/local/lib/").unwrap();
        // remove_dir_all("./blink/icu/usr/local/sbin/").unwrap();
        // remove_dir_all("./blink/icu/usr/local/share/").unwrap();
        cpdir("./blink/icu/usr/local/include/unicode/", "./blink/unicode/");
    } else {
        panic!("Only linux and windows supported.");
    }

    // cpdir("./blink/Source/core/css", "./blink-css");
    // copy("./blink/Source/config.h", "./blink-css/config.h").unwrap();
    

    println!("Patching");

    for patch in PATCHES {
        let prev = std::fs::read_to_string(Path::new(patch.0)).unwrap();
        std::fs::write(Path::new(patch.0), prev.replace(patch.1, patch.2)).unwrap();
    }

    copy_from_overlay("Source/third_party/skia/include/core/SkSize.h");
    copy_from_overlay("Source/platform/graphics/Color.h");
    copy_from_overlay("Source/core/css/parser/CSSParserImpl.cpp");
    copy_from_overlay("Source/core/css/parser/CSSParserMode.cpp");
    copy_from_overlay("Source/core/css/parser/CSSParserMode.h");
    copy_from_overlay("Source/platform/geometry/FloatPoint.h");
    copy_from_overlay("Source/core/layout/LayoutTheme.h");
    copy_from_overlay("Source/core/layout/LayoutTheme.cpp");

    exec_python_file(&["./blink/Source/build/scripts/make_css_property_names.py", "./blink/Source/core/css/CSSProperties.in", "--output_dir", "./blink/Source/core/"]);
    exec_python_file(&["./blink/Source/build/scripts/make_style_shorthands.py", "./blink/Source/core/css/CSSProperties.in", "--output_dir", "./blink/Source/core/"]);
    exec_python_file(&["./blink/Source/build/scripts/make_runtime_features.py", "./blink/Source/platform/RuntimeEnabledFeatures.in", "--output_dir", "./blink/Source/platform/"]);
    exec_python_file(&["./blink/Source/build/scripts/make_css_value_keywords.py", "./blink/Source/core/css/CSSValueKeywords.in", "--output_dir", "./blink/Source/core/"]);
    exec_python_file(&["./blink/Source/build/scripts/make_settings.py", "./blink/Source/core/frame/Settings.in", "--output_dir", "./blink/Source/core/"]);
    exec_python_file(&["./blink/Source/build/scripts/make_css_tokenizer_codepoints.py", "--output_dir", "./blink/Source/core/"]);
    exec_python_file(&["./blink/Source/build/scripts/make_media_features.py", "./blink/Source/core/css/MediaFeatureNames.in", "--output_dir", "./blink/Source/core/"]);
    exec_python_file(&["./blink/Source/build/scripts/make_media_feature_names.py", "./blink/Source/core/css/MediaFeatureNames.in", "--output_dir", "./blink/Source/core/"]);
    exec_python_file(&["./blink/Source/build/scripts/make_names.py", "./blink/Source/core/css/MediaTypeNames.in", "--output_dir", "./blink/Source/core/"]);

    let mut dir = std::fs::read_dir(Path::new("./blink/Source/core/css/parser")).unwrap();

    let parser_files = gather_files_readdir(dir);


    let mut deps = HashSet::new();
    for parser_file in parser_files.clone() {
        let file_deps = gather_deps_repr(&parser_file);
        for file_dep in file_deps.project {
            deps.insert(file_dep);
        }
    }

    println!("DEPS COUNT = {}", deps.iter().count());
    for dep in deps {
        println!("DEP: {}", dep);
    }

    let mut trans_deps = HashSet::new();
    let mut trans_sys_deps = HashSet::new();
    for parser_file in parser_files {
        let file_deps = gather_trans_deps(&parser_file);
        for file_dep in file_deps.project {
            trans_deps.insert(file_dep);
        }
        for file_dep in file_deps.system {
            trans_sys_deps.insert(file_dep);
        }
    }

    println!("\nTRANSITIVE PROJECT DEPS COUNT = {}", trans_deps.iter().count());
    for dep in trans_deps.iter() {
        println!("DEP: {}", dep);
    }

    println!("\nTRANSITIVE SYSTEM DEPS COUNT = {}", trans_sys_deps.iter().count());
    for dep in trans_sys_deps {
        println!("DEP: {}", dep);
    }

    println!("\n");

    println!("Copying to ./blink/comp/");

    std::fs::create_dir_all("./blink/comp/").unwrap();
    for dep in trans_deps {
        let rel_to_blink = dep.path.to_string_lossy().replace("./blink/", "./blink/comp/");
        println!("{}", rel_to_blink);
        std::fs::create_dir_all(Path::new(rel_to_blink.as_str()).parent().unwrap()).unwrap();
        std::fs::copy(dep.path, rel_to_blink).unwrap();
    }

    dir = std::fs::read_dir(Path::new("./blink/comp/")).unwrap();
    let mut blink_css_build = cc::Build::new();
    blink_css_build.cpp(true);
    for entry in gather_files_readdir(dir) {
        blink_css_build.file(entry.to_str().unwrap());
    }
    cpdir("./blink/unicode/", "./blink/comp/unicode/");
    // println!(" HOST {}", std::env::var("HOST").unwrap());
    // println!("HHOST {}", std::env::var("HHOST").unwrap());
    blink_css_build.target(std::env::var("TTARGET").unwrap().as_str());
    blink_css_build.host(std::env::var("HHOST").unwrap().as_str());
    blink_css_build.opt_level(2);
    blink_css_build.out_dir("./blink_css_out/");
    blink_css_build.include("./blink/comp/Source/");
    blink_css_build.include("./blink/comp/");
    blink_css_build.define("ENABLE(feature_name)", "ENABLE_##feature_name"); // define ENABLE MACRO
    println!("COMPILING");
    blink_css_build.compile("blink_css");
}

#[derive(Debug, PartialEq, Eq, Hash)]
struct Dep<T> {
    pub repr: String,
    pub path: T,
}

impl std::fmt::Display for Dep<PathBuf> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} ({})", self.repr, self.path.to_string_lossy())
    }
}

impl std::fmt::Display for Dep<()> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} (SYS)", self.repr)
    }
}

struct Deps {
    pub system: Vec<Dep<()>>,
    pub project: Vec<Dep<PathBuf>>,
}

struct DepsReprs {
    pub system: Vec<String>,
    pub project: Vec<String>,
}

fn gather_trans_deps(file: &Path) -> Deps {
    let mut res = Deps {
        system: Vec::new(),
        project: Vec::new(),
    };
    gather_deps_rec(file, &mut Vec::new(),&mut res, 0);
    res
}

fn gather_deps_rec(file: &Path, handled_paths: &mut Vec<String>, trans_deps: &mut Deps, lvl: usize) {
    println!("GDR {}{}", "|   ".repeat(lvl), file.to_string_lossy());
    if handled_paths.contains(&file.to_string_lossy().to_string()) {
        return;
    }
    handled_paths.push(file.to_string_lossy().to_string());

    if file.ends_with(".h") {
        // recurse into .cpp includes
        let cpp_file_str = file.to_string_lossy().replace(".h", ".cpp");
        let cpp_file = Path::new(&cpp_file_str);
        if cpp_file.exists() {
            gather_deps_rec(cpp_file, handled_paths, trans_deps, lvl);
        }
    }



    // recurse into header includes
    let direct_deps = gather_deps_repr(file);
    for dep in direct_deps.project.into_iter() {
        let source_rel_path = Path::new("./blink/Source/").join(&dep);
        let blink_rel_path = Path::new("./blink/").join(&dep);
        let file_rel_path = file.parent().unwrap().join(&dep);

        let mut found_path = None;
        if file_rel_path.exists() {
            found_path = Some(file_rel_path);
        } else if blink_rel_path.exists(){
            found_path = Some(blink_rel_path);
        } else if source_rel_path.exists() {
            found_path = Some(source_rel_path);
        }
        let dep_path = found_path.expect(&format!("Could not find include {}", dep));

        if !trans_deps.project.iter().any(|d| d.repr == dep) {
            trans_deps.project.push(Dep {
                repr: dep.clone(),
                path: dep_path.clone(),
            });
        }
        if !handled_paths.contains(&dep) {
            gather_deps_rec(&dep_path, handled_paths, trans_deps, lvl+1);
        } 
    }

    for dep in direct_deps.system.into_iter() {
        println!("GDR {} SYS {}", "|   ".repeat(lvl + 1), dep);
        if !trans_deps.system.iter().any(|d| d.repr == dep) {
            trans_deps.system.push(Dep {
                repr: dep,
                path: (),
            });
        }
    }

}

fn gather_deps_repr(file: &Path) -> DepsReprs {
    let mut res_proj = Vec::new();
    let contents = std::fs::read_to_string(file).unwrap();

    let proj_deps_rgx = Regex::new(r#"#include "(?<dep>[^"]*)"#).unwrap();

    let deps = proj_deps_rgx.captures_iter(contents.as_str());

    for dep in deps.into_iter() {
        for dep in dep.iter() {
            let dep = dep.unwrap();
            if dep.as_str().contains("#") || dep.as_str().contains("core/css") {
                continue;
            }
            res_proj.push(dep.as_str().to_owned());
        }
    }

    let mut res_sys = Vec::new();
    let sys_deps_rgx = Regex::new(r#"#include <(?<dep>[^>]*)>"#).unwrap();

    let sys_deps = sys_deps_rgx.captures_iter(contents.as_str());
    for sys_dep in sys_deps.into_iter() {
        for sys_dep in sys_dep.iter() {
            if sys_dep.unwrap().as_str().starts_with("#include") {
                continue;
            }
            res_sys.push(sys_dep.unwrap().as_str().to_owned());
        }
    }

    DepsReprs {
        system: res_sys,
        project: res_proj,
        
    }
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
