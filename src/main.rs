pub mod lib;
pub use lib::*;

use lib::{get_config, get_image,  write_image};

fn main() {
    let config = get_config();

    #[cfg(feature = "gui")]
    if config.gui {
        lib::gui::start(config);
        return;
    }

    let contents = get_image(&config);
    write_image(&config, contents);
}
