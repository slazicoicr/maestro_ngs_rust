use lazy_static;
use maestro_ngs_application::{self, SavedApplication};
use maestro_ngs_emulator;
use rocket;

use std::sync::Mutex;

lazy_static::lazy_static! {
    static ref ARRAY: Mutex<Option<SavedApplication>> = Mutex::new(None);
}

#[rocket::get("/count")]
fn count(hit_count: &rocket::State<&ARRAY>) -> String {
    format!(
        "This is request #{}.",
        hit_count.lock().unwrap().as_ref().unwrap().start_method()
    )
}

fn load_app() -> Result<(), std::io::Error> {
    let mut d = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    d.push("resources/test/Pipette_and_Mix.eap");
    let empty_app = std::fs::read_to_string(d)?;

    let app = maestro_ngs_application::Loader::new(&empty_app).build_application();
    let mut a = ARRAY.lock().unwrap();
    *a = Some(app);
    Ok(())
}

#[rocket::main]
async fn main() {
    match load_app() {
        Ok(_) => {}
        Err(e) => {
            eprintln!("error: {:?}", e);
            std::process::exit(1);
        }
    };

    rocket::build()
        .mount("/", rocket::routes![count])
        .manage(&ARRAY)
        .launch()
        .await
        .unwrap();
}
