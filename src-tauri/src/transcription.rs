use anyhow::Result;
use reqwest::multipart;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TranscriptionProvider {
    OpenAI,
    Groq,
    AssemblyAI,
}

#[derive(Debug, Deserialize)]
struct OpenAIResponse {
    text: String,
}

#[derive(Debug, Deserialize)]
struct GroqResponse {
    text: String,
}

#[derive(Debug, Deserialize)]
struct AssemblyAIUploadResponse {
    upload_url: String,
}

#[derive(Debug, Deserialize)]
struct AssemblyAITranscriptResponse {
    id: String,
    status: String,
    text: Option<String>,
}

pub async fn transcribe(
    audio_data: Vec<u8>,
    provider: TranscriptionProvider,
    api_key: &str,
) -> Result<String> {
    match provider {
        TranscriptionProvider::OpenAI => transcribe_openai(audio_data, api_key).await,
        TranscriptionProvider::Groq => transcribe_groq(audio_data, api_key).await,
        TranscriptionProvider::AssemblyAI => transcribe_assemblyai(audio_data, api_key).await,
    }
}

async fn transcribe_openai(audio_data: Vec<u8>, api_key: &str) -> Result<String> {
    let client = reqwest::Client::new();

    let part = multipart::Part::bytes(audio_data)
        .file_name("audio.wav")
        .mime_str("audio/wav")?;

    let form = multipart::Form::new()
        .text("model", "whisper-1")
        .part("file", part);

    let response = client
        .post("https://api.openai.com/v1/audio/transcriptions")
        .header("Authorization", format!("Bearer {}", api_key))
        .multipart(form)
        .send()
        .await?;

    if !response.status().is_success() {
        let error = response.text().await?;
        return Err(anyhow::anyhow!("OpenAI API error: {}", error));
    }

    let result: OpenAIResponse = response.json().await?;
    Ok(result.text)
}

async fn transcribe_groq(audio_data: Vec<u8>, api_key: &str) -> Result<String> {
    let client = reqwest::Client::new();

    let part = multipart::Part::bytes(audio_data)
        .file_name("audio.wav")
        .mime_str("audio/wav")?;

    let form = multipart::Form::new()
        .text("model", "whisper-large-v3")
        .part("file", part);

    let response = client
        .post("https://api.groq.com/openai/v1/audio/transcriptions")
        .header("Authorization", format!("Bearer {}", api_key))
        .multipart(form)
        .send()
        .await?;

    if !response.status().is_success() {
        let error = response.text().await?;
        return Err(anyhow::anyhow!("Groq API error: {}", error));
    }

    let result: GroqResponse = response.json().await?;
    Ok(result.text)
}

async fn transcribe_assemblyai(audio_data: Vec<u8>, api_key: &str) -> Result<String> {
    let client = reqwest::Client::new();

    // Step 1: Upload the audio file
    let upload_response = client
        .post("https://api.assemblyai.com/v2/upload")
        .header("authorization", api_key)
        .header("content-type", "application/octet-stream")
        .body(audio_data)
        .send()
        .await?;

    if !upload_response.status().is_success() {
        let error = upload_response.text().await?;
        return Err(anyhow::anyhow!("AssemblyAI upload error: {}", error));
    }

    let upload_result: AssemblyAIUploadResponse = upload_response.json().await?;

    // Step 2: Create transcription request
    let transcript_response = client
        .post("https://api.assemblyai.com/v2/transcript")
        .header("authorization", api_key)
        .header("content-type", "application/json")
        .json(&serde_json::json!({
            "audio_url": upload_result.upload_url
        }))
        .send()
        .await?;

    if !transcript_response.status().is_success() {
        let error = transcript_response.text().await?;
        return Err(anyhow::anyhow!("AssemblyAI transcript error: {}", error));
    }

    let transcript_result: AssemblyAITranscriptResponse = transcript_response.json().await?;

    // Step 3: Poll for completion
    let transcript_id = transcript_result.id;
    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

        let status_response = client
            .get(format!(
                "https://api.assemblyai.com/v2/transcript/{}",
                transcript_id
            ))
            .header("authorization", api_key)
            .send()
            .await?;

        let status_result: AssemblyAITranscriptResponse = status_response.json().await?;

        match status_result.status.as_str() {
            "completed" => {
                return Ok(status_result.text.unwrap_or_default());
            }
            "error" => {
                return Err(anyhow::anyhow!("AssemblyAI transcription failed"));
            }
            _ => continue,
        }
    }
}

pub async fn format_transcript(
    text: &str,
    provider: TranscriptionProvider,
    api_key: &str,
    model: &str,
    system_prompt: &str,
) -> Result<String> {
    match provider {
        TranscriptionProvider::OpenAI => format_with_openai(text, api_key, model, system_prompt).await,
        TranscriptionProvider::Groq => format_with_groq(text, api_key, model, system_prompt).await,
        _ => Err(anyhow::anyhow!("Formatting not supported for this provider")),
    }
}

async fn format_with_openai(
    text: &str,
    api_key: &str,
    model: &str,
    system_prompt: &str,
) -> Result<String> {
    let client = reqwest::Client::new();
    let response = client
        .post("https://api.openai.com/v1/chat/completions")
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&serde_json::json!({
            "model": model,
            "messages": [
                {
                    "role": "system",
                    "content": system_prompt
                },
                {
                    "role": "user",
                    "content": text
                }
            ],
            "temperature": 0.3
        }))
        .send()
        .await?;

    if !response.status().is_success() {
        let error = response.text().await?;
        return Err(anyhow::anyhow!("OpenAI formatting error: {}", error));
    }
    parse_chat_response(response).await
}

async fn format_with_groq(
    text: &str,
    api_key: &str,
    model: &str,
    system_prompt: &str,
) -> Result<String> {
    let client = reqwest::Client::new();
    let response = client
        .post("https://api.groq.com/openai/v1/chat/completions")
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&serde_json::json!({
            "model": model,
            "messages": [
                {
                    "role": "system",
                    "content": system_prompt
                },
                {
                    "role": "user",
                    "content": text
                }
            ],
            "temperature": 0.3
        }))
        .send()
        .await?;

    if !response.status().is_success() {
        let error = response.text().await?;
        return Err(anyhow::anyhow!("Groq formatting error: {}", error));
    }
    parse_chat_response(response).await
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}

#[derive(Deserialize)]
struct Choice {
    message: Message,
}

#[derive(Deserialize)]
struct Message {
    content: String,
}

async fn parse_chat_response(response: reqwest::Response) -> Result<String> {
    let result: ChatResponse = response.json().await?;
    Ok(result
        .choices
        .first()
        .map(|c| c.message.content.clone())
        .unwrap_or_else(|| String::from("")))
}

/// Validates an API key by making a lightweight test call to the provider
pub async fn validate_api_key(provider: TranscriptionProvider, api_key: &str) -> Result<bool> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()?;

    match provider {
        TranscriptionProvider::OpenAI => {
            // Call /v1/models - free, no tokens used
            let response = client
                .get("https://api.openai.com/v1/models")
                .header("Authorization", format!("Bearer {}", api_key))
                .send()
                .await?;
            Ok(response.status().is_success())
        }
        TranscriptionProvider::Groq => {
            // Call /v1/models - free
            let response = client
                .get("https://api.groq.com/openai/v1/models")
                .header("Authorization", format!("Bearer {}", api_key))
                .send()
                .await?;
            Ok(response.status().is_success())
        }
        TranscriptionProvider::AssemblyAI => {
            // Call account info endpoint - free
            let response = client
                .get("https://api.assemblyai.com/v2/transcript?limit=1")
                .header("authorization", api_key)
                .send()
                .await?;
            Ok(response.status().is_success())
        }
    }
}
