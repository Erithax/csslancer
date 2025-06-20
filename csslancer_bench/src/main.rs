use std::{borrow::Cow, collections::HashSet, ffi::OsStr, fmt::Display, fs::{copy, create_dir_all, remove_dir_all, rename, DirEntry, ReadDir}, io::{read_to_string, Write}, path::{Path, PathBuf}, str};

use regex::Regex;
use std::io;

mod github_release;

macro_rules! panic_red {
    () => {{
        panic!();
    }};
    ($($arg:tt)*) => {{
        let msg = format!($($arg)*);
        panic!("\x1b[31m{}\x1b[0m", msg);
    }};
}

pub trait ExpectRedOpt<T> {
    fn expect_red(self, msg: &str) -> T;
}
pub trait ExpectRedRes<T, E> {
    fn expect_red(self, msg: &str) -> T;

}

impl<T> ExpectRedOpt<T> for Option<T> {
    fn expect_red(self, msg: &str) -> T {
        self.expect(&format!("\x1b[31m{}\x1b[0m", msg))
    }
}

impl<T, E> ExpectRedRes<T, E> for Result<T, E> where E : std::fmt::Debug {
    fn expect_red(self, msg: &str) -> T {
        self.expect(&format!("\x1b[31m{}\x1b[0m", msg))
    }
}

fn main() {
    println!("csslancer_bench::main()");
    if !std::env::current_dir().unwrap().ends_with("csslancer_bench") {
        panic_red!("Only call inside of /csslancer/csslancer_bench");
    }
    update_parsers();
}



fn rmdir(dir: &str) -> io::Result<()> {
    std::fs::remove_dir_all(Path::new(dir))
}

fn copy_dir_all(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> io::Result<()> {
    std::fs::create_dir_all(&dst)
        .expect_red(&format!("could not create destination dir : {:?}", dst.as_ref()));
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

fn recurse_files(path: &Path) -> impl IntoIterator<Item = PathBuf> {
    if !path.is_dir() {
        panic_red!("Cannot recurse over `{:?}`", path);
    }

    let mut res = Vec::new();

    for entry in std::fs::read_dir(path).expect_red("could read dir") {
        let entry = entry.expect_red(&format!("could not get entry in dir"));
        let path = entry.path();

        if path.is_dir() {
            res.extend(recurse_files(&path));
        } else {
            res.push(path);
        }
    }

    res
}

fn cpdir(from_dir: &str, to_dir: &str) {
    copy_dir_all(from_dir, to_dir)
        .expect_red(&format!("could not copy dir {from_dir} to {to_dir}"));
}

// fn copy_from_overlay(rel_path: &str) {
//     copy(&format!("./blink-overlay/{rel_path}"), &format!("./blink/{rel_path}"));
// }

fn copy_from_overlay(rel_path: &str) {
    create_dir_all(Path::new(&format!("./chromium/src/{rel_path}")).parent().unwrap())
        .expect_red(&format!("Could not create directory for copy from overlay to src: {rel_path}"));
    copy(&format!("./chromium/overlay/{rel_path}"), &format!("./chromium/src/{rel_path}"))
        .expect_red(&format!("Could not copy `{rel_path}` from overlay to src"));
}

fn clone_repo(url: &str) {
    println!("Cloning repo {url}");
    std::process::Command::new("git")
        .args(["clone", "--depth", "1" , "--no-checkout", url])
        .output()
        .expect_red(&format!("failed to clone repo {url}"));
}

fn cmd_succ(exe: &str, args: &[&str], wrk_dir: &str) {
    let txt = format!("``in {wrk_dir} :{exe} {}``", args.join(" "));
    println!("executing {txt}");
    std::process::Command::new(exe)
        .args(args)
        .current_dir(wrk_dir)
        .output()
        .inspect_err(|err| println!("{err}"))
        .expect_red(&format!("failed to exec cmd {txt}"))
        .status.success().then(|| ())
        .expect_red(&format!("error executing cmd {txt}"));
}

fn sparse_clone(to_dir: &str, url: &str, subtrees: &[&str], top_level: &str) {
    println!("Sparse clone {url}");
    let dir = to_dir;
    create_dir_all(dir).unwrap();

    // cmd_succ("git", &["init"], dir);
    // cmd_succ("git", &["remote", "add", " -f", "origin", url], dir);
    // cmd_succ("git", &["sparse-checkout", "init"], dir);
    // cmd_succ("git", &["sparse-checkout", "set", subtree], dir);
    // cmd_succ("git", &["pull", "--depth", "1", "origin", "main"], dir);

    cmd_succ("git", &["clone", "--no-checkout", "--depth=1", "--filter=tree:0", "--branch", "main", "--single-branch", url], dir);
    cmd_succ("git", &["sparse-checkout", "set", "--no-cone", subtrees.join(" ").as_str()], top_level);
    println!("NOTE: it will NOT download the full 50+ GiB reported");
    println!("NOTE: this might take a minute or two");
    cmd_succ("git", &["checkout"], top_level)
}

fn exec_python_file(args: &[&str]) {
    let out = std::process::Command::new("python3")
        .args(args)
        .output()
        .expect_red(&format!("failed to execute python file `{}`", args[0]));
    if !out.status.success() {
        panic_red!("error executing file {}:\nSTDOUT:{}\nSTDERR:{}", 
            args[0],
            std::string::String::from_utf8_lossy(&out.stdout), 
            std::string::String::from_utf8_lossy(&out.stderr));
    }
}

fn exec_python_module<'a, T, U>(args: T, in_dir: &str) 
    where T : IntoIterator<Item = U>, T : Clone,
        U : AsRef<str>, U : AsRef<OsStr>, U : Display
{
    let out = std::process::Command::new("python3")
        .current_dir(in_dir)
        .args(args.clone())
        .output()
        .expect_red(&format!("failed to execute python file `{}`", args.clone().into_iter().nth(1).unwrap()));

    if !out.status.success() {
        panic_red!("error executing file {}:\nSTDOUT:{}\nSTDERR:{}", 
            args.into_iter().nth(0).unwrap(),
            std::string::String::from_utf8_lossy(&out.stdout), 
            std::string::String::from_utf8_lossy(&out.stderr));
    }
}

// see /src/third_party/blink/renderer/build/scripts/scripts.gni:230
// see csslancer_bench/chromium/OG/src/third_party/blink/renderer/core/BUILD.gn scripts deriving from CSSProperties
fn css_properties(module: &str, other_inputs: &[&str], out_dir: &str) {
    exec_python_module(
        [
            &[ "-m", 
                module, 
                "./../../core/css/css_properties.json5",
                "./../../core/css/computed_style_field_aliases.json5",
                "./../../platform/runtime_enabled_features.json5"
            ],
            other_inputs,
            &[ 
                "--output_dir", out_dir,
            ],
        ].into_iter().flatten().collect::<Vec<&&str>>(), "./chromium/src/third_party/blink/renderer/build/scripts/");
}

// const PATCHES: &'static [(&'static str, &'static str, &'static str)] = &[
    // ("./blink/Source/build/scripts/in_generator.py", r#"print "USAGE: %s INPUT_FILES" % script_name"#, r#"print("USAGE: %s INPUT_FILES" % script_name)"#),
    // ("./blink/Source/build/scripts/in_file.py", "print message", r#"print(message)"#),
    // ("./blink/Source/build/scripts/in_generator.py", "basestring", "str"),
    // ("./blink/Source/build/scripts/in_file.py", "self._is_sequence(args[arg_name])", "type(args[arg_name]) == type([])"),
    // ("./blink/Source/build/scripts/template_expander.py", "func_name", "__name__"),
    // ("./blink/Source/build/scripts/make_css_value_keywords.py", "len(enum_enties)", "len(list(enum_enties))"),
    // ("./blink/Source/build/scripts/make_css_value_keywords.py", "import sys", "import sys\nfrom functools import reduce"),
    // ("./blink/Source/core/css/CSSRule.h", "#include \"bindings/core/v8/ScriptWrappable.h\"", ""),
    // ("./blink/Source/core/css/CSSRule.h", ", public ScriptWrappable", ""),
    // ("./blink/Source/core/css/MediaList.h", "#include \"bindings/core/v8/ScriptWrappable.h\"", ""),
    // ("./blink/Source/core/css/MediaList.h", ", public ScriptWrappable", ""),
    // ("./blink/Source/core/css/CSSStyleDeclaration.h", "#include \"bindings/core/v8/ScriptWrappable.h\"", ""),
    // ("./blink/Source/core/css/CSSStyleDeclaration.h", ", public ScriptWrappable", ""),
    // ("./blink/Source/core/css/StyleSheet.h", "#include \"bindings/core/v8/ScriptWrappable.h\"", ""),
    // ("./blink/Source/core/css/StyleSheet.h", ", public ScriptWrappable", ""),
    // ("./blink/Source/core/style/ComputedStyle.h", "#include \"core/animation/css/CSSAnimationData.h\"", ""),
    // ("./blink/Source/core/style/ComputedStyle.h", r#"const CSSAnimationData* animations() const { return rareNonInheritedData->m_animations.get(); }"#, ""),
    // ("./blink/Source/core/style/ComputedStyle.h", r#"CSSAnimationData& accessAnimations();"#, ""),
    // ("./blink/Source/core/css/parser/CSSParser.cpp", "#include \"core/layout/LayoutTheme.h\"", ""),
    // ("./blink/Source/core/css/parser/CSSParser.cpp", "LayoutTheme::theme().systemColor(id)", "0xFFFFFFFF"),
    // ("./blink/Source/core/css/parser/CSSParser.h", "#include \"platform/graphics/Color.h\"", "namespace blink{typedef unsigned RGBA32;}"),
    // ("./blink/Source/core/dom/Document.h", "#include \"bindings/core/v8/ExceptionStatePlaceholder.h\"", ""),
    // ("./blink/Source/core/dom/Document.h", "#include \"bindings/core/v8/ScriptValue.h\"", ""),
    // ("./blink/Source/core/svg/SVGPathSeg.h", "#include \"bindings/core/v8/ScriptWrappable.h\"", ""),
    // ("./blink/Source/core/svg/SVGPathSeg.h", ", public ScriptWrappable", ""),
    // ("./blink/Source/core/svg/SVGPathSeg.h", "    DEFINE_WRAPPERTYPEINFO();", ""),
    // ("./blink/Source/core/frame/UseCounter.h", "#include <v8.h>", ""),
    // TBD: fixes for removing v8 from UseCounter ("./blink/Source/core/frame/UseCounter.h", "")
    // ("./blink/Source/wtf/LinkedHashSet.h", " swapAnchor(m_", " this->swapAnchor(m_"), // fix there are no arguments to 'swapAnchor' that depend on a template parameter, so a declaration of 'swapAnchor' must be available
    // ("./blink/Source/build/scripts/hasher.py", "1L", "1"), // L in python2 denotes long integer literal, in python3 int handles integers of arbitrary size
    // ("./blink/Source/build/scripts/hasher.py", "0x9E3779B9L", "0x9E3779B9"),
    // ("./blink/Source/build/scripts/hasher.py", "long", "int"),
    // ("./blink/Source/build/scripts/templates/MakeNames.h.tmpl", "{% for entry in entries|sort %}", "{% for entry in entries|sort(attribute='name') %}"), // jinja needs key to sort dicts on
// ];

const PATCHES: &'static [(&'static str, &'static str, &'static str)] = &[
    ("./chromium/src/third_party/blink/renderer/platform/wtf/type_traits.h", "#include \"v8/include/cppgc/type-traits.h\"", ""),
    ("./chromium/src/third_party/blink/renderer/platform/heap/collection_support/heap_vector.h", "public GarbageCollected<HeapVector<T, inlineCapacity>>,\n", ""),
    ("./chromium/src/third_party/blink/renderer/platform/heap/collection_support/heap_vector.h", "#include \"third_party/blink/renderer/platform/heap/garbage_collected.h\"", ""),
    ("./chromium/src/third_party/blink/renderer/core/css/css_property_names.h", "#include \"third_party/blink/public/mojom/use_counter/metrics/css_property_id.mojom-blink-forward.h\"", ""),
    ("./chromium/src/third_party/blink/renderer/core/style/position_area.h", "#include \"third_party/blink/renderer/core/layout/geometry/box_sides.h\"", ""),
    ("./chromium/src/third_party/blink/renderer/core/style/position_area.h", "#include \"third_party/blink/renderer/core/layout/geometry/box_strut.h\"", ""),
    ("./chromium/src/third_party/blink/renderer/core/style/position_area.h", "#include \"third_party/blink/renderer/core/layout/geometry/layout_unit.h\"", ""),
    ("./chromium/src/third_party/blink/renderer/core/css/anchor_evaluator.h", "#include \"third_party/blink/renderer/platform/geometry/physical_offset.h\"", ""),
    ("./chromium/src/third_party/blink/renderer/core/css/media_values.h", "#include \"ui/base/pointer/pointer_device.h\"", ""),

    //
    ("./chromium/src/third_party/blink/renderer/core/css/css_identifier_value.h", "#include \"third_party/blink/renderer/core/css/css_value_id_mappings.h\"", ""),
    ("./chromium/src/third_party/blink/renderer/core/css/css_value_id_mappings.h", "#include \"third_party/blink/renderer/core/animation/effect_model.h\"", ""),
    ("./chromium/src/third_party/blink/renderer/core/css/css_value_id_mappings_generated.h", "#include \"cc/input/scroll_snap_data.h\"", ""),
    ("./chromium/src/third_party/blink/renderer/core/css/css_value_id_mappings_generated.h", "#include \"third_party/blink/public/mojom/frame/color_scheme.mojom-blink.h\"", ""),
    ("./chromium/src/third_party/blink/renderer/core/css/css_value_id_mappings_generated.h", "#include \"third_party/blink/public/mojom/scroll/scroll_enums.mojom-blink.h\"", ""),
    ("./chromium/src/third_party/blink/renderer/core/css/css_value_id_mappings_generated.h", "#include \"third_party/blink/renderer/core/animation/effect_model.h\"", ""),
    ("./chromium/src/third_party/blink/renderer/core/css/css_value_id_mappings_generated.h", "#include \"third_party/blink/renderer/core/layout/layout_theme.h\"", ""),
    ("./chromium/src/third_party/blink/renderer/core/css/css_value_id_mappings_generated.h", "#include \"third_party/blink/renderer/core/style/basic_shapes.h\"", ""),

    ("./chromium/src/third_party/blink/renderer/core/css/style_color.h", "#include \"third_party/blink/public/mojom/frame/color_scheme.mojom-blink-forward.h\"", ""),
    ("./chromium/src/third_party/blink/renderer/platform/graphics/color.h", "#include \"third_party/skia/include/core/SkColor.h\"", ""),

    ("./chromium/src/third_party/blink/renderer/platform/graphics/color.h", "#include \"third_party/skia/include/core/SkColor.h\"", ""),
    ("./chromium/src/third_party/blink/renderer/core/css/parser/at_rule_counter_style_descriptor_parser.cc", "#include \"third_party/blink/renderer/core/dom/document.h\"", ""),
    ("./chromium/src/third_party/blink/renderer/core/css/css_font_face_src_value.h", "#include \"third_party/blink/renderer/core/loader/resource/font_resource.h\"", ""),
    ("./chromium/src/third_party/blink/renderer/core/css/css_font_face_src_value.h", "#include \"third_party/blink/renderer/platform/bindings/dom_wrapper_world.h\"", ""),

    ("./chromium/src/third_party/blink/renderer/core/css/css_style_declaration.h", "#include \"third_party/blink/renderer/core/execution_context/execution_context_lifecycle_observer.h\"", ""),
    ("./chromium/src/third_party/blink/renderer/core/css/css_style_declaration.h", "#include \"third_party/blink/renderer/platform/bindings/script_wrappable.h\"", ""),
    ("./chromium/src/third_party/blink/renderer/core/css/css_style_declaration.h", "#include \"third_party/blink/renderer/platform/bindings/v8_binding.h\"", ""),

    ("./chromium/src/third_party/blink/renderer/core/css/media_list.h", "#include \"third_party/blink/renderer/platform/bindings/script_wrappable.h\"", ""),
    ("./chromium/src/third_party/blink/renderer/core/css/parser/at_rule_descriptor_parser_test.cc", "#include \"third_party/blink/renderer/core/dom/document.h\"", ""),
    ("./chromium/src/third_party/blink/renderer/core/css/parser/at_rule_descriptor_parser_test.cc", "#include \"third_party/blink/renderer/core/testing/page_test_base.h\"", ""),

    ("./chromium/src/third_party/blink/renderer/core/css/parser/container_query_parser.cc", "#include \"third_party/blink/renderer/core/css/resolver/style_builder_converter.h\"", ""),
    ("./chromium/src/third_party/blink/renderer/core/css/parser/container_query_parser.cc", "#include \"third_party/blink/renderer/core/frame/web_feature.h\"", ""),
    ("./chromium/src/third_party/blink/renderer/core/css/parser/container_query_parser_test.cc", "#include \"third_party/blink/renderer/core/testing/page_test_base.h\"", ""),
    
    ("./chromium/src/third_party/blink/renderer/core/css/parser/css_at_rule_id.cc", "#include \"third_party/blink/renderer/core/frame/web_feature.h\"", ""),
    ("./chromium/src/third_party/blink/renderer/core/css/parser/css_at_rule_id.cc", "#include \"third_party/blink/renderer/platform/instrumentation/use_counter.h\"", ""),

    ("./chromium/src/third_party/blink/renderer/core/css/parser/css_if_parser_test.cc", "#include \"third_party/blink/renderer/core/testing/page_test_base.h\"", ""),
    ];

const INCLUDES_TO_DELETE: &'static [&'static str] = &[
    "third_party/blink/renderer/core/testing/page_test_base.h",
    "third_party/blink/renderer/platform/bindings/script_wrappable.h",
    "third_party/blink/renderer/core/layout/geometry/(layout_unit|box_sides|box_strut|physical_offset).h",
    "third_party/blink/renderer/core/dom/document.h",
    "third_party/skia/include/core/SkColor.h",
    "third_party/blink/renderer/core/frame/web_feature.h",
    "third_party/blink/renderer/platform/bindings/(v8_binding|script_wrappable).h",
    "third_party/blink/renderer/platform/heap/garbage_collected.h",
    "third_party/blink/public/mojom/use_counter/metrics/css_property_id.mojom-blink-forward.h",
    "third_party/blink/public/mojom/frame/color_scheme.mojom-blink-forward.h",
    "third_party/blink/public/mojom/scroll/scroll_enums.mojom-blink.h",
    "v8/include/cppgc/type-traits.h",
    "third_party/blink/renderer/core/loader/resource/font_resource.h",
    "third_party/blink/renderer/core/testing/page_test_base.h",
    "third_party/blink/renderer/platform/instrumentation/use_counter.h",
    "third_party/blink/renderer/core/css/resolver/style_builder_converter.h",
    "third_party/blink/renderer/platform/bindings/dom_wrapper_world.h",
    "third_party/blink/renderer/core/animation/effect_model.h",
    "ui/base/pointer/pointer_device.h",
];


fn update_parsers() {
    println!("Updating parsers");

    // let _ = rmdir("./blink");
    let _ = rmdir("./blink-css");
    let _ = rmdir("./depot_tools");

    if false {
        let _ = rmdir("./chromium");
        //sparse_clone("./chromium", "https://chromium.googlesource.com/chromium/src", "third_party/blink");
        sparse_clone("./chromium", "https://chromium.googlesource.com/chromium/src", &["/third_party/blink", "/base", "/build", "/third_party/perfetto", "/third_party/rapidhash", "/third_party/abseil-cpp"     ], "./chromium/src");
        cmd_succ("git", &["submodule", "update", "./third_party/perfetto/"], "./chromium/src/");
        cmd_succ("git", &["submodule", "update", "./third_party/googletest/src/"], "./chromium/src/");
        cmd_succ("git", &["submodule", "update", "./third_party/re2"], "./chromium/src/");
    }

    // let rx_patches: Vec<(&'static str, Regex, &'static str)> = vec![
    //     ("./chromium/src/third_party/blink/renderer/core/css/media_values.h", Regex::new(r#"#include \"[a-zA-Z0-9\./]*(?!/mojom/)(/mojom/)[a-zA-Z0-9\./]*\""#).unwrap(), ""),
    // ];

    // cpdir("./chromium/src/third_party/blink/", "./chromium/dst/third_party/blink/");
    
    // clone_repo("https://chromium.googlesource.com/chromium/blink");
    // clone_repo("https://chromium.googlesource.com/chromium/tools/depot_tools");
    // clone_repo("https://chromium.googlesource.com/chromium");

    // let icu_release_asset = github_release::get_release_asset_url("unicode-org", "icu", github_release::ReleaseVersion::Latest, "Win64-MSVC2022.zip").unwrap();
    // github_release::download_file(&icu_release_asset, Path::new("./blink/icu4c-Win64-MSVC2022.zip")).unwrap();
    // std::fs::create_dir_all("./blink/icu/").unwrap();
    // std::process::Command::new("tar")
    //     .args(["-xf", "./blink/icu4c-Win64-MSVC2022.zip", "-C", "./blink/icu/"])
    //     .output()
    //     .expect_red(&format!("failed unzip unicode-org/icu release"));

    // https://github.com/unicode-org/icu/
    if cfg!(windows) {
        // take Github release icu4c-XX.X-Win64-MSVC2022.zip and unzip at ./blink/icu/
        // remove_dir_all("./blink/icu/bin64").unwrap();
        // remove_dir_all("./blink/icu/lib64").unwrap();
        cpdir("./blink/icu/include/unicode/", "./blink/unicode/");
    } else if cfg!(target_os = "linux") {
        // take Github release icu4c-XX.X-Fedora_linux40-x64.tgz and untar at ./blink/icu/
        // remove_dir_all("./blink/icu/usr/local/bin/").unwrap();
        // remove_dir_all("./blink/icu/usr/local/lib/").unwrap();
        // remove_dir_all("./blink/icu/usr/local/sbin/").unwrap();
        // remove_dir_all("./blink/icu/usr/local/share/").unwrap();
        //cpdir("./blink/icu/usr/local/include/unicode/", "./blink/unicode/");
    } else {
        panic_red!("Only linux and windows supported.");
    }

    // cpdir("./blink/Source/core/css", "./blink-css");
    // copy("./blink/Source/config.h", "./blink-css/config.h").unwrap();

    println!("Patching");

    for patch in PATCHES {
        println!("Patch {}", patch.0);
        let prev = std::fs::read_to_string(Path::new(patch.0)).unwrap();
        std::fs::write(Path::new(patch.0), prev.replace(patch.1, patch.2)).unwrap();
    }

    // for file in recurse_files(Path::new("./chromium/src/")) {
        
    //     let Ok(mut contents) = std::fs::read_to_string(&file) else {
    //         continue;
    //     };
    //     for include in INCLUDES_TO_DELETE {
    //         contents = Regex::new(&("#include \"".to_owned() + include + "\""))
    //             .expect_red("include regex invalid")
    //             .replace_all(&contents, "")
    //             .to_string();
    //     }
    //     std::fs::write(file, contents).expect_red("failed to write file after removing includes");
    // }

    // for (path, rx, repl) in rx_patches {
    //     println!("Patch {}", path);
    //     let prev = std::fs::read_to_string(Path::new(path)).unwrap();
    //     std::fs::write(Path::new(path), rx.replace_all(&prev, repl).to_string()).unwrap();
    // }
    let p = Path::new("./chromium/src/third_party/blink/renderer/core/css/media_values.h");
    let prev = std::fs::read_to_string(p).unwrap();
    std::fs::write(p, prev.lines().filter(|line| !(line.starts_with("#include") && line.contains("/mojom/"))).map(|line| line.to_owned() + "\n").collect::<String>()).unwrap();

    // copy_from_overlay("Source/third_party/skia/include/core/SkSize.h");
    // copy_from_overlay("Source/platform/graphics/Color.h");
    // copy_from_overlay("Source/core/css/parser/CSSParserImpl.cpp");
    // copy_from_overlay("Source/core/css/parser/CSSParserMode.cpp");
    // copy_from_overlay("Source/core/css/parser/CSSParserMode.h");
    // copy_from_overlay("Source/platform/geometry/FloatPoint.h");
    // copy_from_overlay("Source/core/layout/LayoutTheme.h");
    // copy_from_overlay("Source/core/layout/LayoutTheme.cpp");


    // copy_from_overlay("v8/include/cppgc/custom-space.h");
    copy_from_overlay("v8/include/v8config.h");
    copy_from_overlay("v8/include/v8-source-location.h");

    copy_from_overlay("v8/tools/gen-v8-gn.py");


    // see v8/BUILD.gn:3313
    exec_python_file(&["./chromium/src/v8/tools/gen-v8-gn.py",
        "-o", "./chromium/src/v8/include/v8-gn.h",
    ]);




    // see /build/buildflag_header.gni

    // see /base/BUILD.gn:2664
    // TODO: actually write GN build flags to the --flags file
    let debugging_buildflags_flags = "./chromium/src/build/debugging_buildflags_flags";
    std::fs::File::create(debugging_buildflags_flags).unwrap()
        .write("--flags".as_bytes()).unwrap();
    exec_python_file(&["./chromium/src/build/write_buildflag_header.py", 
        "--output", "debugging_buildflags.h",
        "--gen-dir", "./chromium/src/base/debug",
        "--definitions", debugging_buildflags_flags]);

    // see /base/BUILD.gn:
    // TODO: actually write GN build flags to the --flags file
    let fuzzing_build_flags_path = "./chromium/src/build/fuzzing_buildflags_flags";
    std::fs::File::create(fuzzing_build_flags_path).unwrap()
        .write("--flags".as_bytes()).unwrap();
    exec_python_file(&["./chromium/src/build/write_buildflag_header.py", 
        "--output", "fuzzing_buildflags.h",
        "--gen-dir", "./chromium/src/base",
        "--definitions", fuzzing_build_flags_path]);

    let tracing_build_flags_path = "./chromium/src/build/tracing_buildflags_flags";
    std::fs::File::create(tracing_build_flags_path).unwrap()
        .write("--flags".as_bytes()).unwrap();
    exec_python_file(&["./chromium/src/build/write_buildflag_header.py",
        "--output", "tracing_buildflags.h",
        "--gen-dir", "./chromium/src/base",
        "--definitions", tracing_build_flags_path]);

    // see ./chromium/src/base/allocator/partition_allocator/src/partition_alloc/buildflag_header.gni
    // ./chromium/src/base/allocator/partition_allocator/src/partition_alloc/BUILD.gn:135
    // TODO: actually write GN build flags to the --flags file
    let part_alloc_build_flags_path = "./chromium/src/base/allocator/partition_allocator/src/partition_alloc/buildflags_flags";
    std::fs::File::create(part_alloc_build_flags_path).unwrap()
        .write("--flags".as_bytes()).unwrap();
    exec_python_file(&["./chromium/src/base/allocator/partition_allocator/src/partition_alloc/write_buildflag_header.py", 
        "--output", "buildflags.h",
        "--gen-dir", "./chromium/src/base/allocator/partition_allocator/src/partition_alloc",
        "--definitions", part_alloc_build_flags_path]);

    // see ./chromium/src/base/synchronization
    // ./chromium/src/base/??/BUILD.gn:135
    // TODO: actually write GN build flags to the --flags file
    let synchronization_build_flags_path = "./chromium/src/base/synchronization/buildflags_flags";
    std::fs::File::create(synchronization_build_flags_path).unwrap()
        .write("--flags".as_bytes()).unwrap();
    exec_python_file(&["./chromium/src/build/write_buildflag_header.py",
        "--output", "synchronization_buildflags.h",
        "--gen-dir", "./chromium/src/base/synchronization",
        "--definitions", synchronization_build_flags_path]);

    // see
    // ./chromium/src/??/BUILD.gn:135
    // TODO: actually write GN build flags to the --flags file
    let chromeos_buildflags_gen_flags = "./chromium/src/build/chromeos_buildflags_flags";
    std::fs::File::create(chromeos_buildflags_gen_flags).unwrap()
        .write("--flags".as_bytes()).unwrap();
    exec_python_file(&["./chromium/src/build/write_buildflag_header.py",
        "--output", "chromeos_buildflags.h",
        "--gen-dir", "./chromium/src/build",
        "--definitions", chromeos_buildflags_gen_flags]);

    // see 
    // ./chromium/src/??/BUILD.gn:135
    // TODO: actually write GN build flags to the --flags file
    let feature_list_buildflags_flags = "./chromium/src/base/feature_list_buildflags_flags";
    std::fs::File::create(feature_list_buildflags_flags).unwrap()
        .write("--flags".as_bytes()).unwrap();
    exec_python_file(&["./chromium/src/build/write_buildflag_header.py",
        "--output", "feature_list_buildflags.h",
        "--gen-dir", "./chromium/src/base",
        "--definitions", feature_list_buildflags_flags]);

    // see 
    // ./chromium/src/??/BUILD.gn:135
    // TODO: actually write GN build flags to the --flags file
    let heap_buildflags_flags = "./chromium/src/third_party/blink/renderer/platform/heap/heap_buildflags_flags";
    std::fs::File::create(heap_buildflags_flags).unwrap()
        .write("--flags".as_bytes()).unwrap();
    exec_python_file(&["./chromium/src/build/write_buildflag_header.py",
        "--output", "heap_buildflags.h",
        "--gen-dir", "./chromium/src/third_party/blink/renderer/platform/heap",
        "--definitions", heap_buildflags_flags]);

    // see 
    // ./chromium/src/base/BUILD.gn:2729
    // TODO: actually write GN build flags to the --flags file
    let protmem_buildflags_flags = "./chromium/src/base/protected_memory_buildflags_flags";
    std::fs::File::create(protmem_buildflags_flags).unwrap()
        .write("--flags".as_bytes()).unwrap();
    exec_python_file(&["./chromium/src/build/write_buildflag_header.py",
        "--output", "protected_memory_buildflags.h",
        "--gen-dir", "./chromium/src/base/memory/",
        "--definitions", protmem_buildflags_flags]);


    // see 
    // ./chromium/src/??/BUILD.gn:2729
    // TODO: actually write GN build flags to the --flags file
    let protmem_buildflags_flags = "./chromium/src/base/protected_memory_buildflags_flags";
    std::fs::File::create(protmem_buildflags_flags).unwrap()
        .write("--flags".as_bytes()).unwrap();
    exec_python_file(&["./chromium/src/build/write_buildflag_header.py",
        "--output", "protected_memory_buildflags.h",
        "--gen-dir", "./chromium/src/base/memory/",
        "--definitions", protmem_buildflags_flags]);
    
    // exec_python_file(&["./blink/Source/build/scripts/make_css_property_names.py", "./blink/Source/core/css/CSSProperties.in", "--output_dir", "./blink/Source/core/"]);
    // exec_python_file(&["./blink/Source/build/scripts/make_style_shorthands.py", "./blink/Source/core/css/CSSProperties.in", "--output_dir", "./blink/Source/core/"]);
    exec_python_file(&["./chromium/src/third_party/blink/renderer/build/scripts/make_runtime_features.py", 
        "./chromium/src/third_party/blink/renderer/platform/runtime_enabled_features.json5", 
        "--output_dir", "./chromium/src/third_party/blink/renderer/platform/"]);
    // exec_python_file(&["./blink/Source/build/scripts/make_css_value_keywords.py", "./blink/Source/core/css/CSSValueKeywords.in", "--output_dir", "./blink/Source/core/"]);
    exec_python_file(&["./chromium/src/third_party/blink/renderer/build/scripts/make_settings.py", 
        "./chromium/src/third_party/blink/renderer/core/frame/settings.json5", 
        "--output_dir", "./chromium/src/third_party/blink/renderer/core/frame/"]);
    // exec_python_file(&["./blink/Source/build/scripts/make_css_tokenizer_codepoints.py", "--output_dir", "./blink/Source/core/"]);
    // exec_python_file(&["./blink/Source/build/scripts/make_media_features.py", "./blink/Source/core/css/MediaFeatureNames.in", "--output_dir", "./blink/Source/core/"]);
    
    // see csslancer_bench/chromium/OG/src/third_party/blink/renderer/core/BUILD.gn:993
    // exec_python_file(&["./chromium/src/third_party/blink/renderer/build/scripts/core/css/make_media_feature_names.py", 
    //     "./chromium/src/third_party/blink/renderer/core/css/media_feature_names.json5", 
    //     "--output_dir", "./blink/Source/core/css/"]);
    exec_python_module(&[
        "-m",
        "core.css.make_media_feature_names",
        "./../../core/css/media_feature_names.json5",
        "--output_dir", "./../../core/css/",
    ], "./chromium/src/third_party/blink/renderer/build/scripts/");

    // see csslancer_bench/chromium/src/third_party/blink/renderer/core/BUILD.gn:1008
    exec_python_file(&["./chromium/src/third_party/blink/renderer/build/scripts/make_names.py", 
        "./chromium/src/third_party/blink/renderer/core/css/media_type_names.json5", 
        "--output_dir", "./chromium/src/third_party/blink/renderer/core/"]);
    // exec_python_file(&["./chromium/src/third_party/blink/renderer/build/scripts/core/css/make_css_property_names.py", 
    //     "./chromium/src/third_party/blink/renderer/core/css/css_properties.json5", 
    //     "--output_dir", "./chromium/src/third_party/blink/renderer/core/css/"]);

    // exec_python_file(&["./chromium/src/third_party/blink/renderer/build/scripts/make_names.py", 
    //     "./chromium/src/third_party/blink/renderer/core/css/media_type_names.json5", 
    //     "--output_dir", "./chromium/src/third_party/blink/renderer/core/css/"]);


    // see csslancer_bench/chromium/src/third_party/blink/renderer/core/BUILD.gn
    exec_python_file(&["./chromium/src/third_party/blink/renderer/build/scripts/make_names.py", 
        "./chromium/src/third_party/blink/renderer/core/html/keywords.json5", 
        "--output_dir", "./chromium/src/third_party/blink/renderer/core/"]);

    
    // see csslancer_bench/chromium/OG/src/third_party/blink/renderer/core/BUILD.gn:883
    // see csslancer_bench/chromium/OG/src/third_party/blink/renderer/build/scripts/scripts.gni:199

    // code_generator("make_core_generated_css_value_keywords") {
    //     script = "../build/scripts/core/css/make_css_value_keywords.py"
    //     json_inputs = [ "css/css_value_keywords.json5" ]
    //     other_inputs = [ "../build/scripts/gperf.py" ]
    //     templates = [
    //       "../build/scripts/core/css/templates/css_value_keywords.cc.tmpl",
    //       "../build/scripts/core/css/templates/css_value_keywords.h.tmpl",
    //     ]
    //     outputs = [
    //       "$blink_core_output_dir/css_value_keywords.cc",
    //       "$blink_core_output_dir/css_value_keywords.h",
    //     ]
    //     other_args = [
    //       "--gperf",
    //       gperf_exe,
    //     ]
    //   }
    exec_python_module(&[
        "-m",
        "core.css.make_css_value_keywords",
        "./../../core/css/css_value_keywords.json5",
        "--output_dir", "./../../core/",
    ], "./chromium/src/third_party/blink/renderer/build/scripts/");

    // csslancer_bench/chromium/OG/src/third_party/blink/renderer/core/BUILD.gn
    // TODO: assert that this file contains 10 occurences of 'css_properties('

    // see csslancer_bench/chromium/OG/src/third_party/blink/renderer/core/BUILD.gn:731
    css_properties("core.style.make_computed_style_initial_values", &["./../../core/style/computed_style_extra_fields.json5"], "./../../core/style/");

    // see csslancer_bench/chromium/OG/src/third_party/blink/renderer/core/BUILD.gn:738
    css_properties("core.style.make_computed_style_base",
     &[
        "./../../core/style/computed_style_extra_fields.json5",
        "./../../core/css/css_value_keywords.json5",
        "./../../core/css/css_group_config.json5",
        ], "./../../core/style/");

    // see csslancer_bench/chromium/OG/src/third_party/blink/renderer/core/BUILD.gn:772
    css_properties("core.css.make_css_value_id_mappings", &["./../../core/css/css_value_keywords.json5"], "./../../core/css/");

    // see csslancer_bench/chromium/OG/src/third_party/blink/renderer/core/BUILD.gn:783
    css_properties("core.css.properties.make_css_property_instances", &[], "./../../core/css/properties/");
    
    // see csslancer_bench/chromium/OG/src/third_party/blink/renderer/core/BUILD.gn:795
    css_properties("core.css.properties.make_css_property_subclasses", &["./../../core/css/properties/css_property_methods.json5"], "./../../core/css/properties/");

    // see csslancer_bench/chromium/OG/src/third_party/blink/renderer/core/BUILD.gn:811
    css_properties("core.css.make_css_property_names", &[], "./../../core/css/");

    // see csslancer_bench/chromium/OG/src/third_party/blink/renderer/core/BUILD.gn:851
    css_properties("core.css.make_style_shorthands", &[], "./../../core/");

    // see csslancer_bench/chromium/OG/src/third_party/blink/renderer/core/BUILD.gn:863
    css_properties("core.css.make_cssom_types", &[], "./../../core/");

    // see csslancer_bench/chromium/OG/src/third_party/blink/renderer/core/BUILD.gn:875
    css_properties("core.css.properties.make_property_bitsets", &[], "./../../core/css/properties/");

    // see csslancer_bench/chromium/OG/src/third_party/blink/renderer/core/BUILD.gn:1735
    css_properties("core.css.parser.make_proto", &["./../../core/css/css_value_keywords.json5"], "./../../core/css/parser/");

    // TODO: determine why these don't get copied over/deleted in previous steps
    // I think this is because of the cleanup in /chromium/src/third_party/blink/renderer/build/scripts/core/css/properties/make_css_property_instances.py
    let copy_from_og = |rel_path: &str| std::fs::copy(
        format!("./chromium/OG/src/{}", rel_path),
        format!("./chromium/src/{}", rel_path),
    ).unwrap();
    let copy_from_og_css = |rel_path: &str| copy_from_og(
        &format!("third_party/blink/renderer/core/css/{}", rel_path)
    );
    copy_from_og_css("properties/css_property.h");
    copy_from_og_css("properties/css_property.cc");
    copy_from_og_css("properties/css_unresolved_property.h");
    copy_from_og_css("properties/css_unresolved_property.cc");

    for patch in PATCHES {
        println!("Patch {}", patch.0);
        let prev = std::fs::read_to_string(Path::new(patch.0)).unwrap();
        std::fs::write(Path::new(patch.0), prev.replace(patch.1, patch.2)).unwrap();
    }

    let mut dir = std::fs::read_dir(Path::new("./chromium/src/third_party/blink/renderer/core/css/parser")).unwrap();

    let parser_files = gather_files_readdir(dir);


    let mut deps = HashSet::new();
    for parser_file in parser_files.clone() {
        let file_deps = gather_deps_repr(&parser_file);
        for file_dep in file_deps.project {
            deps.insert(file_dep);
        }
    }

    println!("DEPS COUNT = {}", deps.iter().count());
    // for dep in deps {
    //     println!("DEP: {}", dep);
    // }

    let include_dirs = &[
        "./chromium/src/", 
        "./chromium/src/base/allocator/partition_allocator/src/",
        "./chromium/src/third_party/googletest/src/googletest/include/",
        "./chromium/src/third_party/abseil-cpp/", // referenced in ./chromium/src/third_party/googletest/src/googletest/include/gtest/internal/gtest-port.h
        "./chromium/src/third_party/re2/src/", // referenced in ./chromium/src/third_party/googletest/src/googletest/include/gtest/internal/gtest-port.h
        "./chromium/src/v8/include/", // referenced in ./chromium/src/v8/include/cppgc
    ];

    let mut trans_deps = HashSet::new();
    let mut trans_sys_deps = HashSet::new();
    for parser_file in parser_files.iter() {
        let file_deps = gather_trans_deps(&parser_file, include_dirs);
        for file_dep in file_deps.project {
            trans_deps.insert(file_dep);
        }
        for file_dep in file_deps.system {
            trans_sys_deps.insert(file_dep);
        }
    }

    println!("\nTRANSITIVE PROJECT DEPS COUNT = {}", trans_deps.iter().count());
    // for dep in trans_deps.iter() {
    //     println!("DEP: {}", dep);
    // }

    println!("\nTRANSITIVE SYSTEM DEPS COUNT = {}", trans_sys_deps.iter().count());
    // for dep in trans_sys_deps {
    //     println!("DEP: {}", dep);
    // }

    let mut all_files = trans_deps;
    all_files.extend(parser_files.into_iter()
        .map(|file| Dep {
            repr: file.to_str().unwrap().to_string(),
            path: file,
        })
    );

    println!("\n");

    println!("Copying to ./chromium/dst/");

    std::fs::create_dir_all("./chromium/dst/").unwrap();
    for file in all_files {
        println!("{file}");
        let rel_to_blink = file.path.to_string_lossy().replace("./blink/", "./blink/comp/");
        println!("{}", rel_to_blink);
        std::fs::create_dir_all(Path::new(rel_to_blink.as_str()).parent().unwrap()).unwrap();
        std::fs::copy(file.path, rel_to_blink).unwrap();
    }

    dir = std::fs::read_dir(Path::new("./chromium/dst/")).unwrap();
    let mut blink_css_build = cc::Build::new();
    blink_css_build.cpp(true);
    for entry in gather_files_readdir(dir) {
        // if entry.to_str().unwrap().contains("CSSTokenizerCodepoints") {
        //     println!("uh no dont include this partial file!");
        //     continue;
        // }
        blink_css_build.file(entry.to_str().unwrap());
    }
    //cpdir("./blink/unicode/", "./blink/comp/unicode/");
    // println!(" HOST {}", std::env::var("HOST").unwrap());
    // println!("HHOST {}", std::env::var("HHOST").unwrap());
    println!("TARGET {}", std::env::var("TTARGET").unwrap().as_str());
    println!("HOST   {}", std::env::var("HHOST").unwrap().as_str());
    blink_css_build.target(std::env::var("TTARGET").unwrap().as_str());
    blink_css_build.host(std::env::var("HHOST").unwrap().as_str());
    blink_css_build.opt_level(2);
    blink_css_build.out_dir("./blink_css_out/");
    // blink_css_build.include("./blink/comp/Source/");
    // blink_css_build.include("./blink/comp/");
    // blink_css_build.include("./blink/comp/renderer/");
    // blink_css_build.include("/usr/include/c++/14/");
    blink_css_build.std("c++20");
    // blink_css_build.flag("-include");
    // blink_css_build.flag("./blink/comp/Source/config.h");
    // blink_css_build.flag("-include");
    // blink_css_build.flag("./blink/comp/Source/platform/PlatformExport.h");
    // blink_css_build.flag("-include");
    // blink_css_build.flag("./blink/comp/Source/wtf/HashTraits.h");
    // blink_css_build.flag("-include");
    // blink_css_build.flag("./blink/comp/platform/wtf/HashTable.h");
    // blink_css_build.flag("-Wno-unused-variable");
    // blink_css_build.flag("-Wno-unused-parameter");
    // blink_css_build.flag("-Wno-template-id-cdtor");
    // blink_css_build.flag("-Wno-register");
    // blink_css_build.flag("-Wno-class-memaccess");
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

/// Includes of a header or source file.
/// Each String is a substring of the file with the include.
/// e.g. "<cstddef>" or "core/wtf/HashTable.h"
struct DepsReprs {
    pub system: Vec<String>,
    pub project: Vec<String>,
}

fn gather_trans_deps(file: &Path, include_dirs: &[&str]) -> Deps {
    let mut res = Deps {
        system: Vec::new(),
        project: Vec::new(),
    };
    gather_deps_rec(include_dirs, file, &mut Vec::new(),&mut res, 0);
    res
}

fn gather_deps_rec(include_dirs: &[&str], file: &Path, handled_paths: &mut Vec<String>, trans_deps: &mut Deps, lvl: usize) {
    println!("GDR {}{}", "|   ".repeat(lvl), file.to_string_lossy());
    if handled_paths.contains(&file.to_string_lossy().to_string()) {
        return;
    }
    handled_paths.push(file.to_string_lossy().to_string());

    if file.ends_with(".h") {
        // recurse into .cpp includes
        let cpp_file_str = file.to_string_lossy().replace(".h", ".cc");
        let cpp_file = Path::new(&cpp_file_str);
        if cpp_file.exists() {
            gather_deps_rec(include_dirs, cpp_file, handled_paths, trans_deps, lvl);
        }
    }



    // recurse into header includes
    let direct_deps = gather_deps_repr(file);
    for dep in direct_deps.project.into_iter() {
        let include_rel_paths = include_dirs.iter().map(|dir| Path::new(dir).join(&dep));
        let file_rel_path = file.parent().unwrap().join(&dep);

        let mut found_path = None;
        if file_rel_path.exists() {
            found_path = Some(file_rel_path);
        } else {
            for include_rel_path in include_rel_paths {
                if include_rel_path.exists() {
                    found_path = Some(include_rel_path);
                    continue;
                }
            }
        }
        let dep_path = found_path.expect_red(&format!("Could not find include '{}' defined in file {:?}", dep, file));

        if !trans_deps.project.iter().any(|d| d.repr == dep) {
            trans_deps.project.push(Dep {
                repr: dep.clone(),
                path: dep_path.clone(),
            });
        }
        if !handled_paths.contains(&dep) {
            gather_deps_rec(include_dirs, &dep_path, handled_paths, trans_deps, lvl+1);
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
    let mut contents = std::fs::read_to_string(file).unwrap();

    for include in INCLUDES_TO_DELETE {
        contents = Regex::new(&("#include \"".to_owned() + include + "\""))
            .expect_red("include regex invalid")
            .replace_all(&contents, "")
            .to_string();
    }
    std::fs::write(file, &contents).expect_red("failed to write file after removing includes");

    let proj_deps_rgx = Regex::new(r#"(?m)^#include "(?<dep>[^"]*)"#).unwrap();

    let deps = proj_deps_rgx.captures_iter(contents.as_str());

    for dep in deps.into_iter() {
        for dep in dep.iter() {
            let dep = dep.unwrap();
            if dep.as_str().contains("#") || dep.as_str().contains("core/css/parser") {
                continue;
            }
            res_proj.push(dep.as_str().to_owned());
        }
    }

    let mut res_sys = Vec::new();
    let sys_deps_rgx = Regex::new(r#"(?m)^#include <(?<dep>[^>]*)>"#).unwrap();

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
    for entry in std::fs::read_dir(in_dir.path()).expect_red(&format!("{} was not a dir", in_dir.file_name().to_str().unwrap())) {
        let e = entry.unwrap();
        if e.file_type().unwrap().is_dir() {
            gather_files(&e, paths);
        } else {
            paths.push(e.path());
        }
    }
}
