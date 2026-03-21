use sample_project::Config;

fn main() {
    let config = Config::default();
    println!("Starting {} v{}", config.name, config.version);

    let items = vec!["alpha", "beta", "gamma"];
    for item in &items {
        println!("Processing: {}", item);
    }

    println!("Done! Processed {} items.", items.len());
}
