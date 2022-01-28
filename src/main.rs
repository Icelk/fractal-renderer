pub mod lib;
pub use lib::*;

fn main() {
    #[cfg(feature = "avif")]
    let config = lib::get_config();

    #[cfg(feature = "gui")]
    if config.gui {
        lib::gui::start(config);
        return;
    }

    #[cfg(feature = "avif")]
    {
        let contents = lib::get_image(&config);
        lib::write_image(&config, contents);
    }
    #[cfg(not(feature = "avif"))]
    {
        eprintln!("Failed to write file; avif feature isn't enabled.")
    }
}
