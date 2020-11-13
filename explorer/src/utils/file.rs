use std::{fs::File, io::Write, path::Path};

pub fn save<P: AsRef<Path>>(text: String, path: P) {
    let path = path.as_ref();
    let display = path.display();

    let mut file = match File::create(path) {
        Err(why) => panic!("couldn't open {}: {}", display, why),
        Ok(file) => file,
    };

    match file.write_all(text.as_bytes()) {
        Err(why) => panic!("couldn't write to {}: {}", display, why),
        Ok(_) => (),
    }
}
