
// use csslancer::css_language_types::{CssDataV1, CssDataV1Source};




pub fn main() {

    // let json_str = include_str!("./../data/WebData.json");
    // let now = std::time::Instant::now();
    // let json_value: CssDataV1Source = serde_json::from_str(json_str).unwrap(); // measured around 5ms on my pc in release mode 
    // let dur = now.elapsed();
    // println!("deserialization of json took {}µs", dur.as_micros());

    // serde_json::to_writer_pretty(std::fs::File::create("./csslancer/src/bin/jsonout.json").unwrap(), &json_value).unwrap();
    // bincode::serialize_into::<_, CssDataV1Source>(std::fs::File::create("./csslancer/src/bin/bincode.bin").unwrap(), &json_value.into()).unwrap();


    // let bincode_bytes = include_bytes!("bincode.bin");
    // let now = std::time::Instant::now();
    // let _bincode_value: CssDataV1 = bincode::deserialize::<'_, CssDataV1Source>(bincode_bytes).unwrap().into();
    // let dur = now.elapsed();
    // println!("deserialization of bincode took {}µs", dur.as_micros());



    // use csslancer::css_language_types::{ 
    //     CssDataVersion,
    //     PropertyData,
    //     AtDirectiveData,
    //     PseudoClassData,
    //     PseudoElementData,
    //     Content,
    //     EntryStatus,
    //     Reference,
    //     PropertyDataSource,
    //     AtDirectiveDataSource,
    // };

    // let uneval_path = concat!(env!("OUT_DIR"), "/web_data.rs");
    // println!("{uneval_path}");
    // //let uneval_path = "./csslancer/src/bin/uneval.uneval";
    // let json_value: CssDataV1Source = serde_json::from_str(json_str).unwrap();
    // //uneval::to_file(json_value, uneval_path).expect("Write failed");
    // let uneval_str = uneval::to_string(json_value).unwrap();
    // println!("unlen : {}", uneval_str.len());
    // let uneval_str = uneval_str.replace("].into_iter().collect()", "]");
    // let uneval_str = uneval_str.replace("AtDirectiveDataSource {", "AtDirectiveDataSource{");
    // let uneval_str = uneval_str.replace("{name: ", "{name:");
    // let uneval_str = uneval_str.replace(",description: ", ",description:");
    // let uneval_str = uneval_str.replace(",syntax: ", ",syntax:");
    // let uneval_str = uneval_str.replace(",status: ", ",status:");
    // let uneval_str = uneval_str.replace(",references: ", ",references:");

    // println!("unlen : {}", uneval_str.len());
    // std::fs::File::create(uneval_path).unwrap().write(uneval_str.as_bytes()).unwrap();
    // let now = std::time::Instant::now();
    //let val: CssDataV1Source = include!(concat!(env!("OUT_DIR"), "/web_data.rs"));
    // let dur = now.elapsed();
    // println!("deserialization from Rust took {}µs", dur.as_micros());

}