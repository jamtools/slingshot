mod bounce;
use bounce::{auto_bounce_garageband, check_garageband_available};

pub fn init<R: tauri::Runtime>() -> tauri::plugin::TauriPlugin<R> {
    tauri::plugin::Builder::new("slingshot")
        .invoke_handler(tauri::generate_handler![
            auto_bounce_garageband,
            check_garageband_available,
        ])
        .build()
}
