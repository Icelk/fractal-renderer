pub mod lib;
pub use lib::*;

fn main() {
    #[cfg(feature = "avif")]
    let options = lib::get_options();

    #[cfg(feature = "gui")]
    if options.gui {
        lib::gui::start(options);
        return;
    }

    #[cfg(feature = "avif")]
    {
        let contents = lib::get_image(&options.config);
        lib::write_image(&options, contents);
    }
    #[cfg(not(feature = "avif"))]
    {
        eprintln!("Failed to write file; avif feature isn't enabled.")
    }
}
