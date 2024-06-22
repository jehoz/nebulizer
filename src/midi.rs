use midir::{MidiInput, MidiInputConnection, MidiInputPort, MidiInputPorts};
use midly::{live::LiveEvent, MidiMessage};

pub struct MidiConfig {
    pub midi_in: MidiInput,
    pub ports: MidiInputPorts,
    pub connection: Option<(String, MidiInputConnection<()>)>,
}

impl MidiConfig {
    pub fn new() -> MidiConfig {
        let midi_in = MidiInput::new("Nebulizer MIDI in").unwrap();
        let ports = midi_in.ports();

        MidiConfig {
            midi_in,
            ports,
            connection: None,
        }
    }

    pub fn connect(&mut self, port: &MidiInputPort) {
        let port_name = self.midi_in.port_name(port).unwrap();
        // have to make a new one because `connect` takes ownership for some reason
        let midi_input = MidiInput::new("Connection input (?)").unwrap();
        let conn = midi_input.connect(
            &port,
            "nebulizer-input-port",
            |_stamp, msg, _| handle_midi_message(msg),
            (),
        );
        self.connection = conn.ok().map(|c| (port_name, c));
    }
}

fn handle_midi_message(msg_raw: &[u8]) {
    let event = LiveEvent::parse(msg_raw).unwrap();
    match event {
        LiveEvent::Midi { channel, message } => match message {
            MidiMessage::NoteOn { key, .. } => {
                println!("CH{}: Note {} down", channel, key)
            }
            MidiMessage::NoteOff { key, .. } => {
                println!("CH{}: Note {} up", channel, key)
            }
            _ => {}
        },
        _ => {}
    }
}
