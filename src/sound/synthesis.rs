use super::spec::{SAMPLE_RATE, SoundSpec, Tone};

pub(super) fn generate_samples(spec: &SoundSpec) -> Vec<f32> {
    let mut samples = Vec::new();
    for (idx, tone) in spec.tones.iter().enumerate() {
        if idx > 0 && spec.gap_ms > 0 {
            let gap_samples = (SAMPLE_RATE as u64 * spec.gap_ms / 1000) as usize;
            samples.extend(std::iter::repeat_n(0.0, gap_samples));
        }
        samples.extend(generate_tone(*tone));
    }
    samples
}

fn generate_tone(tone: Tone) -> Vec<f32> {
    let num_samples = (SAMPLE_RATE as u64 * tone.duration_ms / 1000) as usize;
    let mut samples = Vec::with_capacity(num_samples);

    for i in 0..num_samples {
        let t = i as f32 / SAMPLE_RATE as f32;
        let progress = i as f32 / num_samples as f32;
        let envelope = 1.0 - progress;
        let sample =
            tone.amplitude * envelope * (2.0 * std::f32::consts::PI * tone.freq_hz * t).sin();
        samples.push(sample.clamp(-1.0, 1.0));
    }

    samples
}
