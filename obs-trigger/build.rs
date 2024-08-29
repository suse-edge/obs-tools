fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut config = prost_build::Config::new();
    config.default_package_filename("gangway");
    tonic_build::configure()
        .build_server(false)
        .build_client(true)
        .out_dir("./src")
        .compile_with_config(config, &["proto/gangway.proto"], &["proto"])?;
    Ok(())
}
