//! Explicit, read-only hardware smoke check; never run by automated tests.
fn main() {
    let Some(host) = std::env::args().nth(1) else {
        eprintln!("Usage: cargo run -p canvas-pilot --example read_midra -- DEVICE_IP");
        std::process::exit(2);
    };
    match canvas_pilot::device::fetch(&host) {
        Ok(snapshot) => {
            println!("{}", snapshot.identity);
            for screen in snapshot.screens {
                println!(
                    "Screen {}: {} × {}, Preview {}, {} layer slots",
                    screen.id,
                    screen.resolution[0],
                    screen.resolution[1],
                    screen.preview_bank,
                    screen.layers.len()
                );
            }
        }
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(1);
        }
    }
}
