use anyhow::Result;

/// Audio chunk with metadata for merging
#[derive(Debug)]
pub struct AudioChunk {
    pub data: Vec<u8>,
}

/// Chunk configuration based on provider
#[derive(Debug, Clone)]
pub struct ChunkConfig {
    pub max_size_bytes: usize,
    pub overlap_seconds: f32,
    pub sample_rate: u32,
}

impl ChunkConfig {
    pub fn for_openai() -> Self {
        Self {
            max_size_bytes: 24 * 1024 * 1024, // 24MB (under 25MB limit)
            overlap_seconds: 2.0,
            sample_rate: 16000,
        }
    }

    pub fn for_groq() -> Self {
        Self {
            max_size_bytes: 18 * 1024 * 1024, // 18MB (under 19MB limit)
            overlap_seconds: 2.0,
            sample_rate: 16000,
        }
    }

    pub fn for_assemblyai() -> Self {
        Self {
            max_size_bytes: usize::MAX, // No limit
            overlap_seconds: 0.0,
            sample_rate: 16000,
        }
    }
}

/// Split audio data into chunks for processing
pub fn split_audio(audio_data: &[u8], config: &ChunkConfig) -> Result<Vec<AudioChunk>> {
    let total_size = audio_data.len();

    // If under limit, return as single chunk
    if total_size <= config.max_size_bytes {
        return Ok(vec![AudioChunk {
            data: audio_data.to_vec(),
        }]);
    }

    // Calculate overlap in bytes
    // Assuming 16-bit mono audio at sample_rate
    let bytes_per_second = config.sample_rate * 2; // 16-bit = 2 bytes per sample
    let overlap_bytes = (config.overlap_seconds * bytes_per_second as f32) as usize;

    // Calculate chunk size (excluding overlap)
    let effective_chunk_size = config.max_size_bytes - overlap_bytes;

    let mut chunks = Vec::new();
    let mut offset = 0;

    while offset < total_size {
        let chunk_start = if offset > 0 {
            offset.saturating_sub(overlap_bytes / 2)
        } else {
            0
        };

        let chunk_end = (chunk_start + config.max_size_bytes).min(total_size);

        let chunk_data = audio_data[chunk_start..chunk_end].to_vec();

        chunks.push(AudioChunk { data: chunk_data });

        offset = chunk_start + effective_chunk_size;
    }

    Ok(chunks)
}

/// Merge transcripts from chunked audio, removing duplicate overlapping content
pub fn merge_transcripts(transcripts: &[String]) -> String {
    if transcripts.is_empty() {
        return String::new();
    }

    if transcripts.len() == 1 {
        return transcripts[0].clone();
    }

    let mut merged = transcripts[0].clone();

    for current in transcripts.iter().skip(1) {
        // Find overlap by looking for common sentence endings
        let overlap_removed = remove_overlap(&merged, current);
        merged.push(' ');
        merged.push_str(&overlap_removed);
    }

    // Clean up extra spaces
    merged.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Remove overlapping content between two transcript segments
fn remove_overlap(previous: &str, current: &str) -> String {
    let prev_words: Vec<&str> = previous.split_whitespace().collect();
    let curr_words: Vec<&str> = current.split_whitespace().collect();

    if prev_words.is_empty() || curr_words.is_empty() {
        return current.to_string();
    }

    // Look for overlap in the last N words of previous and first N words of current
    let max_overlap = 20.min(prev_words.len()).min(curr_words.len());

    let mut best_overlap = 0;

    for overlap_size in (3..=max_overlap).rev() {
        let prev_end = &prev_words[prev_words.len() - overlap_size..];
        let curr_start = &curr_words[..overlap_size];

        // Check for exact match
        if prev_end == curr_start {
            best_overlap = overlap_size;
            break;
        }

        // Check for fuzzy match (at least 80% words matching)
        let matching = prev_end
            .iter()
            .zip(curr_start.iter())
            .filter(|(a, b)| a.to_lowercase() == b.to_lowercase())
            .count();

        if matching as f32 / overlap_size as f32 >= 0.8 {
            best_overlap = overlap_size;
            break;
        }
    }

    if best_overlap > 0 {
        curr_words[best_overlap..].join(" ")
    } else {
        current.to_string()
    }
}

/// Read audio file and get duration in seconds

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_merge_transcripts_with_overlap() {
        let transcripts = vec![
            "Hello world, this is a test.".to_string(),
            "this is a test. And here is more content.".to_string(),
        ];

        let merged = merge_transcripts(&transcripts);
        assert!(merged.contains("Hello world"));
        assert!(merged.contains("more content"));
        // Should not have duplicate "this is a test"
        assert_eq!(merged.matches("this is a test").count(), 1);
    }
}
