use std::fs::File;
use std::path::Path;

/// Downloads all the champion icons for use in the app
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let champs: Vec<(String, u16)> =
        serde_json::from_reader(File::open("../model-trainer/champs.json")?)?;
    for (_, champ_id) in champs {
        if !Path::new(&format! {"champ_icons/{champ_id}.png"}).exists() {
            let mut resp = reqwest::blocking::get(format!(
                " https://cdn.communitydragon.org/latest/champion/{champ_id}/square"
            ))
            ?;
            if resp.status().is_success() {
                let mut file = File::create(format! {"champ_icons/{champ_id}.png"})?;
                resp.copy_to(&mut file)?;
            }
        }
    }
    if !Path::new("champ_icons/generic.png").exists() {
        let mut resp = reqwest::blocking::get(
            "https://cdn.communitydragon.org/latest/champion/generic/square",
        )
        ?;
        if resp.status().is_success() {
            let mut file = File::create("champ_icons/generic.png")?;
            resp.copy_to(&mut file)?;
        }
    }
    Ok(())
}
