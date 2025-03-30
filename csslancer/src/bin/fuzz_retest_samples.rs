

type FuzzSampleList = Vec<String>;

pub fn main () {
    
    let fuzz_samples = serde_json::from_reader::<std::fs::File, FuzzSampleList>(
        std::fs::File::open(std::path::Path::new(file!()).parent().unwrap().to_path_buf().join("fuzz_retest_samples.json")).unwrap()
    ).unwrap();

    let div = "#".repeat(44) + "\n";

    for sample in fuzz_samples {
        println!("{div}##### RETESTING FUZZ SAMPLE `{}` ##### \n{div}", sample);
        csslancer::row_parser::fuzz::retest_fuzz_error(&sample);
    }

}