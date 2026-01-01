extern crate winres;

fn main() {
    let svg_path = "src/MaterialSymbolsBatteryAndroidFrame1.svg";
    println!("cargo:rerun-if-changed={}", svg_path);

    let mut res = winres::WindowsResource::new();
    res.set_icon("src/bat/main.ico");

    for i in (10..=50).step_by(10) {
        res.set_icon_with_id(&format!("src/bat/battery{i}.ico"), &format!("{i}"));
        let charging_i = i + 1;
        res.set_icon_with_id(
            &format!("src/bat/battery{charging_i}.ico"),
            &format!("{charging_i}"),
        );
    }

    for i in (15..=55).step_by(10) {
        res.set_icon_with_id(&format!("src/bat/battery{i}.ico"), &format!("{i}"));
        let charging_i = i + 1;
        res.set_icon_with_id(
            &format!("src/bat/battery{charging_i}.ico"),
            &format!("{charging_i}"),
        );
    }

    res.compile().unwrap();
}
