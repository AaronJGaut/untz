use std::fs::File;
use std::io;
use std::io::prelude::*;
use std::f64::consts::PI;

enum Instrument {
    Sine,
    Square,
    Saw,
}

struct Note {
    freq: f64,
    volume: f64,
    start: f64,
    duration: f64,
}

struct Track {
    notes: Vec<Note>,
    instrument: Instrument,
}

struct Song {
    tracks: Vec<Track>,
}

enum Format {
    Wave,
}

struct WriteInfo {
    filepath: String,
    sample_rate: u32,
    stereo: bool,
    format: Format,
}

fn overwrite<T>(_curr: T, new: T) -> T {
    new
}

fn add<T: std::ops::Add<Output = T>>(curr: T, new: T) -> T {
    curr + new
}

fn grab<'a, T>(vec: &'a mut Vec<T>, offset: &mut usize, len: usize) -> &'a mut [T] {
    let start: usize = *offset;
    *offset += len;
    return &mut vec[start..*offset];
}

fn merge<T: Copy>(dst: &mut [T], src: &[T], merge_fn: fn(T, T) -> T) {
    if dst.len() != src.len() {
        panic!("Mismatched length!");
    }
    for i in 0..dst.len() {
        dst[i] = merge_fn(dst[i], src[i]);
    }
}

impl Track {
    fn new(instrument: Instrument) -> Track {
        Track {
            instrument: instrument,
            notes: Vec::new(),
        }
    }

    fn note(&mut self, note: Note) {
        self.notes.push(note);
    }
}

impl Song {
    fn new() -> Song {
        Song {
            tracks: Vec::new(),
        }
    }

    fn track(&mut self, track: Track) {
        self.tracks.push(track);
    }

    fn write(&self, info: &WriteInfo) -> Result<(), io::Error> {
        let mut file = File::create(&info.filepath)?;
        let mut total_length = 0_f64;
        for track in self.tracks.iter() {
            for note in track.notes.iter() {
                let end_time = note.start + note.duration;
                if end_time > total_length {
                    total_length = end_time;
                }
            }
        }
        // Computing byte sizes
        let num_samples = (total_length * info.sample_rate as f64).ceil() as u32;
        let num_channels = if info.stereo {2_u16} else {1_u16};
        let sample_bytes = 2_u16;
        let data_size = num_samples * num_channels as u32 * sample_bytes as u32;
        let pad_size = if data_size % 2 == 0 {0_u32} else {1_u32};
        let wave_chunk_size: u32 = 36 + data_size + pad_size;
        let file_size = (wave_chunk_size + 8) as usize;
        let byte_rate: u32 = info.sample_rate * sample_bytes as u32 * num_channels as u32;
        let block_align: u16 = sample_bytes * num_channels;
        let sample_bits: u16 = 8_u16 * sample_bytes;

        let mut file_data = vec![0_u8; file_size];
        let mut i = 0;

        let mut write_slice = |slice: &[u8]| {
            merge(&mut file_data[i..i+slice.len()], slice, overwrite);
            i += slice.len();
        };

        write_slice(b"RIFF");
        write_slice(&wave_chunk_size.to_le_bytes());
        write_slice(b"WAVE");

        write_slice(b"fmt ");
        write_slice(&(16_u32).to_le_bytes());
        write_slice(b"\x01\x00");
        write_slice(&num_channels.to_le_bytes());
        write_slice(&info.sample_rate.to_le_bytes());
        write_slice(&byte_rate.to_le_bytes());
        write_slice(&block_align.to_le_bytes());
        write_slice(&sample_bits.to_le_bytes());
        write_slice(b"data");
        write_slice(&data_size.to_le_bytes());
        // Add sample data

        let mut sample_data = vec![0_f64; num_samples as usize];
        for track in self.tracks.iter() {
            for note in track.notes.iter() {
                let mut note_samples = vec![0_f64; (note.duration * info.sample_rate as f64) as usize];
                for item in note_samples.iter_mut().enumerate() {
                    let t = item.0 as f64 / info.sample_rate as f64;
                    *item.1 = note.volume * match &track.instrument {
                        Instrument::Sine => {
                            f64::sin(t * 2.0 * PI * note.freq)
                        },
                        Instrument::Square => {
                            if f64::floor(t * 2.0 * note.freq) as u32 % 2 == 0 {1.0} else {-1.0}
                        },
                        Instrument::Saw => {
                            2.0 * f64::fract(t * note.freq) - 1.0
                        },
                    };
                }
                let start_idx = (note.start * info.sample_rate as f64) as usize;
                merge(&mut sample_data[start_idx..start_idx+note_samples.len()], &note_samples, add);
            }
        }

        let sample_max = 32767_f64;  // 2 ** (2 * 8) / 2 - 1
        for sample in sample_data {
            let val = (sample_max * sample.clamp(-1.0, 1.0)).floor() as i16;
            let val_bytes = val.to_le_bytes();
            for _ in 0..num_channels {
                write_slice(&val_bytes);
            }
        }

        file.write(&file_data);

        Ok(())
    }
}

fn main() {
    let mut track1 = Track::new(Instrument::Sine);
    track1.note(Note {
        freq: 440.0,
        duration: 1.0,
        start: 0.0,
        volume: 0.8,
    });
    let mut track2 = Track::new(Instrument::Square);
    track2.note(Note {
        freq: 440.0,
        duration: 1.0,
        start: 0.0,
        volume: 0.2,
    });
    let mut track3 = Track::new(Instrument::Saw);
    track3.note(Note {
        freq: 440.0,
        duration: 1.0,
        start: 0.0,
        volume: 0.3,
    });

    let mut song = Song::new();
    song.track(track1);
    //song.track(track2);
    //song.track(track3);
    song.write(
        &WriteInfo {
            filepath: String::from("test.wav"),
            sample_rate: 44100,
            stereo: false,
            format: Format::Wave,
        }
    );
}
