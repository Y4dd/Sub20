// Mixing every sound track into one in Sub20 Lite, the way the desktop app
// does it (`command::audio_input` in the engine): each track made stereo (a
// mono microphone lands in the middle), all summed at full volume, then a
// limiter for the peaks the sum pushes over. OBS and the NVIDIA app can
// record the microphone on a track of its own, and keeping only the first
// track lost it.
//
// The first track goes through Mediabunny's conversion, which hands each of
// its samples to `process` already stereo at `RATE`; the other tracks are
// decoded alongside, converted here, and added in.

import { AudioSample, AudioSampleSink, type InputAudioTrack } from "mediabunny";

/** The mix's sample rate. */
export const RATE = 48000;
/** The limiter's ceiling, as the desktop's `alimiter=limit=0.95`. */
const LIMIT = 0.95;
/** How fast the limiter lets go after a peak. */
const RELEASE_SECONDS = 0.05;

/** One extra track's sound, made stereo at `RATE`, read ahead as the mix needs it. */
class Feed {
  #samples: AsyncGenerator<AudioSample, void, unknown>;
  #start: number;
  #rate = 0;
  /** Decoded frames at the track's own rate, not used yet. */
  #left: number[] = [];
  #right: number[] = [];
  /** Where the next output frame falls in `#left`/`#right`, in the track's own frames. */
  #position = 0;
  #ended = false;

  constructor(track: InputAudioTrack, start: number, end: number) {
    this.#samples = new AudioSampleSink(track).samples(start, end);
    this.#start = start;
  }

  /** Adds the next `count` frames to `left` and `right`; silence once the track ends. */
  async addTo(left: Float32Array, right: Float32Array) {
    const count = left.length;
    for (let i = 0; i < count; i++) {
      const at = this.#position;
      const whole = Math.floor(at);
      while (whole + 1 >= this.#left.length && !this.#ended) await this.#read();
      if (whole >= this.#left.length) break; // The track is over.
      const next = Math.min(whole + 1, this.#left.length - 1);
      const t = at - whole;
      left[i] += this.#left[whole] * (1 - t) + this.#left[next] * t;
      right[i] += this.#right[whole] * (1 - t) + this.#right[next] * t;
      this.#position += this.#rate / RATE;
    }
    // Drop what is behind us, keeping the frame the next read interpolates from.
    const used = Math.max(0, Math.floor(this.#position) - 1);
    if (used > 4096) {
      this.#left.splice(0, used);
      this.#right.splice(0, used);
      this.#position -= used;
    }
  }

  async #read() {
    const next = await this.#samples.next();
    if (next.done) {
      this.#ended = true;
      return;
    }
    const sample = next.value;
    try {
      if (!this.#rate) this.#rate = sample.sampleRate;
      const frames = sample.numberOfFrames;
      const plane = (index: number) => {
        const data = new Float32Array(frames);
        sample.copyTo(data, { planeIndex: index, format: "f32-planar" });
        return data;
      };
      const left = plane(0);
      const right = sample.numberOfChannels > 1 ? plane(1) : left;
      // The first sample can begin before the trim's start.
      const skip = Math.max(0, Math.min(frames, Math.round((this.#start - sample.timestamp) * sample.sampleRate)));
      for (let i = skip; i < frames; i++) {
        this.#left.push(left[i]);
        this.#right.push(right[i]);
      }
    } finally {
      sample.close();
    }
  }

  async close() {
    await this.#samples.return();
  }
}

/** Sums the other tracks into the first, sample by sample, with a limiter after. */
export class Mixer {
  #feeds: Feed[];
  #gain = 1;
  #release = 1 - Math.exp(-1 / (RELEASE_SECONDS * RATE));

  /** `others` are every audio track but the one the conversion carries. */
  constructor(others: InputAudioTrack[], start: number, end: number) {
    this.#feeds = others.map((track) => new Feed(track, start, end));
  }

  /** For Mediabunny's `process`: the first track's stereo sample at `RATE`, with the others added. */
  process = async (sample: AudioSample): Promise<AudioSample> => {
    const frames = sample.numberOfFrames;
    const data = new Float32Array(frames * 2);
    const left = data.subarray(0, frames);
    const right = data.subarray(frames);
    sample.copyTo(left, { planeIndex: 0, format: "f32-planar" });
    if (sample.numberOfChannels > 1) sample.copyTo(right, { planeIndex: 1, format: "f32-planar" });
    else right.set(left);
    for (const feed of this.#feeds) await feed.addTo(left, right);
    this.#limit(left, right);
    return new AudioSample({ data, format: "f32-planar", numberOfChannels: 2, sampleRate: sample.sampleRate, timestamp: sample.timestamp });
  };

  /** Pulls the gain down at once when a peak would pass `LIMIT`, and lets it back up slowly. */
  #limit(left: Float32Array, right: Float32Array) {
    let gain = this.#gain;
    for (let i = 0; i < left.length; i++) {
      const peak = Math.max(Math.abs(left[i]), Math.abs(right[i]));
      if (peak * gain > LIMIT) gain = LIMIT / peak;
      left[i] *= gain;
      right[i] *= gain;
      gain += (1 - gain) * this.#release;
    }
    this.#gain = gain;
  }

  async close() {
    await Promise.all(this.#feeds.map((feed) => feed.close()));
  }
}
