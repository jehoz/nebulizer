use std::error::Error;

use midir::MidiInput;

pub fn run() -> Result<(), Box<dyn Error>> {
    let midi_in = MidiInput::new("nebulizer midi input")?;

    let in_ports = midi_in.ports();
    let in_port = match in_ports.len() {
        0 => return Err("no input port found".into()),
        1 => {
            println!(
                "Choosing the only available input port: {}",
                midi_in.port_name(&in_ports[0]).unwrap()
            );
            &in_ports[0]
        }
        _ => {
            println!("\nAvailable input ports:");
            for (i, p) in in_ports.iter().enumerate() {
                println!("{}: {}", i, midi_in.port_name(p).unwrap());
            }
            println!(
                "Picking the first one: {}",
                midi_in.port_name(&in_ports[0]).unwrap()
            );
            &in_ports[0]
        }
    };

    let _conn = midi_in.connect(
        in_port,
        "nebulizer-input-port",
        move |stamp, msg, _| {
            println!("{}: {:?} (len = {})", stamp, msg, msg.len());
        },
        (),
    );

    loop {}
}
