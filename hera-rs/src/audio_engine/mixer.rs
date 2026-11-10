//! Fractional-step linear-interpolating stereo deck mixer for [`DualDeckEngine`].

use super::DeckState;

#[inline]
fn accumulate_stereo(out: &mut [f32], out_frame: usize, l: f32, r: f32, gain: f32) {
    let base = out_frame * 2;
    if l.is_finite() {
        out[base] += l * gain;
    }
    if r.is_finite() {
        out[base + 1] += r * gain;
    }
}

pub(super) fn mix_deck_into_output(deck: &mut DeckState, out_stereo: &mut [f32], frames: usize) {
    if !deck.playing {
        return;
    }
    let Some(buf) = deck.buffer.as_deref().filter(|b| b.total_frames() > 0) else {
        deck.playing = false;
        return;
    };
    let total_frames = buf.total_frames();
    let samples = buf.interleaved_stereo();
    let gain = deck.gain.clamp(0.0, 2.0);
    let ratio = deck.tempo_ratio.clamp(0.5, 2.0);

    if (ratio - 1.0).abs() <= f32::EPSILON && deck.playhead_frames.fract() == 0.0 {
        let mut idx = deck.playhead_frames.max(0.0) as u64;
        for out_frame in 0..frames {
            if idx >= total_frames {
                deck.playhead_frames = total_frames as f64;
                deck.playing = false;
                return;
            }
            let base = (idx as usize) * 2;
            let (l, r) = (samples[base], samples[base + 1]);
            accumulate_stereo(out_stereo, out_frame, l, r, gain);
            idx += 1;
        }
        deck.playhead_frames = idx as f64;
        deck.playing = idx < total_frames;
        return;
    }

    let step = f64::from(ratio);
    let total_f64 = total_frames as f64;
    for out_frame in 0..frames {
        let idx0 = if deck.playhead_frames.is_finite() && deck.playhead_frames < total_f64 {
            deck.playhead_frames.max(0.0).floor() as u64
        } else {
            total_frames
        };
        if idx0 >= total_frames {
            deck.playhead_frames = total_f64;
            deck.playing = false;
            return;
        }
        let frac = (deck.playhead_frames - (idx0 as f64)) as f32;
        let idx1 = (idx0 + 1).min(total_frames - 1);
        let (b0, b1) = ((idx0 as usize) * 2, (idx1 as usize) * 2);
        let l = samples[b0] + (samples[b1] - samples[b0]) * frac;
        let r = samples[b0 + 1] + (samples[b1 + 1] - samples[b0 + 1]) * frac;
        accumulate_stereo(out_stereo, out_frame, l, r, gain);
        deck.playhead_frames += step;
    }

    if deck.playhead_frames >= total_f64 {
        deck.playhead_frames = total_f64;
        deck.playing = false;
    }
}
