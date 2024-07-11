use std::sync::Arc;

use nih_plug::prelude::*;
use nih_plug_egui::{create_egui_editor, egui, widgets, EguiState};

pub struct Nebulizer {
    params: Arc<NebParams>,
}

#[derive(Params)]
pub struct NebParams {
    #[persist = "editor_state"]
    editor_state: Arc<EguiState>,

    #[id = "level"]
    pub level: FloatParam,
}

impl Default for Nebulizer {
    fn default() -> Self {
        Self {
            params: Arc::new(NebParams::default()),
        }
    }
}

impl Default for NebParams {
    fn default() -> Self {
        Self {
            editor_state: EguiState::from_size(300, 180),

            level: FloatParam::new("Level", 1.0, FloatRange::Linear { min: 0.0, max: 1.0 }),
        }
    }
}

impl Plugin for Nebulizer {
    const NAME: &'static str = "Nebulizer";

    const VENDOR: &'static str = "";

    const URL: &'static str = "https://github.com/jehoz/nebulizer";

    const EMAIL: &'static str = "";

    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[
        AudioIOLayout {
            main_input_channels: NonZeroU32::new(2),
            main_output_channels: NonZeroU32::new(2),
            ..AudioIOLayout::const_default()
        },
        AudioIOLayout {
            main_input_channels: NonZeroU32::new(1),
            main_output_channels: NonZeroU32::new(1),
            ..AudioIOLayout::const_default()
        },
    ];

    type SysExMessage = ();

    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn editor(&mut self, async_executor: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        let params = self.params.clone();
        create_egui_editor(
            self.params.editor_state.clone(),
            (),
            |_, _| {},
            move |egui_ctx, setter, _state| {
                egui::CentralPanel::default().show(egui_ctx, |ui| {
                    ui.label("Is this working?");
                    ui.add(widgets::ParamSlider::for_param(&params.level, setter));
                });
            },
        )
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        aux: &mut AuxiliaryBuffers,
        context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        for channel_samples in buffer.iter_samples() {
            for sample in channel_samples {
                *sample = 0.0;
            }
        }
        ProcessStatus::Normal
    }
}
