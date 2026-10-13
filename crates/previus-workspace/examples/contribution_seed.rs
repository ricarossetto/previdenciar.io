//! Create an isolated, encrypted workspace with fictional data for native UI checks.
use previus_workspace::{Calculation,Workspace};
fn main(){
    let path=std::path::PathBuf::from(std::env::args().nth(1).expect("Explicit new synthetic workspace path required"));
    assert!(!path.exists(),"Never overwrite an existing workspace");
    std::fs::create_dir_all(path.parent().expect("Parent directory")).expect("Create test profile");
    let cnis=previus_cnis::import_text("CNIS\nCompetência Remuneração Indicadores\n01/2020 100,10\n01/2020 200,20").expect("Synthetic CNIS");
    let calculation=Calculation{id:1,name:"Livro fictício - teste nativo isolado".into(),kind:"RGPS".into(),cnis:Some(cnis),..Default::default()};
    let mut workspace=Workspace{next_id:2,calculations:vec![calculation],..Default::default()};
    workspace.save(&path).expect("Encrypted synthetic workspace");
    println!("Synthetic encrypted workspace created; no user data read.");
}
