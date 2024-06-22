use midir::{MidiInput, MidiInputConnection, MidiInputPort, MidiInputPorts};
use midly::{live::LiveEvent, num::u4, MidiMessage};

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

    pub fn connect<F>(&mut self, port: &MidiInputPort, mut callback: F)
    where
        F: FnMut(u4, MidiMessage) + Send + 'static,
    {
        let port_name = self.midi_in.port_name(port).unwrap();
        // have to make a new one because `connect` takes ownership for some reason
        let midi_input = MidiInput::new("Connection input (?)").unwrap();
        let conn = midi_input.connect(
            &port,
            "nebulizer-input-port",
            move |_stamp, msg_raw, _| {
                let event = LiveEvent::parse(msg_raw).unwrap();
                match event {
                    LiveEvent::Midi { channel, message } => {
                        callback(channel, message);
                    }
                    _ => {}
                }
            },
            (),
        );
        self.connection = conn.ok().map(|c| (port_name, c));
    }
}
