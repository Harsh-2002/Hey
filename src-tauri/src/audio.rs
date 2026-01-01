use anyhow::Result;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::Stream;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

/// Audio device information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioDevice {
    pub name: String,
    pub is_default: bool,
}

/// Audio samples for waveform visualization
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct AudioLevel {
    pub peak: f32,
    pub rms: f32,
}

pub struct AudioRecorder {
    samples: Arc<Mutex<Vec<f32>>>,
    sample_rate: Arc<Mutex<u32>>,
    is_recording: Arc<Mutex<bool>>,
    stream: Mutex<Option<Stream>>,
    selected_device: Arc<Mutex<Option<String>>>,
    audio_levels: Arc<Mutex<Vec<AudioLevel>>>,
}

// SAFETY: We only access the stream from the main thread via Mutex
unsafe impl Send for AudioRecorder {}
unsafe impl Sync for AudioRecorder {}

impl AudioRecorder {
    pub fn new() -> Result<Self> {
        Ok(Self {
            samples: Arc::new(Mutex::new(Vec::new())),
            sample_rate: Arc::new(Mutex::new(16000)), // Default to 16kHz for Whisper
            is_recording: Arc::new(Mutex::new(false)),
            stream: Mutex::new(None),
            selected_device: Arc::new(Mutex::new(None)),
            audio_levels: Arc::new(Mutex::new(Vec::new())),
        })
    }

    /// List available audio input devices
    pub fn list_devices(&self) -> Result<Vec<AudioDevice>> {
        let host = cpal::default_host();
        let default_device = host.default_input_device();
        let default_name = default_device.as_ref().and_then(|d| d.name().ok());

        let mut devices = Vec::new();

        for device in host.input_devices()? {
            if let Ok(name) = device.name() {
                devices.push(AudioDevice {
                    name: name.clone(),
                    is_default: Some(&name) == default_name.as_ref(),
                });
            }
        }

        Ok(devices)
    }

    /// Set the audio input device by name
    pub fn set_device(&self, device_name: Option<String>) {
        let mut selected = self.selected_device.lock().unwrap();
        *selected = device_name;
    }

    /// Get the currently selected device
    fn get_device(&self) -> Result<cpal::Device> {
        let host = cpal::default_host();
        let selected = self.selected_device.lock().unwrap();

        if let Some(ref name) = *selected {
            // Find the device by name
            for device in host.input_devices()? {
                if let Ok(device_name) = device.name() {
                    if &device_name == name {
                        return Ok(device);
                    }
                }
            }
        }

        // Fall back to default device
        host.default_input_device()
            .ok_or_else(|| anyhow::anyhow!("No input device available"))
    }

    pub fn start_recording(&self) -> Result<()> {
        let device = self.get_device()?;

        // Use the device's default input config for maximum compatibility
        // Attempting to force 16kHz on macOS sometimes leads to sample rate mismatches
        // (capturing at 48k but tagging as 16k, causing slow-motion audio)
        let config = device.default_input_config()?;

        let actual_sample_rate = config.sample_rate().0;
        let channels = config.channels();

        println!(
            "[Audio] Starting recording at {}Hz, {} channels",
            actual_sample_rate, channels
        );

        // Store sample rate for later
        {
            let mut sr = self.sample_rate.lock().unwrap();
            *sr = actual_sample_rate;
        }

        // Clear previous samples and levels
        {
            let mut s = self.samples.lock().unwrap();
            s.clear();
        }
        {
            let mut l = self.audio_levels.lock().unwrap();
            l.clear();
        }

        // Set recording flag
        {
            let mut r = self.is_recording.lock().unwrap();
            *r = true;
        }

        let samples = Arc::clone(&self.samples);
        let is_recording = Arc::clone(&self.is_recording);
        let audio_levels = Arc::clone(&self.audio_levels);

        // Buffer for level calculation (every 50ms worth of samples)
        let level_buffer_size = (actual_sample_rate as f32 * 0.05) as usize;
        let mut level_buffer = Vec::with_capacity(level_buffer_size);

        let stream = device.build_input_stream(
            &config.into(),
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                let recording = *is_recording.lock().unwrap();
                if recording {
                    let mut s = samples.lock().unwrap();

                    // Convert to mono if stereo
                    if channels > 1 {
                        for chunk in data.chunks(channels as usize) {
                            let mono = chunk.iter().sum::<f32>() / channels as f32;
                            s.push(mono);
                            level_buffer.push(mono);
                        }
                    } else {
                        s.extend_from_slice(data);
                        level_buffer.extend_from_slice(data);
                    }

                    // Calculate audio levels for waveform
                    if level_buffer.len() >= level_buffer_size {
                        let peak = level_buffer
                            .iter()
                            .map(|s| (s * 5.0).abs()) // Amplify for visualization
                            .fold(0.0f32, |a, b| a.max(b));

                        let sum_sq: f32 = level_buffer.iter().map(|s| s * s).sum();
                        let rms = (sum_sq / level_buffer.len() as f32).sqrt();

                        let mut l = audio_levels.lock().unwrap();
                        l.push(AudioLevel { peak, rms });

                        // Keep only last 100 levels (~5 seconds)
                        if l.len() > 100 {
                            l.remove(0);
                        }

                        level_buffer.clear();
                    }
                }
            },
            |err| eprintln!("Audio stream error: {:?}", err),
            None,
        )?;

        stream.play()?;

        // Store the stream to keep it alive
        {
            let mut stream_guard = self.stream.lock().unwrap();
            *stream_guard = Some(stream);
        }

        Ok(())
    }

    pub fn stop_recording(&self) -> Result<Vec<u8>> {
        // Stop recording
        {
            let mut r = self.is_recording.lock().unwrap();
            *r = false;
        }

        // Drop the stream to stop it
        {
            let mut stream_guard = self.stream.lock().unwrap();
            *stream_guard = None;
        }

        // Small delay to ensure stream buffer is flushed
        std::thread::sleep(std::time::Duration::from_millis(50));

        // Get samples and encode to WAV
        let samples = self.samples.lock().unwrap();
        let sample_rate = *self.sample_rate.lock().unwrap();

        if samples.is_empty() {
            return Err(anyhow::anyhow!("No audio recorded"));
        }

        let duration_secs = samples.len() as f32 / sample_rate as f32;
        println!(
            "[Audio] Stopping recording. Samples: {}, Rate: {}, Calculated Duration: {:.2}s",
            samples.len(),
            sample_rate,
            duration_secs
        );

        // Resample to 16kHz for Whisper compatibility and smaller size
        let target_rate = 16000;
        let resampled = self.resample(&samples, sample_rate, target_rate);
        println!(
            "[Audio] Resampled to {}Hz. New sample count: {}",
            target_rate,
            resampled.len()
        );

        let wav_data = self.encode_wav(&resampled, target_rate)?;
        Ok(wav_data)
    }

    fn resample(&self, samples: &[f32], from_rate: u32, to_rate: u32) -> Vec<f32> {
        if from_rate == to_rate {
            return samples.to_vec();
        }

        let ratio = from_rate as f32 / to_rate as f32;
        let new_len = (samples.len() as f32 / ratio).ceil() as usize;
        let mut new_samples = Vec::with_capacity(new_len);

        for i in 0..new_len {
            let src_idx = i as f32 * ratio;
            let idx_floor = src_idx.floor() as usize;
            let idx_ceil = (idx_floor + 1).min(samples.len() - 1);
            let weight = src_idx - idx_floor as f32;

            let s1 = samples[idx_floor];
            let s2 = samples[idx_ceil];

            // Linear interpolation
            let sample = s1 * (1.0 - weight) + s2 * weight;
            new_samples.push(sample);
        }

        new_samples
    }

    fn encode_wav(&self, samples: &[f32], sample_rate: u32) -> Result<Vec<u8>> {
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };

        let mut cursor = std::io::Cursor::new(Vec::new());
        {
            let mut writer = hound::WavWriter::new(&mut cursor, spec)?;
            for &sample in samples {
                let amplitude = (sample.clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
                writer.write_sample(amplitude)?;
            }
            writer.finalize()?;
        }

        Ok(cursor.into_inner())
    }

    pub fn is_recording(&self) -> bool {
        *self.is_recording.lock().unwrap()
    }

    /// Get current audio levels for waveform visualization
    pub fn get_audio_levels(&self) -> Vec<AudioLevel> {
        let levels = self.audio_levels.lock().unwrap();
        levels.clone()
    }

    /// Get the recording duration in seconds
    pub fn get_duration(&self) -> f32 {
        let samples = self.samples.lock().unwrap();
        let sample_rate = *self.sample_rate.lock().unwrap();
        samples.len() as f32 / sample_rate as f32
    }
}

impl Default for AudioRecorder {
    fn default() -> Self {
        Self::new().unwrap()
    }
}
