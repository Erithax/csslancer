

// We fuzz 2 things
// ## 1. Reparsing vs. parsing
// We fuzz test that incremental reparsing generates the same syntax tree and
// errors as parsing. 
// # 2. Csslancer vs. Firefox
// We fuzz test that any css file which parses without errors in Firefox' 
// parser also parses without errors in our parser.

// # Integration testing
// Pull css files from repositories and top websites and parse them. Files with
// parsing errors are collected for review.


// pub fn collect_files() -> Vec<&str> {
    
// }

// pub fn generate_plausible_css() -> &str {
    
// }