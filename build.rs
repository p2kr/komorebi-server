fn main() {
    println!("cargo:rerun-if-changed=.env");

    // Loads the .env file and passes variables to your app
    dotenv_build::output(dotenv_build::Config::default()).unwrap();
}
