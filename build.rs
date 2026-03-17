fn main() {
    // Compile GSettings schemas so `cargo run` works without manual steps.
    // The compiled schema file (gschemas.compiled) is placed in data/.
    let status = std::process::Command::new("glib-compile-schemas")
        .arg("data")
        .status();

    match status {
        Ok(s) if s.success() => {}
        Ok(s) => {
            eprintln!(
                "Warning: glib-compile-schemas exited with status {}. \
                 GSettings will not work at runtime.",
                s
            );
        }
        Err(e) => {
            eprintln!(
                "Warning: could not run glib-compile-schemas: {}. \
                 Install glib-2.0 development tools for GSettings support.",
                e
            );
        }
    }

    println!("cargo:rerun-if-changed=data/de.chords.Chords.gschema.xml");
}
