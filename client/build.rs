use std::fs::File;
use std::path::Path;

/// Downloads all the champion icons for use in the app
fn main() {
    let champs: Vec<(String, u16)> =
        serde_json::from_reader(File::open("../model-trainer/champs.json").unwrap()).unwrap();
    for (_, champ_id) in champs {
        if !Path::new(&format! {"champ_icons/{champ_id}.png"}).exists() {
            let mut resp = reqwest::blocking::get(format!(
                " https://cdn.communitydragon.org/latest/champion/{champ_id}/square"
            ))
            .unwrap();
            if resp.status().is_success() {
                let mut file = File::create(format! {"champ_icons/{champ_id}.png"}).unwrap();
                resp.copy_to(&mut file).unwrap();
            }
        }
    }
    if !Path::new("champ_icons/generic.png").exists() {
        let mut resp = reqwest::blocking::get(
            "https://cdn.communitydragon.org/latest/champion/generic/square",
        )
        .unwrap();
        if resp.status().is_success() {
            let mut file = File::create("champ_icons/generic.png").unwrap();
            resp.copy_to(&mut file).unwrap();
        }
    }
}
